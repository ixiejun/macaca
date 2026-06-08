//! Built-in local Strategy for Skill proposal materialization.
//!
//! The Strategy is deliberately narrow: it accepts only proposals that the
//! proposal-processing lane has already marked `ReadyForMaterialization`, builds
//! bounded AgentSkills-compatible markdown, delegates the file write to the
//! existing content-mutation Strategy, and promotes governance metadata only
//! after the write succeeds.  It never branches on application or workflow
//! names, and it never logs or returns the generated `SKILL.md` body.
//!
//! Module layout (Facade + Strategy + Builder + Specification vocabulary):
//! - `mod.rs` — traced command adapter and orchestration Strategy on governance state
//! - `draft_materialization_builder.rs` — Builder that turns sanitized proposals into bounded drafts
//! - `identity_vocabulary.rs` — deterministic slug and semantic token Specification helpers
//! - `content_digest_vocabulary.rs` — bounded digest and mutation metadata Value Object helpers

mod content_digest_vocabulary;
mod draft_materialization_builder;
mod identity_vocabulary;

use std::sync::Arc;

use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, TraceContext};
use macaca_skill::{
    SkillContentMutationCommand, SkillContentMutationKind, SkillContentMutationStatus,
    SkillEvolutionPromoteDraftCommand, SkillEvolutionProposalLifecycle,
    SkillExperienceProposalRecord, SkillProposalMaterializationCommand,
    SkillProposalMaterializationDenied, SkillProposalMaterializationResult,
    SkillProposalMaterializationStatus, SkillProposalProcessingState,
};
use serde_json::Value;

use crate::skill_service_codec::{decode, service_result, to_value};
use crate::skill_service_content_mutation::LocalSkillContentMutationStrategy;
use crate::skill_service_provider_state::SkillProviderGovernanceState;

pub(crate) use content_digest_vocabulary::materialization_mutation_metadata;
pub(crate) use draft_materialization_builder::{
    SkillDraftMaterialization, SkillDraftMaterializationBuilder,
};

/// Decode and apply a traced proposal materialization command.
///
/// This is the service-runtime entry point: it decodes the JSON payload into a
/// typed command, delegates to the governance-state Strategy, and wraps the
/// outcome in a traced `ServiceCallResult` for the Skill provider facade.
pub(crate) async fn apply_command(
    state: &Arc<SkillProviderGovernanceState>,
    content_mutation: &LocalSkillContentMutationStrategy,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillProposalMaterializationCommand = decode(payload)?;
    let result = state
        .materialize_ready_proposal(typed.clone(), content_mutation)
        .await?;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        proposal_id = %typed.proposal_id,
        status = ?result.status,
        skill_id = %result.skill_id,
        planned_bytes = result.planned_bytes,
        bytes_written = result.bytes_written,
        rollback_ref_present = result.rollback_memento_ref.is_some(),
        mutated = result.mutated,
        promoted = result.promoted,
        evidence_refs = result.evidence_ids.len(),
        policy_decision_refs = result.policy_decision_refs.len(),
        audit_event_ids = result.audit_event_ids.len(),
        "skill proposal materialization command completed"
    );
    Ok(service_result(to_value(result)?, trace))
}

