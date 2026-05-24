//! Deterministic proposal processing Strategy for the built-in Skill provider.
//!
//! This module owns the local post-capture processing lane for Skill evolution
//! proposals.  It intentionally mutates only processing records.  It does not
//! modify proposal lifecycle, Skill package files, aliases, registries, usage
//! telemetry, executable scripts, or active catalog state.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, TraceContext};
use macaca_skill::{
    SkillEvolutionCandidateClassification, SkillExperienceCandidateDestination,
    SkillExperienceProposalRecord, SkillProposalDuplicateSignature, SkillProposalProcessingRecord,
    SkillProposalProcessingRunCommand, SkillProposalProcessingRunResult,
    SkillProposalProcessingSnapshotCommand, SkillProposalProcessingSnapshotResult,
    SkillProposalProcessingState, SkillProposalProcessingStateCounts, SkillProposalQualityScore,
};
use serde_json::Value;

use crate::skill_service_codec::{decode, service_result, to_value};
use crate::skill_service_provider_state::{event_id, SkillProviderGovernanceState};

/// Handle a traced proposal-processing run command.
pub(crate) async fn run_command(
    state: &Arc<SkillProviderGovernanceState>,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillProposalProcessingRunCommand = decode(payload)?;
    typed.validate().map_err(ServiceError::InvalidArgument)?;
    let result = state.process_proposals(typed.clone()).await;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        run_id = %result.run_id,
        dry_run = typed.dry_run,
        proposals = result.records.len(),
        duplicate_groups = result.duplicate_group_count,
        ready = result.state_counts.ready_for_materialization,
        suppressed = result.state_counts.suppressed_duplicate,
        rejected = result.state_counts.rejected,
        evidence_refs = typed.evidence_ids.len(),
        policy_decision_refs = typed.policy_decision_refs.len(),
        mutated = result.mutated,
        "skill proposal processing run completed"
    );
    Ok(service_result(to_value(result)?, trace))
}

/// Handle a read-only proposal-processing snapshot command.
pub(crate) async fn snapshot_command(
    state: &Arc<SkillProviderGovernanceState>,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillProposalProcessingSnapshotCommand = decode(payload)?;
    let result = state.proposal_processing_snapshot(&typed).await;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        records = result.records.len(),
        waiting = result.waiting_proposal_count,
        duplicate_groups = result.duplicate_group_count,
        ready = result.state_counts.ready_for_materialization,
        suppressed = result.state_counts.suppressed_duplicate,
        rejected = result.state_counts.rejected,
        "skill proposal processing snapshot emitted"
    );
    Ok(service_result(to_value(result)?, trace))
}

impl SkillProviderGovernanceState {
    /// Process captured proposals through the local deterministic Strategy.
    pub(crate) async fn process_proposals(
        &self,
        command: SkillProposalProcessingRunCommand,
    ) -> SkillProposalProcessingRunResult {
        let captured_at = chrono::Utc::now();
        let proposals = self.sorted_proposals_for_scope(&command.scope).await;
        let group_sizes = duplicate_group_sizes(&proposals);
        let mut first_ready_signature = BTreeSet::new();
        let mut records = Vec::new();

        tracing::info!(
            trace_id = %command.trace.trace_id,
            dry_run = command.dry_run,
            proposals = proposals.len(),
            scope_application_id = ?command.scope.application_id,
            duplicate_groups = group_sizes.len(),
            min_ready_score = command.min_ready_score,
            "skill proposal processing deterministic phase started"
        );

        for proposal in &proposals {
            let signature = SkillProposalDuplicateSignature::from_proposal(proposal);
            let group_size = group_sizes.get(&signature).copied().unwrap_or(1);
            let quality = score_proposal(proposal);
            let state = processing_state(
                &signature,
                group_size,
                quality.value,
                command.min_ready_score,
                &mut first_ready_signature,
            );
            let decision_reason = decision_reason(&state, group_size, quality.value);
            records.push(SkillProposalProcessingRecord::from_proposal(
                proposal,
                state,
                quality,
                group_size,
                decision_reason,
                command.policy_decision_refs.clone(),
                command.audit_event_ids.clone(),
                captured_at,
            ));
        }

        if !command.dry_run {
            let mut state = self.proposal_processing.lock().await;
            for record in &records {
                state.insert(record.proposal_id.clone(), record.clone());
            }
        }

        let state_counts = SkillProposalProcessingStateCounts::from_records(records.iter());
        SkillProposalProcessingRunResult {
            run_id: event_id("skill-proposal-processing-run", captured_at),
            duplicate_group_count: group_sizes.len() as u64,
            mutated: !command.dry_run,
            captured_at,
            records,
            state_counts,
        }
    }

