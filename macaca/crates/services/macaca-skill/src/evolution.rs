//! Draft-only Skill experience evolution contracts.
//!
//! These DTOs model the first safe step of self-evolving skills: a verified
//! task may produce a sanitized proposal, but active skill files and catalog
//! state remain unchanged.  Future providers can persist proposals in Store or
//! EventLog and add approval/promotion commands without changing the caller
//! contract introduced here.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use macaca_proto::{ApplicationId, TraceContext};
use serde::{Deserialize, Serialize};

use crate::service_contract::SkillServiceScope;

/// High-level classification for a bounded task experience candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillEvolutionCandidateClassification {
    ReusableProcedure,
    ProjectConvention,
    KnowledgeOnly,
    OneOffTaskArtifact,
}

/// Non-destructive recommendation emitted for a proposed experience.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillEvolutionProposalAction {
    CreateDraft,
    ProposePatch,
    WriteSupportFileDraft,
    Discard,
}

/// Evidence-gate verdict supplied by Task/Autonomy before Skill Evolution runs.
///
/// The Skill service must not infer task completion from free-form summaries.
/// A caller has to pass an explicit, provider-neutral gate result so the
/// evolution path can reject unverified experience before it creates even a
/// draft-only proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillExperienceEvidenceGateStatus {
    Accepted,
    Rejected,
    Missing,
}

impl Default for SkillExperienceEvidenceGateStatus {
    fn default() -> Self {
        Self::Accepted
    }
}

/// Generic destination selected for a bounded reusable experience candidate.
///
/// These labels describe OS-owned routing classes, not application domains.
/// Memory and Knowledge destinations are routed to their owning service
/// facades by runtime-host providers; skill-oriented destinations remain in
/// the Skill governance proposal workflow until a later approval command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillExperienceCandidateDestination {
    MemoryFact,
    KnowledgeDigest,
    ExistingSkillPatchProposal,
    NewSkillDraft,
    SupportFileDraft,
    NoOp,
}

impl Default for SkillExperienceCandidateDestination {
    fn default() -> Self {
        Self::NewSkillDraft
    }
}

/// Sanitized task evidence that may become a governed skill proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillExperienceCandidate {
    pub task_id: String,
    pub session_id: Option<String>,
    pub application_id: Option<ApplicationId>,
    pub agent_name: Option<String>,
    #[serde(default = "default_verified_terminal_success")]
    pub verified_terminal_success: bool,
    #[serde(default)]
    pub evidence_gate: SkillExperienceEvidenceGateStatus,
    pub bounded_summary: String,
    #[serde(default)]
    pub trace_digest: Option<String>,
    #[serde(default)]
    pub memory_digest_refs: Vec<String>,
    pub reusable_procedure: String,
    pub classification: SkillEvolutionCandidateClassification,
    #[serde(default)]
    pub destination: SkillExperienceCandidateDestination,
    pub recommended_action: SkillEvolutionProposalAction,
    pub target_skill_id: Option<String>,
    pub target_skill_name: Option<String>,
    pub evidence_ids: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl SkillExperienceCandidate {
    /// Validate the minimum evidence required before a proposal can exist.
    pub fn validate(&self) -> Result<(), String> {
        const MAX_BOUNDED_SUMMARY_CHARS: usize = 2_048;
        const MAX_REUSABLE_PROCEDURE_CHARS: usize = 4_096;

        if self.task_id.trim().is_empty() {
            return Err("skill experience proposal requires task_id".into());
        }
        if !self.verified_terminal_success {
            return Err("skill experience proposal requires verified terminal task success".into());
        }
        if self.evidence_gate != SkillExperienceEvidenceGateStatus::Accepted {
            return Err("skill experience proposal requires accepted evidence gate".into());
        }
        if self.bounded_summary.trim().is_empty() {
            return Err("skill experience proposal requires bounded summary".into());
        }
        if self.bounded_summary.chars().count() > MAX_BOUNDED_SUMMARY_CHARS {
            return Err("skill experience proposal bounded summary is too large".into());
        }
        if self.reusable_procedure.trim().is_empty() {
            return Err("skill experience proposal requires reusable procedure".into());
        }
        if self.reusable_procedure.chars().count() > MAX_REUSABLE_PROCEDURE_CHARS {
            return Err("skill experience proposal reusable procedure is too large".into());
        }
        if self.evidence_ids.iter().all(|id| id.trim().is_empty()) {
            return Err("skill experience proposal requires evidence references".into());
        }
        Ok(())
    }
}

fn default_verified_terminal_success() -> bool {
    true
}

