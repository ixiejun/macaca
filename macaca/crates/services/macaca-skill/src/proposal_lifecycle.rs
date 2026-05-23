//! Proposal lifecycle commands for self-evolving Skill governance.
//!
//! The DTOs in this module keep proposal promotion and rejection behind the
//! Skill service Command boundary.  They carry only trace, scope, evidence,
//! policy decision refs, and bounded proposal metadata so SDK and shell callers
//! can request side effects without owning lifecycle semantics.

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::{
    SkillEvolutionProposalAction, SkillExperienceCandidate, SkillExperienceCandidateDestination,
    SkillExperienceProposalRecord, SkillGovernanceRecord, SkillServicePolicyHints,
    SkillServiceScope,
};

/// Lifecycle state for a sanitized Skill evolution proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillEvolutionProposalLifecycle {
    Draft,
    Promoted,
    Rejected,
}

impl Default for SkillEvolutionProposalLifecycle {
    fn default() -> Self {
        Self::Draft
    }
}

/// Command for proposing a bounded patch to an existing governed skill.
///
/// The command reuses [`SkillExperienceCandidate`] so Task and Semantic Review
/// providers can submit one evidence envelope.  Validation constrains it to the
/// patch lane before any provider stores draft metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvolutionProposePatchCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub candidate: SkillExperienceCandidate,
}

impl SkillEvolutionProposePatchCommand {
    /// Validate both generic experience evidence and patch-specific targeting.
    pub fn validate(&self) -> Result<(), String> {
        self.candidate.validate()?;
        if self.candidate.destination
            != SkillExperienceCandidateDestination::ExistingSkillPatchProposal
        {
            return Err("skill patch proposal requires existing-skill patch destination".into());
        }
        if self.candidate.recommended_action != SkillEvolutionProposalAction::ProposePatch {
            return Err("skill patch proposal requires propose-patch action".into());
        }
        if self
            .candidate
            .target_skill_id
            .as_ref()
            .is_none_or(|id| id.trim().is_empty())
        {
            return Err("skill patch proposal requires target skill id".into());
        }
        Ok(())
    }
}

/// Result for draft patch proposal creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvolutionProposePatchResult {
    pub proposal: SkillExperienceProposalRecord,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

/// Command for policy-gated promotion of a draft proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvolutionPromoteDraftCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub proposal_id: String,
    pub reason: String,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub policy: SkillServicePolicyHints,
}

impl SkillEvolutionPromoteDraftCommand {
    /// Validate the approval envelope before promotion mutates governance state.
    pub fn validate(&self) -> Result<(), String> {
        validate_proposal_lifecycle_envelope(
            &self.proposal_id,
            &self.reason,
            &self.evidence_ids,
            &self.policy_decision_refs,
            &self.policy,
            "promotion",
        )
    }
}

/// Result returned after a draft is promoted into active governance metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvolutionPromoteDraftResult {
    pub proposal: SkillExperienceProposalRecord,
    pub record: SkillGovernanceRecord,
    pub mutated: bool,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub trace_id: String,
    pub captured_at: DateTime<Utc>,
}

/// Command for policy-gated rejection of a draft proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvolutionRejectDraftCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub proposal_id: String,
    pub rationale: String,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub policy: SkillServicePolicyHints,
}

impl SkillEvolutionRejectDraftCommand {
    /// Validate the approval envelope before rejection mutates proposal state.
    pub fn validate(&self) -> Result<(), String> {
        validate_proposal_lifecycle_envelope(
            &self.proposal_id,
            &self.rationale,
            &self.evidence_ids,
            &self.policy_decision_refs,
            &self.policy,
            "rejection",
        )
    }
}

/// Result returned after a proposal is rejected and retained for audit replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvolutionRejectDraftResult {
    pub proposal: SkillExperienceProposalRecord,
    pub mutated: bool,
    pub evidence_ids: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub trace_id: String,
    pub captured_at: DateTime<Utc>,
}

fn validate_proposal_lifecycle_envelope(
    proposal_id: &str,
    reason: &str,
    evidence_ids: &[String],
    policy_decision_refs: &[String],
    policy: &SkillServicePolicyHints,
    operation: &str,
) -> Result<(), String> {
    if proposal_id.trim().is_empty() {
        return Err(format!("skill proposal {operation} requires proposal id"));
    }
    if reason.trim().is_empty() {
        return Err(format!("skill proposal {operation} requires rationale"));
    }
    if evidence_ids.iter().all(|id| id.trim().is_empty()) {
        return Err(format!(
            "skill proposal {operation} requires evidence references"
        ));
    }
    if policy_decision_refs.iter().all(|id| id.trim().is_empty()) {
        return Err(format!(
            "skill proposal {operation} requires policy decision references"
        ));
    }
    if policy.entitlement_ready == Some(false) || policy.package_ready == Some(false) {
        return Err(format!("skill proposal {operation} policy denied"));
    }
    Ok(())
}
