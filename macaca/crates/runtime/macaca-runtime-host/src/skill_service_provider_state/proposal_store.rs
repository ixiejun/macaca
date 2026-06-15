//! Draft proposal store and append-only event bridge for Skill governance state.
//!
//! Experience proposals are stored without mutating active catalog records.
//! `append_event` forwards to the `SkillGovernanceStoreStrategy` adapter so
//! durable journal persistence stays centralized in the governance store module.

use macaca_skill::{
    SkillExperienceDestinationRouteResult, SkillExperienceProposalCommand,
    SkillExperienceProposalRecord, SkillExperienceProposalResult,
    SkillExperienceProposalSnapshotResult, SkillGovernanceEventPayload, SkillGovernanceEventRecord,
    SkillGovernanceStoreStrategy,
};

use super::{event_id, SkillProviderGovernanceState};

impl SkillProviderGovernanceState {
    /// Store a draft-only experience proposal without changing active skills.
    pub(crate) async fn propose_experience(
        &self,
        command: SkillExperienceProposalCommand,
        destination_route: SkillExperienceDestinationRouteResult,
    ) -> SkillExperienceProposalResult {
        let captured_at = chrono::Utc::now();
        let proposal = SkillExperienceProposalRecord::from_candidate(
            &command.trace,
            command.candidate,
            captured_at,
        );
        self.proposals
            .lock()
            .await
            .insert(proposal.proposal_id.clone(), proposal.clone());
        self.append_event(SkillGovernanceEventRecord::new(
            event_id("skill-governance-proposal", captured_at),
            &command.trace,
            command.scope,
            captured_at,
            SkillGovernanceEventPayload::ProposalCreated(proposal.clone()),
        ))
        .await;

        SkillExperienceProposalResult {
            proposal,
            mutated: false,
            captured_at,
            destination_route,
        }
    }

    /// Return a deterministic, read-only snapshot of stored draft proposals.
    pub(crate) async fn experience_snapshot(&self) -> SkillExperienceProposalSnapshotResult {
        let captured_at = chrono::Utc::now();
        let mut proposals = self
            .proposals
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        proposals.sort_by(|left, right| left.proposal_id.cmp(&right.proposal_id));

        SkillExperienceProposalSnapshotResult {
            proposals,
            mutated: false,
            captured_at,
        }
    }

    /// Append one governance event through the shared store Strategy adapter.
    pub(crate) async fn append_event(&self, event: SkillGovernanceEventRecord) {
        let _ = <Self as SkillGovernanceStoreStrategy>::append_event(self, event).await;
    }
}
