//! Provider-neutral proposal processing contracts for Skill self-evolution.
//!
//! The types in this module describe the post-capture processing lane between
//! draft proposal creation and any future materialization service.  They are
//! deliberately metadata-only: processing can score quality, group duplicates,
//! and mark readiness, but it cannot write `SKILL.md`, package bytes, executable
//! scripts, aliases, registry entries, or usage files.

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::{
    SkillEvolutionCandidateClassification, SkillEvolutionProposalAction,
    SkillEvolutionProposalLifecycle, SkillExperienceCandidateDestination,
    SkillExperienceProposalRecord, SkillServicePolicyHints, SkillServiceScope,
};

/// Explicit state for one proposal inside the processing lane.
///
/// These states are service-owned review states, not package lifecycle states.
/// `ReadyForMaterialization` only means a future materializer may consider the
/// proposal; it is not evidence that a Skill package was created or activated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillProposalProcessingState {
    Queued,
    Reviewing,
    ReadyForMaterialization,
    SuppressedDuplicate,
    Rejected,
}

impl Default for SkillProposalProcessingState {
    fn default() -> Self {
        Self::Queued
    }
}

/// Bounded deterministic quality score for a proposal processing decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposalQualityScore {
    pub value: u8,
    pub reasons: Vec<String>,
}

impl SkillProposalQualityScore {
    /// Clamp a score into the stable 0-100 range used by shells and reports.
    pub fn new(value: u8, reasons: Vec<String>) -> Self {
        Self {
            value: value.min(100),
            reasons: bounded_reasons(reasons),
        }
    }
}

/// Sanitized duplicate signature used to group repeated low-information drafts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SkillProposalDuplicateSignature {
    pub value: String,
}

impl SkillProposalDuplicateSignature {
    /// Build a stable signature from provider-neutral proposal fields.
    ///
    /// The summary is normalized and truncated so the signature cannot leak raw
    /// task output.  Classification, destination, action, and target identity
    /// keep grouping generic while still separating patch proposals from new
    /// draft proposals.
    pub fn from_parts(
        bounded_summary: &str,
        classification: &SkillEvolutionCandidateClassification,
        destination: &SkillExperienceCandidateDestination,
        action: &SkillEvolutionProposalAction,
        target_skill_id: Option<&str>,
    ) -> Self {
        let normalized_summary = bounded_signature_part(bounded_summary);
        let target = target_skill_id
            .filter(|value| !value.trim().is_empty())
            .map(bounded_signature_part)
            .unwrap_or_else(|| "none".into());
        Self {
            value: format!(
                "summary={normalized_summary}|classification={classification:?}|destination={destination:?}|action={action:?}|target={target}"
            )
            .chars()
            .take(160)
            .collect(),
        }
    }

    /// Build the signature directly from a stored proposal record.
    pub fn from_proposal(proposal: &SkillExperienceProposalRecord) -> Self {
        Self::from_parts(
            &proposal.bounded_summary,
            &proposal.classification,
            &proposal.destination,
            &proposal.recommended_action,
            proposal.target_skill_id.as_deref(),
        )
    }
}

/// One service-owned processing decision for a captured proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposalProcessingRecord {
    pub proposal_id: String,
    pub trace_id: String,
    pub task_id: String,
    pub lifecycle: SkillEvolutionProposalLifecycle,
    pub state: SkillProposalProcessingState,
    pub quality: SkillProposalQualityScore,
    pub duplicate_signature: SkillProposalDuplicateSignature,
    pub duplicate_group_size: u64,
    pub decision_reason: String,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_event_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SkillProposalProcessingRecord {
    /// Create a metadata-only processing record from proposal evidence.
    pub fn from_proposal(
        proposal: &SkillExperienceProposalRecord,
        state: SkillProposalProcessingState,
        quality: SkillProposalQualityScore,
        duplicate_group_size: u64,
        decision_reason: impl Into<String>,
        policy_decision_refs: Vec<String>,
        audit_event_ids: Vec<String>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            proposal_id: proposal.proposal_id.clone(),
            trace_id: proposal.trace_id.clone(),
            task_id: proposal.task_id.clone(),
            lifecycle: proposal.lifecycle.clone(),
            state,
            quality,
            duplicate_signature: SkillProposalDuplicateSignature::from_proposal(proposal),
            duplicate_group_size,
            decision_reason: bounded_reason(decision_reason.into()),
            evidence_ids: sanitize_refs(&proposal.evidence_ids),
            policy_decision_refs: sanitize_refs(&policy_decision_refs),
            audit_event_ids: sanitize_refs(&audit_event_ids),
            created_at: proposal.created_at,
            updated_at,
        }
    }
}

