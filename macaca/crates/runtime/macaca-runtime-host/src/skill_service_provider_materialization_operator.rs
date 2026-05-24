//! Service-owned Director for autonomous Skill proposal materialization.
//!
//! The operator composes existing Skill service Strategies rather than
//! duplicating their rules: proposal processing scores and deduplicates backlog
//! records, the package-target resolver chooses a provider-neutral package
//! directory, and the materialization Strategy owns `SKILL.md` construction plus
//! governed content mutation. This module only coordinates those boundaries and
//! records body-free audit-friendly refs.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, TraceContext};
use macaca_skill::{
    operator_event_ref, SkillAutonomousMaterializationDenied,
    SkillAutonomousMaterializationRunCommand, SkillAutonomousMaterializationRunResult,
    SkillAutonomousMaterializationStatus, SkillEvolutionProposalLifecycle,
    SkillProposalMaterializationCommand, SkillProposalMaterializationResult,
    SkillProposalMaterializationStatus, SkillProposalProcessingRecord,
    SkillProposalProcessingRunCommand, SkillProposalProcessingState,
};
use serde_json::Value;

use crate::skill_service_codec::{decode, service_result, to_value};
use crate::skill_service_content_mutation::LocalSkillContentMutationStrategy;
use crate::skill_service_provider_state::SkillProviderGovernanceState;

/// Decode and run one traced autonomous materialization operator command.
pub(crate) async fn run_command(
    state: &Arc<SkillProviderGovernanceState>,
    content_mutation: &LocalSkillContentMutationStrategy,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillAutonomousMaterializationRunCommand = decode(payload)?;
    let result = state
        .run_materialization_operator(typed.clone(), content_mutation)
        .await?;
    state
        .record_materialization_operator_run(result.clone())
        .await;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        run_id = %result.run_id,
        dry_run = typed.dry_run,
        selected = result.selected_proposal_ids.len(),
        materializations = result.materializations.len(),
        denied = result.denied.len(),
        status = ?result.status,
        mutated = result.mutated,
        "skill autonomous materialization operator command completed"
    );
    Ok(service_result(to_value(result)?, trace))
}

/// Decode and return recent autonomous materialization operator evidence.
pub(crate) async fn snapshot_command(
    state: &Arc<SkillProviderGovernanceState>,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: macaca_skill::SkillAutonomousMaterializationSnapshotCommand = decode(payload)?;
    let result = state.materialization_operator_snapshot(typed).await;
    Ok(service_result(to_value(result)?, trace))
}

