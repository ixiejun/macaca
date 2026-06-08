//! Skill evolution proposal command handlers.
//!
//! Experience and patch proposals are validated at the adapter boundary, routed
//! through `SkillExperienceDestinationRouter`, and persisted by governance state.

use macaca_proto::{ServiceCallResult, ServiceCommand, ServiceError, ServiceResult};
use macaca_skill::{
    SkillEvolutionPromoteDraftCommand, SkillEvolutionProposePatchCommand,
    SkillEvolutionRejectDraftCommand, SkillExperienceProposalCommand,
    SkillExperienceProposalSnapshotCommand,
};

use crate::skill_service_codec::{decode, service_result, to_value};

use super::SkillSystemServiceProvider;

impl SkillSystemServiceProvider {
    /// Create a draft experience proposal from a verified task outcome candidate.
    pub(crate) async fn handle_evolution_propose_from_task(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillExperienceProposalCommand = decode(command.payload)?;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            task_id = %typed.candidate.task_id,
            verified_terminal_success = typed.candidate.verified_terminal_success,
            evidence_gate = ?typed.candidate.evidence_gate,
            destination = ?typed.candidate.destination,
            evidence_refs = typed.candidate.evidence_ids.len(),
            "skill experience candidate received"
        );
        if let Err(err) = typed.candidate.validate() {
            tracing::warn!(
                trace_id = %typed.trace.trace_id,
                task_id = %typed.candidate.task_id,
                evidence_gate = ?typed.candidate.evidence_gate,
                reason = %err,
                "skill experience candidate rejected before proposal creation"
            );
            return Err(ServiceError::InvalidArgument(err));
        }
        let destination_route = self
            .experience_router
            .route(&typed.scope, &typed.candidate, &typed.trace)
            .await;
        let result = self
            .governance_state
            .propose_experience(typed.clone(), destination_route)
            .await;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            proposal_id = %result.proposal.proposal_id,
            task_id = %result.proposal.task_id,
            destination = ?result.proposal.destination,
            destination_route_status = result.destination_route.status.as_str(),
            action = ?result.proposal.recommended_action,
            mutated = result.mutated,
            "skill experience proposal created"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Create a patch proposal against an existing skill identity.
    pub(crate) async fn handle_evolution_propose_patch(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillEvolutionProposePatchCommand = decode(command.payload)?;
        typed.validate().map_err(ServiceError::InvalidArgument)?;
        let result = self.governance_state.propose_patch(typed.clone()).await;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            proposal_id = %result.proposal.proposal_id,
            target_skill_id = ?result.proposal.target_skill_id,
            evidence_refs = result.proposal.evidence_ids.len(),
            mutated = result.mutated,
            "skill patch proposal created"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Promote an approved draft proposal into active governance records.
    pub(crate) async fn handle_evolution_promote_draft(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillEvolutionPromoteDraftCommand = decode(command.payload)?;
        typed.validate().map_err(ServiceError::InvalidArgument)?;
        let result = self
            .governance_state
            .promote_draft(typed.clone())
            .await
            .map_err(ServiceError::InvalidArgument)?;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            proposal_id = %typed.proposal_id,
            skill_id = %result.record.provenance.skill_id,
            policy_decision_refs = result.policy_decision_refs.len(),
            evidence_refs = result.evidence_ids.len(),
            mutated = result.mutated,
            "skill draft proposal promoted"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Reject a draft proposal while preserving audit references.
    pub(crate) async fn handle_evolution_reject_draft(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillEvolutionRejectDraftCommand = decode(command.payload)?;
        typed.validate().map_err(ServiceError::InvalidArgument)?;
        let result = self
            .governance_state
            .reject_draft(typed.clone())
            .await
            .map_err(ServiceError::InvalidArgument)?;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            proposal_id = %typed.proposal_id,
            policy_decision_refs = result.policy_decision_refs.len(),
            evidence_refs = result.evidence_ids.len(),
            mutated = result.mutated,
            "skill draft proposal rejected"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Emit a snapshot of in-flight experience proposals.
    pub(crate) async fn handle_evolution_snapshot(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillExperienceProposalSnapshotCommand = decode(command.payload)?;
        let result = self.governance_state.experience_snapshot().await;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            proposals = result.proposals.len(),
            mutated = result.mutated,
            include_discarded = typed.include_discarded,
            "skill experience proposal snapshot emitted"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }
}