/// Command for running deterministic proposal processing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposalProcessingRunCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub dry_run: bool,
    pub min_ready_score: u8,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_event_ids: Vec<String>,
    pub policy: SkillServicePolicyHints,
}

impl SkillProposalProcessingRunCommand {
    /// Validate admission before apply-mode processing mutates processing state.
    pub fn validate(&self) -> Result<(), String> {
        if self.trace.trace_id.trim().is_empty() {
            return Err("skill proposal processing requires trace_id".into());
        }
        if !self.dry_run {
            if self.evidence_ids.iter().all(|id| id.trim().is_empty()) {
                return Err("skill proposal processing apply requires evidence refs".into());
            }
            if self
                .policy_decision_refs
                .iter()
                .all(|id| id.trim().is_empty())
            {
                return Err("skill proposal processing apply requires policy decision refs".into());
            }
            // Fail-closed readiness (2026-07-08 audit P0-3): the apply path
            // mutates state, so it proceeds only when both readiness signals are
            // explicitly `Some(true)`. `Unknown` (`None`) is treated as denied,
            // matching `mutation.rs`/`proposal_materialization.rs`.
            if self.policy.entitlement_ready != Some(true)
                || self.policy.package_ready != Some(true)
            {
                tracing::warn!(
                    target = "macaca_skill::proposal_processing",
                    event = "proposal_apply_denied",
                    reason_code = "readiness_not_confirmed"
                );
                return Err("skill proposal processing policy denied".into());
            }
        }
        Ok(())
    }
}

/// Result returned by a processing run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposalProcessingRunResult {
    pub run_id: String,
    pub records: Vec<SkillProposalProcessingRecord>,
    pub state_counts: SkillProposalProcessingStateCounts,
    pub duplicate_group_count: u64,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

/// Command for reading processing state without mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposalProcessingSnapshotCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
}

/// Read-only snapshot of proposal processing state and backlog pressure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposalProcessingSnapshotResult {
    pub records: Vec<SkillProposalProcessingRecord>,
    pub state_counts: SkillProposalProcessingStateCounts,
    pub duplicate_group_count: u64,
    pub waiting_proposal_count: u64,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

/// Bounded counters used by operations surfaces without recomputing semantics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposalProcessingStateCounts {
    pub queued: u64,
    pub reviewing: u64,
    pub ready_for_materialization: u64,
    pub suppressed_duplicate: u64,
    pub rejected: u64,
}

impl SkillProposalProcessingStateCounts {
    /// Count records by state inside the service-owned DTO.
    pub fn from_records<'a>(
        records: impl IntoIterator<Item = &'a SkillProposalProcessingRecord>,
    ) -> Self {
        let mut counts = Self::default();
        for record in records {
            match record.state {
                SkillProposalProcessingState::Queued => counts.queued += 1,
                SkillProposalProcessingState::Reviewing => counts.reviewing += 1,
                SkillProposalProcessingState::ReadyForMaterialization => {
                    counts.ready_for_materialization += 1
                }
                SkillProposalProcessingState::SuppressedDuplicate => {
                    counts.suppressed_duplicate += 1
                }
                SkillProposalProcessingState::Rejected => counts.rejected += 1,
            }
        }
        counts
    }
}

fn bounded_signature_part(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
        .chars()
        .take(64)
        .collect()
}

fn bounded_reasons(reasons: Vec<String>) -> Vec<String> {
    reasons.into_iter().map(bounded_reason).take(8).collect()
}

fn bounded_reason(value: String) -> String {
    value.trim().chars().take(160).collect()
}

fn sanitize_refs(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.chars().take(256).collect())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{
        SkillEvolutionCandidateClassification, SkillEvolutionProposalAction,
        SkillExperienceCandidateDestination, SkillProposalDuplicateSignature,
    };

    #[test]
    fn processing_signature_is_bounded_and_generic() {
        let signature = SkillProposalDuplicateSignature::from_parts(
            "Verified terminal task completion observed through service.agent_execution; output_chars=10, artifact_count=0, token_total=unavailable.",
            &SkillEvolutionCandidateClassification::ReusableProcedure,
            &SkillExperienceCandidateDestination::NewSkillDraft,
            &SkillEvolutionProposalAction::CreateDraft,
            None,
        );

        assert!(signature.value.len() <= 160);
        assert!(!signature.value.contains("provider_payload"));
    }
}