/// Stored draft proposal metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillExperienceProposalRecord {
    pub proposal_id: String,
    pub trace_id: String,
    pub task_id: String,
    pub session_id: Option<String>,
    pub application_id: Option<ApplicationId>,
    pub agent_name: Option<String>,
    pub bounded_summary: String,
    pub trace_digest: Option<String>,
    pub memory_digest_refs: Vec<String>,
    pub reusable_procedure: String,
    pub classification: SkillEvolutionCandidateClassification,
    pub destination: SkillExperienceCandidateDestination,
    pub recommended_action: SkillEvolutionProposalAction,
    pub target_skill_id: Option<String>,
    pub target_skill_name: Option<String>,
    pub evidence_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Structured status for routing non-skill destinations to their owning service.
///
/// The value is serialized as a stable lowercase label so SDK and shell
/// adapters can display it without learning Memory, Knowledge, or Skill
/// service internals.  Routing may be unavailable when the destination service
/// is absent; that is an explicit state, never fake success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillExperienceDestinationRouteStatus {
    Routed,
    Skipped,
    Unavailable,
    Failed,
}

impl SkillExperienceDestinationRouteStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Routed => "routed",
            Self::Skipped => "skipped",
            Self::Unavailable => "unavailable",
            Self::Failed => "failed",
        }
    }
}

impl Default for SkillExperienceDestinationRouteStatus {
    fn default() -> Self {
        Self::Skipped
    }
}

/// Provider-neutral route result for the candidate destination.
///
/// `target_ref` stores only stable ids or synthetic service refs.  It must not
/// contain memory bodies, knowledge statements, raw task output, prompts, or
/// provider payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillExperienceDestinationRouteResult {
    pub destination: SkillExperienceCandidateDestination,
    pub status: SkillExperienceDestinationRouteStatus,
    pub target_ref: Option<String>,
    pub reason: Option<String>,
}

impl SkillExperienceDestinationRouteResult {
    pub fn skipped(
        destination: SkillExperienceCandidateDestination,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            destination,
            status: SkillExperienceDestinationRouteStatus::Skipped,
            target_ref: None,
            reason: Some(bounded_metadata_value(reason.into().trim())),
        }
    }

    pub fn routed(
        destination: SkillExperienceCandidateDestination,
        target_ref: impl Into<String>,
    ) -> Self {
        Self {
            destination,
            status: SkillExperienceDestinationRouteStatus::Routed,
            target_ref: Some(bounded_metadata_value(target_ref.into().trim())),
            reason: None,
        }
    }

    pub fn unavailable(
        destination: SkillExperienceCandidateDestination,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            destination,
            status: SkillExperienceDestinationRouteStatus::Unavailable,
            target_ref: None,
            reason: Some(bounded_metadata_value(reason.into().trim())),
        }
    }

    pub fn failed(
        destination: SkillExperienceCandidateDestination,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            destination,
            status: SkillExperienceDestinationRouteStatus::Failed,
            target_ref: None,
            reason: Some(bounded_metadata_value(reason.into().trim())),
        }
    }
}

impl Default for SkillExperienceDestinationRouteResult {
    fn default() -> Self {
        Self::skipped(
            SkillExperienceCandidateDestination::NewSkillDraft,
            "candidate remains in skill governance proposal store",
        )
    }
}

impl SkillExperienceProposalRecord {
    /// Build a deterministic, sanitized proposal record from validated input.
    pub fn from_candidate(
        trace: &TraceContext,
        candidate: SkillExperienceCandidate,
        created_at: DateTime<Utc>,
    ) -> Self {
        let created_at_nanos = created_at
            .timestamp_nanos_opt()
            .unwrap_or_else(|| created_at.timestamp_micros() * 1_000);
        let proposal_id = format!(
            "skill-exp-{}-{}",
            candidate.task_id.trim(),
            created_at_nanos
        );
        Self {
            proposal_id,
            trace_id: trace.trace_id.clone(),
            task_id: candidate.task_id,
            session_id: candidate.session_id,
            application_id: candidate.application_id,
            agent_name: candidate.agent_name,
            bounded_summary: candidate.bounded_summary,
            trace_digest: candidate
                .trace_digest
                .filter(|digest| !digest.trim().is_empty())
                .map(|digest| bounded_metadata_value(digest.trim())),
            memory_digest_refs: candidate
                .memory_digest_refs
                .into_iter()
                .filter(|digest| !digest.trim().is_empty())
                .map(|digest| bounded_metadata_value(digest.trim()))
                .collect(),
            reusable_procedure: candidate.reusable_procedure,
            classification: candidate.classification,
            destination: candidate.destination,
            recommended_action: candidate.recommended_action,
            target_skill_id: candidate.target_skill_id,
            target_skill_name: candidate.target_skill_name,
            evidence_ids: candidate
                .evidence_ids
                .into_iter()
                .filter(|id| !id.trim().is_empty())
                .collect(),
            created_at,
            metadata: sanitize_proposal_metadata(candidate.metadata),
        }
    }
}

