//! Metadata-only umbrella merge proposal contracts.
//!
//! Merge proposals describe what a curation provider wants to do without
//! applying the mutation.  Runtime-host can later feed these DTOs into approval,
//! lifecycle, alias, and content-mutation commands, while SDK, kernel, Web, CLI,
//! and frontend layers keep treating them as opaque Skill service metadata.

use serde::{Deserialize, Serialize};

use crate::alias::SkillAliasKind;
use crate::alias::SkillAliasRecord;
use crate::service_contract::SkillServiceScope;
use macaca_proto::TraceContext;

const MAX_MERGE_SOURCE_SKILLS: usize = 32;
const MAX_MERGE_SUPPORT_FILE_MOVEMENTS: usize = 64;
const MAX_MERGE_ALIAS_EFFECTS: usize = 64;
const MAX_MERGE_EVIDENCE_REFS: usize = 32;
const MAX_MERGE_RATIONALE_BYTES: usize = 1024;
const MAX_MERGE_RISK_REASONS: usize = 16;

/// Target support-file area for detail demoted out of `SKILL.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillMergeSupportFileDestination {
    References,
    Templates,
    Scripts,
    Assets,
}

/// Why content should move from core instructions into a support file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillSupportFileDemotionKind {
    SessionSpecificReference,
    StarterTemplate,
    RepeatableScript,
    Asset,
}

impl SkillSupportFileDemotionKind {
    /// Destination required by the demotion class.
    pub fn expected_destination(&self) -> SkillMergeSupportFileDestination {
        match self {
            Self::SessionSpecificReference => SkillMergeSupportFileDestination::References,
            Self::StarterTemplate => SkillMergeSupportFileDestination::Templates,
            Self::RepeatableScript => SkillMergeSupportFileDestination::Scripts,
            Self::Asset => SkillMergeSupportFileDestination::Assets,
        }
    }
}

/// Standalone support-file demotion proposal used before merge apply exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSupportFileDemotionProposal {
    pub skill_id: String,
    pub source_path: String,
    pub destination: SkillMergeSupportFileDestination,
    pub kind: SkillSupportFileDemotionKind,
    pub destination_path: String,
    pub rationale: String,
    pub evidence_ids: Vec<String>,
}

impl SkillSupportFileDemotionProposal {
    /// Validate destination class, identity, rationale, and evidence refs.
    pub fn validate(&self) -> Result<(), String> {
        ensure_present(&self.skill_id, "support-file demotion requires skill id")?;
        ensure_present(
            &self.source_path,
            "support-file demotion requires source path",
        )?;
        ensure_present(
            &self.destination_path,
            "support-file demotion requires destination path",
        )?;
        ensure_bounded_text(&self.rationale, "support-file demotion rationale")?;
        ensure_evidence_refs(&self.evidence_ids, "support-file demotion")?;
        if self.destination != self.kind.expected_destination() {
            return Err("support-file demotion destination does not match demotion kind".into());
        }
        Ok(())
    }
}

/// One planned movement from an absorbed skill into a support-file area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMergeSupportFileMovement {
    pub source_skill_id: String,
    pub source_path: String,
    pub destination: SkillMergeSupportFileDestination,
    pub destination_path: String,
    pub rationale: String,
    pub evidence_ids: Vec<String>,
}

impl SkillMergeSupportFileMovement {
    /// Validate movement identity and bounded audit metadata.
    pub fn validate(&self) -> Result<(), String> {
        ensure_present(
            &self.source_skill_id,
            "support-file movement requires source skill id",
        )?;
        ensure_present(
            &self.source_path,
            "support-file movement requires source path",
        )?;
        ensure_present(
            &self.destination_path,
            "support-file movement requires destination path",
        )?;
        ensure_bounded_text(&self.rationale, "support-file movement rationale")?;
        ensure_evidence_refs(&self.evidence_ids, "support-file movement")?;
        Ok(())
    }
}

