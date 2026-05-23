//! Runtime-host merge apply adapter for Skill curation.
//!
//! Umbrella merge apply is a privileged curation mutation.  This module keeps
//! the runtime-host provider as a Strategy that orchestrates existing Skill
//! service commands: lifecycle supersede, alias upsert, rollback memento, and
//! bounded report refs.  It deliberately avoids file writes and shell-owned
//! semantics; support-file writes remain delegated to the safe mutation command
//! surface referenced by the merge apply envelope.

use std::collections::BTreeSet;
use std::sync::Arc;

use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, TraceContext};
use macaca_skill::{
    SkillAliasRecord, SkillCurationLifecycleCommand, SkillCurationSupersedeCommand,
    SkillGovernanceEventPayload, SkillGovernanceEventRecord, SkillGovernanceReadModel,
    SkillGovernanceStoreStrategy, SkillMergeApplyResult, SkillRollbackRefRecord,
    SkillUmbrellaMergeApplyCommand,
};
use serde_json::Value;

use crate::skill_service_codec::{decode, service_result, to_value};
use crate::skill_service_provider_state::{event_id, SkillProviderGovernanceState};

/// Decode and apply an approved umbrella merge command through service state.
pub(crate) async fn merge_apply_command(
    state: &Arc<SkillProviderGovernanceState>,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillUmbrellaMergeApplyCommand = decode(payload)?;
    typed.validate().map_err(ServiceError::InvalidArgument)?;
    let result = state
        .apply_umbrella_merge(typed.clone())
        .await
        .map_err(ServiceError::InvalidArgument)?;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        proposal_id = %result.proposal_id,
        target_skill_id = %result.target_umbrella_skill_id,
        superseded = result.superseded_skill_ids.len(),
        aliases = result.alias_records.len(),
        report_ref = result.report_ref.as_deref().unwrap_or(""),
        rollback_ref = result.rollback_ref.as_deref().unwrap_or(""),
        mutated = result.mutated,
        "skill umbrella merge apply completed"
    );
    Ok(service_result(to_value(result)?, trace))
}

impl SkillProviderGovernanceState {
    /// Apply an approved umbrella merge by superseding every absorbed source.
    ///
    /// The method uses Command and State patterns together: the merge envelope
    /// validates approval and policy refs, while each source transition still
    /// passes through the existing supersede state machine.  A Memento is
    /// captured before any alias or lifecycle mutation so rollback can restore
    /// governance records and alias maps if later support-file work fails.
    pub(crate) async fn apply_umbrella_merge(
        &self,
        command: SkillUmbrellaMergeApplyCommand,
    ) -> Result<SkillMergeApplyResult, String> {
        ensure_aliases_cover_sources(&command)?;
        let captured_at = chrono::Utc::now();
        let rollback_ref = self
            .capture_merge_rollback_memento(&command, captured_at)
            .await;
        let report_ref = Some(format!(
            "store://skill-curation/{}/merge/{}/REPORT.md",
            command.trace.trace_id,
            captured_at.timestamp_nanos_opt().unwrap_or_default()
        ));
        let mut superseded_skill_ids = Vec::new();
        let mut alias_records = Vec::new();

        for alias in command.proposal.alias_effects.clone() {
            let lifecycle = self.lifecycle_command_for_alias(&command, &alias).await?;
            let result = self
                .supersede(SkillCurationSupersedeCommand {
                    lifecycle,
                    alias: alias_to_record(alias, captured_at),
                })
                .await?;
            superseded_skill_ids.push(result.skill_id);
            let aliases = self.aliases.lock().await;
            if let Some(record) = aliases.get(superseded_skill_ids.last().unwrap()).cloned() {
                alias_records.push(record);
            }
        }

        self.record_merge_rollback_ref(&command, &rollback_ref, report_ref.clone(), captured_at)
            .await;
        tracing::info!(
            trace_id = %command.trace.trace_id,
            proposal_id = %command.proposal.proposal_id,
            target_skill_id = %command.proposal.target_umbrella_skill_id,
            superseded = superseded_skill_ids.len(),
            aliases = alias_records.len(),
            rollback_ref = %rollback_ref,
            "skill umbrella merge applied through lifecycle and alias commands"
        );

        Ok(SkillMergeApplyResult {
            proposal_id: command.proposal.proposal_id,
            target_umbrella_skill_id: command.proposal.target_umbrella_skill_id,
            superseded_skill_ids,
            alias_records,
            support_file_movement_refs: command
                .proposal
                .support_file_movements
                .iter()
                .map(|movement| {
                    format!(
                        "skill.content.mutate://{}/{}",
                        movement.source_skill_id, movement.destination_path
                    )
                })
                .collect(),
            bounded_report_summary: bounded_report_summary(&command.proposal.rationale),
            evidence_ids: non_empty(&command.proposal.evidence_ids),
            report_ref,
            rollback_ref: Some(rollback_ref),
            policy_decision_refs: non_empty(&command.policy_decision_refs),
            audit_event_ids: non_empty(&command.audit_event_ids),
            mutated: true,
            captured_at,
        })
    }