    /// Return read-only processing records and backlog counters.
    pub(crate) async fn proposal_processing_snapshot(
        &self,
        command: &SkillProposalProcessingSnapshotCommand,
    ) -> SkillProposalProcessingSnapshotResult {
        let captured_at = chrono::Utc::now();
        let processed_records = self
            .proposal_processing
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();

        let processed_ids = processed_records
            .iter()
            .map(|record| record.proposal_id.clone())
            .collect::<BTreeSet<_>>();

        let proposals = self.sorted_proposals_for_scope(&command.scope).await;
        let group_sizes = duplicate_group_sizes(&proposals);
        let mut records = processed_records;
        let mut waiting_proposal_count = 0u64;

        // Snapshot reads must expose backlog pressure even before an apply-mode
        // processing run mutates the durable processing map.  These synthesized
        // Queued records are metadata-only and read-only: they make pending work
        // observable without changing proposal lifecycle, Skill files, registry
        // state, usage telemetry, or materialization outputs.
        for proposal in &proposals {
            if processed_ids.contains(&proposal.proposal_id) {
                continue;
            }
            waiting_proposal_count += 1;
            let signature = SkillProposalDuplicateSignature::from_proposal(proposal);
            let group_size = group_sizes.get(&signature).copied().unwrap_or(1);
            let quality = score_proposal(proposal);
            let quality_score = quality.value;
            records.push(SkillProposalProcessingRecord::from_proposal(
                proposal,
                SkillProposalProcessingState::Queued,
                quality,
                group_size,
                decision_reason(
                    &SkillProposalProcessingState::Queued,
                    group_size,
                    quality_score,
                ),
                Vec::new(),
                Vec::new(),
                captured_at,
            ));
        }

        records.sort_by(|left, right| left.proposal_id.cmp(&right.proposal_id));
        let duplicate_group_count = records
            .iter()
            .map(|record| record.duplicate_signature.clone())
            .collect::<BTreeSet<_>>()
            .len() as u64;
        let state_counts = SkillProposalProcessingStateCounts::from_records(records.iter());

        tracing::info!(
            trace_id = %command.trace.trace_id,
            processed_records = processed_ids.len(),
            scope_application_id = ?command.scope.application_id,
            queued_records = waiting_proposal_count,
            total_records = records.len(),
            duplicate_groups = duplicate_group_count,
            "skill proposal processing snapshot synthesized queued backlog records"
        );

        SkillProposalProcessingSnapshotResult {
            records,
            state_counts,
            duplicate_group_count,
            waiting_proposal_count,
            mutated: false,
            captured_at,
        }
    }

    async fn sorted_proposals_for_scope(
        &self,
        scope: &macaca_skill::SkillServiceScope,
    ) -> Vec<SkillExperienceProposalRecord> {
        let mut proposals = self
            .proposals
            .lock()
            .await
            .values()
            .filter(|proposal| proposal_matches_scope(proposal, scope))
            .cloned()
            .collect::<Vec<_>>();
        proposals.sort_by(|left, right| left.proposal_id.cmp(&right.proposal_id));
        proposals
    }
}