/// Alias effect that an approved merge must materialize after lifecycle checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMergeAliasEffect {
    pub source_skill_id: String,
    pub source_name: String,
    pub target_skill_id: String,
    pub target_name: String,
    pub kind: SkillAliasKind,
    pub rationale: String,
    pub evidence_ids: Vec<String>,
}

impl SkillMergeAliasEffect {
    /// Validate that an alias effect can become a service-owned alias record.
    pub fn validate_for_target(&self, target_skill_id: &str) -> Result<(), String> {
        ensure_present(
            &self.source_skill_id,
            "merge alias requires source skill id",
        )?;
        ensure_present(&self.source_name, "merge alias requires source name")?;
        ensure_present(
            &self.target_skill_id,
            "merge alias requires target skill id",
        )?;
        ensure_present(&self.target_name, "merge alias requires target name")?;
        ensure_bounded_text(&self.rationale, "merge alias rationale")?;
        ensure_evidence_refs(&self.evidence_ids, "merge alias")?;
        if self.target_skill_id.trim() != target_skill_id.trim() {
            return Err("merge alias target must match proposal umbrella target".into());
        }
        Ok(())
    }
}

/// Bounded policy-facing risk score for an umbrella merge proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillMergeRiskScore {
    pub score: f32,
    pub reasons: Vec<String>,
}

impl Default for SkillMergeRiskScore {
    fn default() -> Self {
        Self {
            score: 0.0,
            reasons: Vec::new(),
        }
    }
}

impl SkillMergeRiskScore {
    /// Validate score range and bounded explanatory reasons.
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.score) {
            return Err("merge risk score must be between 0 and 1".into());
        }
        if self.reasons.len() > MAX_MERGE_RISK_REASONS {
            return Err("merge risk score carries too many reasons".into());
        }
        for reason in &self.reasons {
            ensure_bounded_text(reason, "merge risk reason")?;
        }
        Ok(())
    }
}

/// Proposal DTO for absorbing narrow skills into an umbrella skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillUmbrellaMergeProposal {
    pub proposal_id: String,
    pub source_skill_ids: Vec<String>,
    pub target_umbrella_skill_id: String,
    pub support_file_movements: Vec<SkillMergeSupportFileMovement>,
    pub alias_effects: Vec<SkillMergeAliasEffect>,
    pub risk: SkillMergeRiskScore,
    pub policy_decision_refs: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub rationale: String,
}

/// Approval-gated command envelope for applying one umbrella merge.
///
/// The command intentionally carries refs to lifecycle and safe mutation
/// commands rather than raw file content.  Runtime providers must materialize
/// the actual supersede, alias, and support-file writes through those narrower
/// command surfaces so merge apply cannot become an unbounded mutation path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillUmbrellaMergeApplyCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub proposal: SkillUmbrellaMergeProposal,
    pub approval_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub lifecycle_transition_refs: Vec<String>,
    pub safe_mutation_refs: Vec<String>,
    pub audit_event_ids: Vec<String>,
}

/// Bounded result returned after an approved umbrella merge apply.
///
/// The result exposes refs, lifecycle ids, and alias metadata only.  It never
/// embeds skill bodies, package bytes, provider payloads, prompts, manifests,
/// credentials, or support-file content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillMergeApplyResult {
    pub proposal_id: String,
    pub target_umbrella_skill_id: String,
    pub superseded_skill_ids: Vec<String>,
    pub alias_records: Vec<SkillAliasRecord>,
    pub support_file_movement_refs: Vec<String>,
    pub bounded_report_summary: String,
    pub evidence_ids: Vec<String>,
    pub report_ref: Option<String>,
    pub rollback_ref: Option<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_event_ids: Vec<String>,
    pub mutated: bool,
    pub captured_at: chrono::DateTime<chrono::Utc>,
}

