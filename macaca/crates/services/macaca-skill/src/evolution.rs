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

/// Sanitized task evidence that may become a governed skill proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillExperienceCandidate {
    pub task_id: String,
    pub session_id: Option<String>,
    pub application_id: Option<ApplicationId>,
    pub agent_name: Option<String>,
    pub bounded_summary: String,
    pub reusable_procedure: String,
    pub classification: SkillEvolutionCandidateClassification,
    pub recommended_action: SkillEvolutionProposalAction,
    pub target_skill_id: Option<String>,
    pub target_skill_name: Option<String>,
    pub evidence_ids: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl SkillExperienceCandidate {
    /// Validate the minimum evidence required before a proposal can exist.
    pub fn validate(&self) -> Result<(), String> {
        if self.task_id.trim().is_empty() {
            return Err("skill experience proposal requires task_id".into());
        }
        if self.bounded_summary.trim().is_empty() {
            return Err("skill experience proposal requires bounded summary".into());
        }
        if self.reusable_procedure.trim().is_empty() {
            return Err("skill experience proposal requires reusable procedure".into());
        }
        if self.evidence_ids.iter().all(|id| id.trim().is_empty()) {
            return Err("skill experience proposal requires evidence references".into());
        }
        Ok(())
    }
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
    pub reusable_procedure: String,
    pub classification: SkillEvolutionCandidateClassification,
    pub recommended_action: SkillEvolutionProposalAction,
    pub target_skill_id: Option<String>,
    pub target_skill_name: Option<String>,
    pub evidence_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
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
            reusable_procedure: candidate.reusable_procedure,
            classification: candidate.classification,
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
                bounded_summary: "bounded reusable summary".into(),
                reusable_procedure: "bounded reusable procedure".into(),
                classification: SkillEvolutionCandidateClassification::ReusableProcedure,
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