impl SkillProviderGovernanceState {
    /// Run one bounded autonomous materialization cycle.
    ///
    /// Apply mode persists processing records before materialization so the
    /// existing single-proposal materializer can enforce its ready-state gate.
    /// Dry-run mode keeps processing read-only, temporarily stages only the
    /// selected ready record for the preview call, and restores the previous
    /// processing memento immediately after each preview.
    pub(crate) async fn run_materialization_operator(
        &self,
        command: SkillAutonomousMaterializationRunCommand,
        content_mutation: &LocalSkillContentMutationStrategy,
    ) -> ServiceResult<SkillAutonomousMaterializationRunResult> {
        if let Err(denial) = command.validate() {
            tracing::warn!(
                trace_id = %command.trace.trace_id,
                reason = %denial.reason,
                "skill autonomous materialization operator denied by command validation"
            );
            return Ok(SkillAutonomousMaterializationRunResult::denied(
                &command, denial,
            ));
        }

        tracing::info!(
            trace_id = %command.trace.trace_id,
            dry_run = command.dry_run,
            batch_limit = command.batch_limit,
            min_ready_score = command.min_ready_score,
            "skill autonomous materialization operator started"
        );

        let processing = self.process_proposals(processing_command(&command)).await;
        let selected_records = processing
            .records
            .iter()
            .filter(|record| {
                record.state == SkillProposalProcessingState::ReadyForMaterialization
                    && record.lifecycle == SkillEvolutionProposalLifecycle::Draft
            })
            .take(command.batch_limit as usize)
            .cloned()
            .collect::<Vec<_>>();
        let selected_proposal_ids = selected_records
            .iter()
            .map(|record| record.proposal_id.clone())
            .collect::<Vec<_>>();

        tracing::info!(
            trace_id = %command.trace.trace_id,
            processing_run_id = %processing.run_id,
            ready = processing.state_counts.ready_for_materialization,
            suppressed = processing.state_counts.suppressed_duplicate,
            rejected = processing.state_counts.rejected,
            selected = selected_records.len(),
            "skill autonomous materialization operator selected ready proposals"
        );

        let resolver = ProposalScopedPackageTargetResolver;
        let mut materializations = Vec::new();
        let mut denied = Vec::new();

        for record in selected_records {
            let target_name = self.proposal_target_name(&record.proposal_id).await;
            let package_root = match resolver.resolve(
                &command.package_collection_root,
                &record.proposal_id,
                target_name.as_deref(),
            ) {
                Ok(root) => root,
                Err(denial) => {
                    tracing::warn!(
                        trace_id = %command.trace.trace_id,
                        proposal_id = %record.proposal_id,
                        reason = %denial.reason,
                        "skill autonomous materialization operator target resolution denied"
                    );
                    denied.push(denial);
                    continue;
                }
            };

            if !command.dry_run {
                tokio::fs::create_dir_all(&package_root)
                    .await
                    .map_err(|err| {
                        ServiceError::InvalidArgument(format!(
                            "skill materialization package root provisioning failed: {err}"
                        ))
                    })?;
            }

            let materialization_command =
                single_materialization_command(&command, &record, package_root);
            let result = if command.dry_run {
                self.preview_with_temporary_ready_record(
                    record.clone(),
                    materialization_command,
                    content_mutation,
                )
                .await?
            } else {
                self.materialize_ready_proposal(materialization_command, content_mutation)
                    .await?
            };

            if result.status == SkillProposalMaterializationStatus::Denied {
                denied.push(SkillAutonomousMaterializationDenied::new(
                    Some(record.proposal_id.clone()),
                    result
                        .denied_reason
                        .clone()
                        .unwrap_or_else(|| "skill proposal materialization denied".into()),
                    false,
                ));
            }

            tracing::info!(
                trace_id = %command.trace.trace_id,
                proposal_id = %record.proposal_id,
                materialization_status = ?result.status,
                skill_id = %result.skill_id,
                rollback_ref_present = result.rollback_memento_ref.is_some(),
                mutated = result.mutated,
                promoted = result.promoted,
                "skill autonomous materialization operator recorded proposal result"
            );
            materializations.push(result);
        }

        let captured_at = chrono::Utc::now();
        let status = operator_status(&materializations, &denied);
        let mutated = processing.mutated
            || materializations
                .iter()
                .any(|materialization| materialization.mutated || materialization.promoted);
        let result = SkillAutonomousMaterializationRunResult {
            status,
            run_id: operator_event_ref("skill-materialization-operator-run", captured_at),
            trace_id: command.trace.trace_id.clone(),
            dry_run: command.dry_run,
            batch_limit: command.batch_limit,
            selected_proposal_ids,
            processing,
            materializations,
            denied,
            mutated,
            captured_at,
        };

        tracing::info!(
            trace_id = %command.trace.trace_id,
            run_id = %result.run_id,
            status = ?result.status,
            selected = result.selected_proposal_ids.len(),
            materializations = result.materializations.len(),
            denied = result.denied.len(),
            mutated = result.mutated,
            "skill autonomous materialization operator completed aggregate run"
        );
        Ok(result)
    }

    async fn proposal_target_name(&self, proposal_id: &str) -> Option<String> {
        self.proposals
            .lock()
            .await
            .get(proposal_id)
            .and_then(|proposal| proposal.target_skill_name.clone())
    }