fn sanitize_proposal_metadata(metadata: BTreeMap<String, String>) -> BTreeMap<String, String> {
    metadata
        .into_iter()
        .filter_map(|(key, value)| {
            let key = key.trim();
            if !is_safe_proposal_metadata_key(key) {
                return None;
            }
            let value = value.trim();
            if value.is_empty() {
                return None;
            }
            Some((key.to_string(), bounded_metadata_value(value)))
        })
        .collect()
}

fn is_safe_proposal_metadata_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    if [
        "raw",
        "task_output",
        "provider_payload",
        "secret",
        "credential",
        "signature",
        "package_bytes",
        "manifest_body",
        "skill_body",
        "body",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
    {
        return false;
    }
    key.ends_with("_id") || key.ends_with("_ref") || key.starts_with("evidence_ref.")
}

fn bounded_metadata_value(value: &str) -> String {
    const MAX_METADATA_VALUE_CHARS: usize = 256;
    value.chars().take(MAX_METADATA_VALUE_CHARS).collect()
}

/// Command for proposing reusable task experience as a draft skill asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillExperienceProposalCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub candidate: SkillExperienceCandidate,
}

/// Result for proposal creation. `mutated` stays false in this first slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillExperienceProposalResult {
    pub proposal: SkillExperienceProposalRecord,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
    #[serde(default)]
    pub destination_route: SkillExperienceDestinationRouteResult,
}

/// Command for reading draft experience proposals through the Skill service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillExperienceProposalSnapshotCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    /// Reserved lifecycle filter for future discarded/rejected proposals.
    ///
    /// The first provider only stores accepted draft proposals, but exposing
    /// the filter now keeps future lifecycle states out of shell-owned logic.
    pub include_discarded: bool,
}

/// Read-only snapshot of draft experience proposals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillExperienceProposalSnapshotResult {
    pub proposals: Vec<SkillExperienceProposalRecord>,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposal_record_keeps_only_bounded_reference_metadata() {
        let mut metadata = BTreeMap::new();
        metadata.insert("memory_digest_ref".into(), "memory://digest/1".into());
        metadata.insert("raw_prompt".into(), "do not store this prompt".into());
        metadata.insert("provider_payload".into(), "provider bytes".into());
        metadata.insert("long_ref".into(), "x".repeat(300));

        let record = SkillExperienceProposalRecord::from_candidate(
            &TraceContext::new("trace-proposal-metadata-sanitize"),
            SkillExperienceCandidate {
                task_id: "task-1".into(),
                session_id: Some("session-1".into()),
                application_id: None,
                agent_name: Some("agent".into()),
                verified_terminal_success: true,
                evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
                bounded_summary: "bounded reusable summary".into(),
                trace_digest: Some("trace-digest://task/1".into()),
                memory_digest_refs: vec!["memory://digest/1".into()],
                reusable_procedure: "bounded reusable procedure".into(),
                classification: SkillEvolutionCandidateClassification::ReusableProcedure,
                destination: SkillExperienceCandidateDestination::NewSkillDraft,
                recommended_action: SkillEvolutionProposalAction::CreateDraft,
                target_skill_id: None,
                target_skill_name: Some("governed-skill".into()),
                evidence_ids: vec!["evidence://task/1".into()],
                metadata,
            },
            Utc::now(),
        );

        let serialized =
            serde_json::to_string(&record).expect("proposal record should serialize safely");
        assert_eq!(
            record.metadata.get("memory_digest_ref").map(String::as_str),
            Some("memory://digest/1")
        );
        assert_eq!(
            record.trace_digest.as_deref(),
            Some("trace-digest://task/1")
        );
        assert_eq!(record.memory_digest_refs, vec!["memory://digest/1"]);
        assert_eq!(
            record.destination,
            SkillExperienceCandidateDestination::NewSkillDraft
        );
        assert_eq!(
            record
                .metadata
                .get("long_ref")
                .expect("safe refs are retained")
                .chars()
                .count(),
            256
        );
        assert!(!serialized.contains("do not store this prompt"));
        assert!(!serialized.contains("provider bytes"));
    }
}