fn proposal_matches_scope(
    proposal: &SkillExperienceProposalRecord,
    scope: &macaca_skill::SkillServiceScope,
) -> bool {
    if let Some(application_id) = scope.application_id {
        if proposal.application_id != Some(application_id) {
            return false;
        }
    }
    if let Some(session_id) = scope.session_id.as_deref() {
        if proposal.session_id.as_deref() != Some(session_id) {
            return false;
        }
    }
    if let Some(agent_name) = scope.agent_name.as_deref() {
        if proposal.agent_name.as_deref() != Some(agent_name) {
            return false;
        }
    }
    true
}

fn duplicate_group_sizes(
    proposals: &[SkillExperienceProposalRecord],
) -> BTreeMap<SkillProposalDuplicateSignature, u64> {
    let mut sizes = BTreeMap::new();
    for proposal in proposals {
        *sizes
            .entry(SkillProposalDuplicateSignature::from_proposal(proposal))
            .or_insert(0) += 1;
    }
    sizes
}

fn score_proposal(proposal: &SkillExperienceProposalRecord) -> SkillProposalQualityScore {
    let mut score = 30u8;
    let mut reasons = vec!["proposal_has_verified_capture_metadata".into()];
    if !proposal.evidence_ids.is_empty() {
        score += 20;
        reasons.push("proposal_has_evidence_refs".into());
    }
    if proposal.trace_digest.is_some() {
        score += 10;
        reasons.push("proposal_has_trace_digest".into());
    }
    if proposal.classification == SkillEvolutionCandidateClassification::ReusableProcedure {
        score += 10;
        reasons.push("proposal_classified_reusable_procedure".into());
    }
    if proposal.destination == SkillExperienceCandidateDestination::NewSkillDraft {
        score += 10;
        reasons.push("proposal_targets_new_skill_draft_lane".into());
    }
    if proposal.reusable_procedure.chars().count() > 80 {
        score += 10;
        reasons.push("proposal_has_bounded_reusable_procedure".into());
    }
    if proposal_has_artifact_evidence(proposal) {
        score += 10;
        reasons.push("proposal_reports_artifact_refs".into());
    }
    SkillProposalQualityScore::new(score, reasons)
}

fn proposal_has_artifact_evidence(proposal: &SkillExperienceProposalRecord) -> bool {
    // Older proposal records may carry a bounded artifact count, while newer
    // service-owned capture stores durable artifact references as sanitized
    // metadata keys.  Treat either form as artifact evidence so the processing
    // Strategy remains backward-compatible and avoids raw application paths.
    if proposal
        .metadata
        .get("artifact_count")
        .is_some_and(|value| value.trim() != "0")
    {
        return true;
    }
    proposal.metadata.iter().any(|(key, value)| {
        !value.trim().is_empty()
            && (key == "artifact_ref" || key.starts_with("evidence_ref.artifact_"))
    })
}

fn processing_state(
    signature: &SkillProposalDuplicateSignature,
    group_size: u64,
    quality_score: u8,
    min_ready_score: u8,
    first_ready_signature: &mut BTreeSet<SkillProposalDuplicateSignature>,
) -> SkillProposalProcessingState {
    if quality_score < min_ready_score {
        return SkillProposalProcessingState::Rejected;
    }
    if group_size > 1 && first_ready_signature.contains(signature) {
        return SkillProposalProcessingState::SuppressedDuplicate;
    }
    first_ready_signature.insert(signature.clone());
    SkillProposalProcessingState::ReadyForMaterialization
}

fn decision_reason(
    state: &SkillProposalProcessingState,
    group_size: u64,
    quality_score: u8,
) -> String {
    match state {
        SkillProposalProcessingState::ReadyForMaterialization => {
            format!("quality_score={quality_score}; first eligible proposal in duplicate_group_size={group_size}")
        }
        SkillProposalProcessingState::SuppressedDuplicate => {
            format!("quality_score={quality_score}; suppressed duplicate in duplicate_group_size={group_size}")
        }
        SkillProposalProcessingState::Rejected => {
            format!("quality_score={quality_score}; below configured ready threshold")
        }
        SkillProposalProcessingState::Queued => "proposal is queued for future processing".into(),
        SkillProposalProcessingState::Reviewing => {
            "proposal is currently under service-owned review".into()
        }
    }
}