impl SkillProviderGovernanceState {
    /// Materialize one ready proposal through the local Builder and mutation Strategy.
    ///
    /// The method follows a strict side-effect order.  It first performs all
    /// service-owned readiness checks, then builds bounded content, then either
    /// returns a preview or writes through `SkillContentMutationCommand`.  Only
    /// after the mutation Strategy reports `Applied` does it promote proposal
    /// lifecycle into active governance metadata.
    pub(crate) async fn materialize_ready_proposal(
        &self,
        command: SkillProposalMaterializationCommand,
        content_mutation: &LocalSkillContentMutationStrategy,
    ) -> ServiceResult<SkillProposalMaterializationResult> {
        if let Err(denial) = command.validate() {
            tracing::warn!(
                trace_id = %command.trace.trace_id,
                proposal_id = %command.proposal_id,
                reason = %denial.reason,
                "skill proposal materialization denied by command validation"
            );
            return Ok(SkillProposalMaterializationResult::denied(&command, denial));
        }

        let proposal = match self.ready_materialization_proposal(&command).await {
            Ok(proposal) => proposal,
            Err(denial) => {
                tracing::warn!(
                    trace_id = %command.trace.trace_id,
                    proposal_id = %command.proposal_id,
                    reason = %denial.reason,
                    "skill proposal materialization denied by readiness gate"
                );
                return Ok(SkillProposalMaterializationResult::denied(&command, denial));
            }
        };
        let draft = SkillDraftMaterializationBuilder::new(&proposal).build();

        tracing::info!(
            trace_id = %command.trace.trace_id,
            proposal_id = %proposal.proposal_id,
            skill_id = %draft.skill_id,
            skill_name = %draft.name,
            identity_source = %draft.identity_source,
            identity_used_fallback = draft.identity_used_fallback,
            dry_run = command.dry_run,
            planned_bytes = draft.content.len(),
            "skill proposal materialization built bounded semantic draft identity"
        );

        if command.dry_run {
            return Ok(SkillProposalMaterializationResult {
                status: SkillProposalMaterializationStatus::Previewed,
                proposal_id: proposal.proposal_id,
                skill_id: draft.skill_id,
                relative_path: draft.relative_path,
                content_digest: Some(draft.content_digest),
                planned_bytes: draft.content.len() as u64,
                bytes_written: 0,
                rollback_memento_ref: None,
                denied_reason: None,
                mutated: false,
                promoted: false,
                evidence_ids: command.sanitized_evidence_ids(),
                policy_decision_refs: command.sanitized_policy_decision_refs(),
                audit_event_ids: command.sanitized_audit_event_ids(),
                trace_id: command.trace.trace_id,
                captured_at: chrono::Utc::now(),
            });
        }

        let mutation = content_mutation
            .apply(SkillContentMutationCommand {
                trace: command.trace.clone(),
                scope: command.scope.clone(),
                skill_id: draft.skill_id.clone(),
                package_root: command.package_root.clone(),
                relative_path: draft.relative_path.clone(),
                kind: SkillContentMutationKind::CreateSkill,
                ownership: command.ownership.clone(),
                content: Some(draft.content.clone()),
                reason: command.reason.clone(),
                evidence_ids: command.sanitized_evidence_ids(),
                policy_decision_refs: command.sanitized_policy_decision_refs(),
                allow_executable_script_mutation: false,
                policy: command.policy.clone(),
                metadata: materialization_mutation_metadata(&proposal, &draft),
            })
            .await?;

        if mutation.status != SkillContentMutationStatus::Applied {
            tracing::warn!(
                trace_id = %command.trace.trace_id,
                proposal_id = %proposal.proposal_id,
                skill_id = %draft.skill_id,
                mutation_status = ?mutation.status,
                denied_reason = mutation.denied_reason.as_deref().unwrap_or(""),
                "skill proposal materialization stopped after mutation denial"
            );
            return Ok(SkillProposalMaterializationResult {
                status: SkillProposalMaterializationStatus::Denied,
                proposal_id: proposal.proposal_id,
                skill_id: draft.skill_id,
                relative_path: draft.relative_path,
                content_digest: Some(draft.content_digest),
                planned_bytes: draft.content.len() as u64,
                bytes_written: 0,
                rollback_memento_ref: mutation.rollback_memento_ref,
                denied_reason: mutation.denied_reason,
                mutated: false,
                promoted: false,
                evidence_ids: mutation.evidence_ids,
                policy_decision_refs: mutation.policy_decision_refs,
                audit_event_ids: command.sanitized_audit_event_ids(),
                trace_id: command.trace.trace_id,
                captured_at: chrono::Utc::now(),
            });
        }

        self.prepare_proposal_materialization_target(&command, &draft)
            .await;
        let promotion = self
            .promote_draft(SkillEvolutionPromoteDraftCommand {
                trace: command.trace.clone(),
                scope: command.scope.clone(),
                proposal_id: command.proposal_id.clone(),
                reason: command.reason.clone(),
                evidence_ids: command.sanitized_evidence_ids(),
                policy_decision_refs: command.sanitized_policy_decision_refs(),
                policy: command.policy.clone(),
            })
            .await
            .map_err(ServiceError::InvalidArgument)?;

        tracing::info!(
            trace_id = %command.trace.trace_id,
            proposal_id = %promotion.proposal.proposal_id,
            skill_id = %promotion.record.provenance.skill_id,
            bytes_written = mutation.bytes_written,
            rollback_ref_present = mutation.rollback_memento_ref.is_some(),
            "skill proposal materialization promoted governance after write"
        );

        Ok(SkillProposalMaterializationResult {
            status: SkillProposalMaterializationStatus::Applied,
            proposal_id: promotion.proposal.proposal_id,
            skill_id: draft.skill_id,
            relative_path: draft.relative_path,
            content_digest: Some(draft.content_digest),
            planned_bytes: draft.content.len() as u64,
            bytes_written: mutation.bytes_written,
            rollback_memento_ref: mutation.rollback_memento_ref,
            denied_reason: None,
            mutated: true,
            promoted: true,
            evidence_ids: promotion.evidence_ids,
            policy_decision_refs: promotion.policy_decision_refs,
            audit_event_ids: command.sanitized_audit_event_ids(),
            trace_id: command.trace.trace_id,
            captured_at: chrono::Utc::now(),
        })
    }

