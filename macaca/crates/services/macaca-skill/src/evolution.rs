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
            metadata: candidate.metadata,
        }
    }
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
