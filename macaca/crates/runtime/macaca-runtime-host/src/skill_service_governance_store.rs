//! Local governance store Strategy for the built-in Skill provider.
//!
//! The state owner keeps in-memory records for development and tests, while
//! this module implements the append/replay Strategy that a future Store or
//! EventLog provider can replace without changing SDK or shell contracts.

use async_trait::async_trait;
use macaca_skill::{
    SkillGovernanceEventPayload, SkillGovernanceEventRecord, SkillGovernanceReadModel,
    SkillGovernanceStoreStrategy, SkillGovernanceStoreUnavailable,
};

use crate::skill_service_provider_state::SkillProviderGovernanceState;

#[async_trait]
impl SkillGovernanceStoreStrategy for SkillProviderGovernanceState {
    async fn append_event(
        &self,
        mut event: SkillGovernanceEventRecord,
    ) -> Result<SkillGovernanceEventRecord, SkillGovernanceStoreUnavailable> {
        if event.provenance.policy_decision_refs.is_empty() {
            event.provenance.policy_decision_refs = event.policy_decision_ids.clone();
        }
        if event.provenance.audit_event_ids.is_empty() {
            event.provenance.audit_event_ids = event.audit_event_ids.clone();
        }
        tracing::info!(
            event_id = %event.event_id,
            trace_id = %event.trace_id,
            event_kind = %governance_event_kind(&event.payload),
            provenance_action = ?event.provenance.action,
            application_id = ?event.scope.application_id,
            session_id = ?event.scope.session_id,
            agent_name = ?event.scope.agent_name,
            "skill governance event appended through local compatibility adapter"
        );
        self.event_log.lock().await.push(event.clone());
        if let Err(error) = self.persist_governance_event_journal(&event).await {
            tracing::warn!(
                event_id = %event.event_id,
                trace_id = %event.trace_id,
                error = %error,
                "skill governance event durable journal append failed"
            );
        }
        Ok(event)
    }

    async fn read_model(
        &self,
    ) -> Result<SkillGovernanceReadModel, SkillGovernanceStoreUnavailable> {
        let events = self.event_log.lock().await.clone();
        let read_model = SkillGovernanceReadModel::from_events(events);
        tracing::info!(
            events = read_model.replayed_events,
            records = read_model.records.len(),
            provenance_events = read_model.provenance_events.len(),
            aliases = read_model.aliases.len(),
            proposals = read_model.proposals.len(),
            curation_runs = read_model.curation_runs.len(),
            rollback_refs = read_model.rollback_refs.len(),
            "skill governance read model replayed through local compatibility adapter"
        );
        Ok(read_model)
    }
}

fn governance_event_kind(payload: &SkillGovernanceEventPayload) -> &'static str {
    match payload {
        SkillGovernanceEventPayload::UsageRecorded(_) => "usage_recorded",
        SkillGovernanceEventPayload::LifecycleApplied(_) => "lifecycle_applied",
        SkillGovernanceEventPayload::AliasUpserted(_) => "alias_upserted",
        SkillGovernanceEventPayload::ProposalCreated(_) => "proposal_created",
        SkillGovernanceEventPayload::CurationRunRecorded(_) => "curation_run_recorded",
        SkillGovernanceEventPayload::SnapshotRefRecorded(_) => "snapshot_ref_recorded",
        SkillGovernanceEventPayload::RollbackRefRecorded(_) => "rollback_ref_recorded",
    }
}