    /// Return the proposal only if processing has explicitly marked it ready.
    ///
    /// Readiness is a conjunction of three service-owned gates: the proposal
    /// record must exist, remain in draft lifecycle, and carry a processing lane
    /// state of `ReadyForMaterialization`.  Any failure returns a structured
    /// denial rather than an internal error so callers can audit the reason.
    async fn ready_materialization_proposal(
        &self,
        command: &SkillProposalMaterializationCommand,
    ) -> Result<SkillExperienceProposalRecord, SkillProposalMaterializationDenied> {
        let proposals = self.proposals.lock().await;
        let proposal = proposals
            .get(command.proposal_id.trim())
            .cloned()
            .ok_or_else(|| {
                SkillProposalMaterializationDenied::new(
                    "skill proposal materialization requires an existing proposal",
                    false,
                )
            })?;
        drop(proposals);

        if proposal.lifecycle != SkillEvolutionProposalLifecycle::Draft {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires draft proposal lifecycle",
                false,
            ));
        }

        let processing = self.proposal_processing.lock().await;
        let Some(record) = processing.get(command.proposal_id.trim()) else {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires ready processing record",
                false,
            ));
        };
        if record.state != SkillProposalProcessingState::ReadyForMaterialization {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal processing state is not ready for materialization",
                false,
            ));
        }
        Ok(proposal)
    }

    /// Store target identity refs on the proposal before promotion reads it.
    ///
    /// Promotion reads proposal metadata to attach provenance.  Writing digest
    /// and materialization refs here keeps the Builder pure while still giving
    /// downstream governance a stable audit trail.
    async fn prepare_proposal_materialization_target(
        &self,
        command: &SkillProposalMaterializationCommand,
        draft: &SkillDraftMaterialization,
    ) {
        let mut proposals = self.proposals.lock().await;
        if let Some(proposal) = proposals.get_mut(command.proposal_id.trim()) {
            proposal.target_skill_id = Some(draft.skill_id.clone());
            proposal.target_skill_name = Some(draft.name.clone());
            proposal.metadata.insert(
                "materialization_ref".into(),
                draft.materialization_ref.clone(),
            );
            proposal
                .metadata
                .insert("content_digest_ref".into(), draft.content_digest.clone());
        }
    }
}
