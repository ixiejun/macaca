//! Proposal lifecycle state transitions for the built-in Skill provider.
//!
//! Keeping these helpers outside `skill_service_provider_state` prevents the
//! local compatibility adapter from becoming a large mixed curation engine.
//! The module still operates only on provider-neutral governance DTOs and
//! append-only events; package file mutation is deliberately left to a later
//! safe-mutation service slice.

use std::collections::BTreeMap;

use macaca_skill::{
    SkillEvolutionPromoteDraftCommand, SkillEvolutionPromoteDraftResult,
    SkillEvolutionProposalLifecycle, SkillEvolutionProposePatchCommand,
    SkillEvolutionProposePatchResult, SkillEvolutionRejectDraftCommand,
    SkillEvolutionRejectDraftResult, SkillExperienceProposalRecord, SkillGovernanceEventPayload,
    SkillGovernanceEventRecord, SkillGovernanceRecord, SkillLifecycleState, SkillUsageEventKind,
    SkillUsageObservation,
};

use crate::skill_service_provider_state::{event_id, SkillProviderGovernanceState};

impl SkillProviderGovernanceState {
    /// Store a patch proposal without changing the active governance record.
    pub(crate) async fn propose_patch(
        &self,
        command: SkillEvolutionProposePatchCommand,
    ) -> SkillEvolutionProposePatchResult {
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

        SkillEvolutionProposePatchResult {
            proposal,
            mutated: false,
            captured_at,
        }
    }

    /// Promote one draft proposal into active governance metadata.
    pub(crate) async fn promote_draft(
        &self,
        command: SkillEvolutionPromoteDraftCommand,
    ) -> Result<SkillEvolutionPromoteDraftResult, String> {
        let captured_at = chrono::Utc::now();
        let evidence_ids = sanitized_refs(&command.evidence_ids);
        let policy_decision_refs = sanitized_refs(&command.policy_decision_refs);
        let mut proposals = self.proposals.lock().await;
        let proposal = proposals
            .get_mut(command.proposal_id.trim())
            .ok_or_else(|| {
                "skill proposal promotion requires an existing draft proposal".to_string()
            })?;
        match proposal.lifecycle {
            SkillEvolutionProposalLifecycle::Draft => {}
            SkillEvolutionProposalLifecycle::Promoted => {
                return Err("skill proposal is already promoted".into());
            }
            SkillEvolutionProposalLifecycle::Rejected => {
                return Err("skill proposal is already rejected".into());
            }
        }
        proposal.lifecycle = SkillEvolutionProposalLifecycle::Promoted;
        merge_unique(&mut proposal.evidence_ids, &evidence_ids);
        proposal
            .metadata
            .insert("promotion_reason_ref".into(), command.reason.clone());
        let proposal = proposal.clone();
        drop(proposals);

        let observation = proposal_governance_observation(
            &proposal,
            SkillUsageEventKind::Patched,
            evidence_ids.first().cloned(),
            &command.scope,
        );
        let key = observation.key();
        let mut records = self.records.lock().await;
        let record = records
            .entry(key)
            .and_modify(|record| {
                record.apply(&observation, captured_at);
                record.lifecycle = SkillLifecycleState::Active;
                merge_unique(&mut record.evidence_ids, &evidence_ids);
            })
            .or_insert_with(|| {
                let mut record = SkillGovernanceRecord::from_observation(&observation, captured_at);
                record.lifecycle = SkillLifecycleState::Active;
                merge_unique(&mut record.evidence_ids, &evidence_ids);
                record
            })
            .clone();
        drop(records);

        let mut event = SkillGovernanceEventRecord::new(
            event_id("skill-governance-proposal-promote", captured_at),
            &command.trace,
            command.scope,
            captured_at,
            SkillGovernanceEventPayload::ProposalCreated(proposal.clone()),
        );
        event.policy_decision_ids = policy_decision_refs.clone();
        self.append_event(event).await;

        Ok(SkillEvolutionPromoteDraftResult {
            proposal,
            record,
            mutated: true,
            evidence_ids,
            policy_decision_refs,
            trace_id: command.trace.trace_id,
            captured_at,
        })
    }

    /// Reject one draft proposal while keeping it in the durable proposal map.
    pub(crate) async fn reject_draft(
        &self,
        command: SkillEvolutionRejectDraftCommand,
    ) -> Result<SkillEvolutionRejectDraftResult, String> {
        let captured_at = chrono::Utc::now();
        let evidence_ids = sanitized_refs(&command.evidence_ids);
        let policy_decision_refs = sanitized_refs(&command.policy_decision_refs);
        let mut proposals = self.proposals.lock().await;
        let proposal = proposals
            .get_mut(command.proposal_id.trim())
            .ok_or_else(|| {
                "skill proposal rejection requires an existing draft proposal".to_string()
            })?;
        match proposal.lifecycle {
            SkillEvolutionProposalLifecycle::Draft => {}
            SkillEvolutionProposalLifecycle::Promoted => {
                return Err("skill proposal is already promoted".into());
            }
            SkillEvolutionProposalLifecycle::Rejected => {
                return Err("skill proposal is already rejected".into());
            }
        }
        proposal.lifecycle = SkillEvolutionProposalLifecycle::Rejected;
        merge_unique(&mut proposal.evidence_ids, &evidence_ids);
        proposal
            .metadata
            .insert("rejection_reason_ref".into(), command.rationale.clone());
        let proposal = proposal.clone();
        drop(proposals);

        let mut event = SkillGovernanceEventRecord::new(
            event_id("skill-governance-proposal-reject", captured_at),
            &command.trace,
            command.scope,
            captured_at,
            SkillGovernanceEventPayload::ProposalCreated(proposal.clone()),
        );
        event.policy_decision_ids = policy_decision_refs.clone();
        self.append_event(event).await;

        Ok(SkillEvolutionRejectDraftResult {
            proposal,
            mutated: true,
            evidence_ids,
            policy_decision_refs,
            trace_id: command.trace.trace_id,
            captured_at,
        })
    }
}

fn proposal_governance_observation(
    proposal: &SkillExperienceProposalRecord,
    event: SkillUsageEventKind,
    evidence_id: Option<String>,
    scope: &macaca_skill::SkillServiceScope,
) -> SkillUsageObservation {
    let skill_id = proposal
        .target_skill_id
        .clone()
        .unwrap_or_else(|| proposal.proposal_id.clone());
    let name = proposal
        .target_skill_name
        .clone()
        .unwrap_or_else(|| skill_id.clone());
    let mut metadata = BTreeMap::new();
    metadata.insert("action_ref".into(), proposal.proposal_id.clone());
    metadata.insert("task_id".into(), proposal.task_id.clone());
    SkillUsageObservation {
        skill_id,
        name,
        source: "skill.evolution".into(),
        source_scope: "proposal".into(),
        event,
        author_kind: macaca_skill::SkillAuthorKind::Agent,
        created_by: proposal
            .agent_name
            .clone()
            .or_else(|| scope.agent_name.clone()),
        pinned: None,
        evidence_id,
        metadata,
    }
}

fn sanitized_refs(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect()
}

fn merge_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}