    /// Build the narrower lifecycle command from the current source record.
    async fn lifecycle_command_for_alias(
        &self,
        command: &SkillUmbrellaMergeApplyCommand,
        alias: &macaca_skill::SkillMergeAliasEffect,
    ) -> Result<SkillCurationLifecycleCommand, String> {
        let records = self.records.lock().await;
        let record = records
            .get(alias.source_skill_id.trim())
            .cloned()
            .ok_or_else(|| {
                format!(
                    "umbrella merge source skill is not governed: {}",
                    alias.source_skill_id
                )
            })?;
        drop(records);
        Ok(SkillCurationLifecycleCommand {
            trace: command.trace.clone(),
            scope: command.scope.clone(),
            skill_id: record.provenance.skill_id,
            name: record.provenance.name,
            source: record.provenance.source,
            source_scope: record.provenance.source_scope,
            author_kind: record.provenance.author_kind,
            reason: command.proposal.rationale.clone(),
            evidence_ids: command.proposal.evidence_ids.clone(),
            task_id: None,
            policy_decision_refs: command.policy_decision_refs.clone(),
            policy: Default::default(),
        })
    }

    /// Capture rollback state before merge side effects.
    async fn capture_merge_rollback_memento(
        &self,
        command: &SkillUmbrellaMergeApplyCommand,
        captured_at: chrono::DateTime<chrono::Utc>,
    ) -> String {
        let rollback_ref = format!(
            "store://skill-curation/{}/merge-rollback-{}",
            command.trace.trace_id,
            captured_at.timestamp_nanos_opt().unwrap_or_default()
        );
        self.curation_mementos.lock().await.insert(
            rollback_ref.clone(),
            crate::skill_service_provider_state::SkillCurationRollbackMemento {
                rollback_ref: rollback_ref.clone(),
                before_snapshot_ref: format!(
                    "store://skill-curation/{}/merge-before-{}",
                    command.trace.trace_id,
                    captured_at.timestamp_nanos_opt().unwrap_or_default()
                ),
                after_snapshot_ref: Some(format!(
                    "store://skill-curation/{}/merge-after-{}",
                    command.trace.trace_id,
                    captured_at.timestamp_nanos_opt().unwrap_or_default()
                )),
                records: self.records.lock().await.clone(),
                aliases: self.aliases.lock().await.clone(),
                proposals: self.proposals.lock().await.clone(),
                event_log: self.event_log.lock().await.clone(),
                report_refs: vec![format!(
                    "store://skill-curation/{}/merge/{}/REPORT.md",
                    command.trace.trace_id,
                    captured_at.timestamp_nanos_opt().unwrap_or_default()
                )],
                package_memento_refs: vec![rollback_ref.clone()],
            },
        );
        rollback_ref
    }

    /// Record a rollback ref event so Store/EventLog replay can audit merge apply.
    async fn record_merge_rollback_ref(
        &self,
        command: &SkillUmbrellaMergeApplyCommand,
        rollback_ref: &str,
        report_ref: Option<String>,
        captured_at: chrono::DateTime<chrono::Utc>,
    ) {
        let restored_read_model =
            SkillGovernanceReadModel::from_events(self.event_log.lock().await.clone());
        let mut event = SkillGovernanceEventRecord::new(
            event_id("skill-governance-merge-rollback-ref", captured_at),
            &command.trace,
            command.scope.clone(),
            captured_at,
            SkillGovernanceEventPayload::RollbackRefRecorded(SkillRollbackRefRecord {
                rollback_ref: rollback_ref.to_string(),
                run_id: restored_read_model
                    .curation_runs
                    .last()
                    .map(|run| run.run_id.clone())
                    .unwrap_or_else(|| command.proposal.proposal_id.clone()),
                trace_id: command.trace.trace_id.clone(),
                before_snapshot_ref: format!(
                    "store://skill-curation/{}/merge-before-{}",
                    command.trace.trace_id,
                    captured_at.timestamp_nanos_opt().unwrap_or_default()
                ),
                after_snapshot_ref: Some(format!(
                    "store://skill-curation/{}/merge-after-{}",
                    command.trace.trace_id,
                    captured_at.timestamp_nanos_opt().unwrap_or_default()
                )),
                report_ref,
                captured_at,
            }),
        );
        event.policy_decision_ids = non_empty(&command.policy_decision_refs);
        event.audit_event_ids = non_empty(&command.audit_event_ids);
        let _ = <Self as SkillGovernanceStoreStrategy>::append_event(self, event).await;
    }
}

fn ensure_aliases_cover_sources(command: &SkillUmbrellaMergeApplyCommand) -> Result<(), String> {
    let source_ids = command
        .proposal
        .source_skill_ids
        .iter()
        .map(|id| id.trim().to_string())
        .collect::<BTreeSet<_>>();
    let alias_source_ids = command
        .proposal
        .alias_effects
        .iter()
        .map(|alias| alias.source_skill_id.trim().to_string())
        .collect::<BTreeSet<_>>();
    if source_ids.is_empty() || source_ids != alias_source_ids {
        return Err("umbrella merge apply requires one alias effect per absorbed source".into());
    }
    Ok(())
}

fn alias_to_record(
    alias: macaca_skill::SkillMergeAliasEffect,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> SkillAliasRecord {
    SkillAliasRecord {
        source_skill_id: alias.source_skill_id,
        source_name: alias.source_name,
        target_skill_id: alias.target_skill_id,
        target_name: alias.target_name,
        kind: alias.kind,
        rationale: alias.rationale,
        created_at: captured_at,
        updated_at: captured_at,
        evidence_ids: alias.evidence_ids,
    }
}

fn non_empty(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect()
}

fn bounded_report_summary(rationale: &str) -> String {
    const MAX_REPORT_SUMMARY_BYTES: usize = 512;
    let mut summary = rationale.trim().to_string();
    if summary.len() > MAX_REPORT_SUMMARY_BYTES {
        summary.truncate(MAX_REPORT_SUMMARY_BYTES);
    }
    summary
}