impl SkillUmbrellaMergeApplyCommand {
    /// Validate approval and command-ref gates before a provider can apply.
    pub fn validate(&self) -> Result<(), String> {
        if self.trace.trace_id.trim().is_empty() {
            return Err("umbrella merge apply requires trace_id".into());
        }
        self.proposal.validate()?;
        ensure_non_empty_list(
            &self.approval_refs,
            MAX_MERGE_EVIDENCE_REFS,
            "merge approval refs",
        )?;
        ensure_non_empty_list(
            &self.policy_decision_refs,
            MAX_MERGE_EVIDENCE_REFS,
            "merge policy decision refs",
        )?;
        ensure_non_empty_list(
            &self.lifecycle_transition_refs,
            MAX_MERGE_EVIDENCE_REFS,
            "merge lifecycle transition refs",
        )?;
        ensure_non_empty_list(
            &self.safe_mutation_refs,
            MAX_MERGE_EVIDENCE_REFS,
            "merge safe mutation refs",
        )?;
        ensure_evidence_refs(&self.audit_event_ids, "merge audit event")?;
        Ok(())
    }
}

impl SkillUmbrellaMergeProposal {
    /// Build the smallest proposal shape used by policy and validation tests.
    pub fn metadata_only(
        proposal_id: impl Into<String>,
        source_skill_ids: Vec<String>,
        target_umbrella_skill_id: impl Into<String>,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            source_skill_ids,
            target_umbrella_skill_id: target_umbrella_skill_id.into(),
            support_file_movements: Vec::new(),
            alias_effects: Vec::new(),
            risk: SkillMergeRiskScore::default(),
            policy_decision_refs: Vec::new(),
            evidence_ids: Vec::new(),
            rationale: rationale.into(),
        }
    }

    /// Validate the DTO envelope before it can enter policy or apply flows.
    pub fn validate(&self) -> Result<(), String> {
        ensure_present(&self.proposal_id, "umbrella merge proposal requires id")?;
        ensure_present(
            &self.target_umbrella_skill_id,
            "umbrella merge proposal requires target umbrella skill id",
        )?;
        ensure_non_empty_list(
            &self.source_skill_ids,
            MAX_MERGE_SOURCE_SKILLS,
            "umbrella merge proposal source skills",
        )?;
        ensure_bounded_text(&self.rationale, "umbrella merge proposal rationale")?;
        ensure_evidence_refs(&self.evidence_ids, "umbrella merge proposal")?;
        ensure_evidence_refs(&self.policy_decision_refs, "umbrella merge policy")?;
        if self.support_file_movements.len() > MAX_MERGE_SUPPORT_FILE_MOVEMENTS {
            return Err("umbrella merge proposal carries too many support-file movements".into());
        }
        if self.alias_effects.len() > MAX_MERGE_ALIAS_EFFECTS {
            return Err("umbrella merge proposal carries too many alias effects".into());
        }
        for source_skill_id in &self.source_skill_ids {
            ensure_present(source_skill_id, "umbrella merge proposal source skill id")?;
            if source_skill_id.trim() == self.target_umbrella_skill_id.trim() {
                return Err("umbrella merge source cannot equal target umbrella skill".into());
            }
        }
        for movement in &self.support_file_movements {
            movement.validate()?;
        }
        for alias in &self.alias_effects {
            alias.validate_for_target(&self.target_umbrella_skill_id)?;
        }
        self.risk.validate()
    }
}

fn ensure_present(value: &str, message: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(message.into());
    }
    Ok(())
}

fn ensure_bounded_text(value: &str, label: &str) -> Result<(), String> {
    ensure_present(value, &format!("{label} is required"))?;
    if value.len() > MAX_MERGE_RATIONALE_BYTES {
        return Err(format!("{label} exceeds bounded metadata"));
    }
    Ok(())
}

fn ensure_non_empty_list(values: &[String], max: usize, label: &str) -> Result<(), String> {
    if values.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if values.len() > max {
        return Err(format!("{label} exceeds bounded metadata"));
    }
    Ok(())
}

fn ensure_evidence_refs(values: &[String], label: &str) -> Result<(), String> {
    if values.len() > MAX_MERGE_EVIDENCE_REFS {
        return Err(format!("{label} carries too many evidence refs"));
    }
    if values.iter().any(|id| id.trim().is_empty()) {
        return Err(format!("{label} evidence refs must not be blank"));
    }
    Ok(())
}