    async fn preview_with_temporary_ready_record(
        &self,
        record: SkillProposalProcessingRecord,
        command: SkillProposalMaterializationCommand,
        content_mutation: &LocalSkillContentMutationStrategy,
    ) -> ServiceResult<SkillProposalMaterializationResult> {
        let proposal_id = record.proposal_id.clone();
        let previous = {
            let mut processing = self.proposal_processing.lock().await;
            processing.insert(proposal_id.clone(), record)
        };
        let result = self
            .materialize_ready_proposal(command, content_mutation)
            .await;
        {
            let mut processing = self.proposal_processing.lock().await;
            if let Some(previous) = previous {
                processing.insert(proposal_id, previous);
            } else {
                processing.remove(&proposal_id);
            }
        }
        result
    }
}

/// Strategy interface for resolving proposal-scoped package targets.
trait SkillPackageTargetResolver {
    fn resolve(
        &self,
        collection_root: &Path,
        proposal_id: &str,
        target_skill_name: Option<&str>,
    ) -> Result<PathBuf, SkillAutonomousMaterializationDenied>;
}

/// Default resolver that derives one child package directory per proposal.
struct ProposalScopedPackageTargetResolver;

impl SkillPackageTargetResolver for ProposalScopedPackageTargetResolver {
    fn resolve(
        &self,
        collection_root: &Path,
        proposal_id: &str,
        target_skill_name: Option<&str>,
    ) -> Result<PathBuf, SkillAutonomousMaterializationDenied> {
        if collection_root.as_os_str().is_empty() {
            return Err(SkillAutonomousMaterializationDenied::new(
                Some(proposal_id.into()),
                "skill materialization target resolver requires collection root",
                false,
            ));
        }
        let slug = target_skill_name
            .map(slugify)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| slugify(proposal_id));
        if slug.is_empty() {
            return Err(SkillAutonomousMaterializationDenied::new(
                Some(proposal_id.into()),
                "skill materialization target resolver could not derive package slug",
                false,
            ));
        }
        Ok(collection_root.join(slug))
    }
}

fn processing_command(
    command: &SkillAutonomousMaterializationRunCommand,
) -> SkillProposalProcessingRunCommand {
    SkillProposalProcessingRunCommand {
        trace: command.trace.clone(),
        scope: command.scope.clone(),
        dry_run: command.dry_run,
        min_ready_score: command.min_ready_score,
        evidence_ids: command.sanitized_evidence_ids(),
        policy_decision_refs: command.sanitized_policy_decision_refs(),
        audit_event_ids: command.sanitized_audit_event_ids(),
        policy: command.policy.clone(),
    }
}

fn single_materialization_command(
    command: &SkillAutonomousMaterializationRunCommand,
    record: &SkillProposalProcessingRecord,
    package_root: PathBuf,
) -> SkillProposalMaterializationCommand {
    SkillProposalMaterializationCommand {
        trace: command.trace.clone(),
        scope: command.scope.clone(),
        proposal_id: record.proposal_id.clone(),
        package_root,
        dry_run: command.dry_run,
        ownership: command.ownership.clone(),
        reason: command.reason.clone(),
        evidence_ids: command.sanitized_evidence_ids(),
        policy_decision_refs: command.sanitized_policy_decision_refs(),
        audit_event_ids: command.sanitized_audit_event_ids(),
        policy: command.policy.clone(),
    }
}

fn operator_status(
    materializations: &[SkillProposalMaterializationResult],
    denied: &[SkillAutonomousMaterializationDenied],
) -> SkillAutonomousMaterializationStatus {
    if materializations
        .iter()
        .any(|result| result.status == SkillProposalMaterializationStatus::Applied)
    {
        return SkillAutonomousMaterializationStatus::Applied;
    }
    if materializations
        .iter()
        .any(|result| result.status == SkillProposalMaterializationStatus::Previewed)
    {
        return SkillAutonomousMaterializationStatus::Previewed;
    }
    if !denied.is_empty() {
        return SkillAutonomousMaterializationStatus::Denied;
    }
    SkillAutonomousMaterializationStatus::Noop
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
        if slug.len() >= 80 {
            break;
        }
    }
    slug.trim_matches('-').to_string()
}
