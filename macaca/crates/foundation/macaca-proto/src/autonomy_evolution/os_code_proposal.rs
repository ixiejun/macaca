//! Provider-neutral DTOs for governed OS-code evolution proposals.
//!
//! The contract intentionally carries only bounded metadata and evidence
//! references. Service providers may evaluate these DTOs, but `macaca-proto`
//! must not own adapter strategies, source mutation, filesystem writes, or
//! command execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::TraceContext;

use super::EvolutionScope;

/// Bounded description of the source-code change being proposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsCodeEvolutionProposalIntent {
    pub title: String,
    pub summary: String,
    pub affected_capability_refs: Vec<String>,
    pub requested_change_refs: Vec<String>,
    pub allow_source_mutation: bool,
    pub blast_radius_score: u32,
}

/// Mandatory governance evidence for a non-mutating OS-code proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsCodeEvolutionProposalEvidence {
    pub openspec_proposal_refs: Vec<String>,
    pub openspec_design_refs: Vec<String>,
    pub openspec_tasks_refs: Vec<String>,
    pub superpowers_design_refs: Vec<String>,
    pub superpowers_plan_refs: Vec<String>,
    pub gitnexus_impact_refs: Vec<String>,
    pub expected_test_refs: Vec<String>,
    pub release_gate_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
}

/// Typed command for building an OS-code proposal bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsCodeEvolutionProposalCommand {
    pub proposal_id: String,
    pub run_id: String,
    pub actor_id: String,
    pub trace: TraceContext,
    pub scope: EvolutionScope,
    pub intent: OsCodeEvolutionProposalIntent,
    pub evidence: OsCodeEvolutionProposalEvidence,
    pub policy_decision_refs: Vec<String>,
    pub audit_refs: Vec<String>,
}

/// Conservative adapter decision for OS-code proposals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OsCodeEvolutionProposalDecision {
    ReadyForReview,
    NeedsEvidence,
    Quarantined,
    Denied,
}

/// Sanitized proposal bundle that can be recorded in the governance ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsCodeEvolutionProposalBundle {
    pub proposal_ref: Option<String>,
    pub title: String,
    pub summary: String,
    pub affected_capability_refs: Vec<String>,
    pub requested_change_refs: Vec<String>,
    pub openspec_refs: Vec<String>,
    pub superpowers_refs: Vec<String>,
    pub gitnexus_refs: Vec<String>,
    pub expected_test_refs: Vec<String>,
    pub release_gate_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
}

/// Structured finding for one adapter gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsCodeEvolutionProposalFinding {
    pub gate: String,
    pub accepted: bool,
    pub reason_code: String,
    pub evidence_refs: Vec<String>,
}

/// Result emitted by the non-mutating adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsCodeEvolutionProposalResult {
    pub proposal_id: String,
    pub run_id: String,
    pub decision: OsCodeEvolutionProposalDecision,
    pub trace: TraceContext,
    pub bundle: OsCodeEvolutionProposalBundle,
    pub findings: Vec<OsCodeEvolutionProposalFinding>,
    pub missing_evidence: Vec<String>,
    pub reason_codes: Vec<String>,
    pub source_mutation_performed: bool,
    pub policy_decision_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub captured_at: DateTime<Utc>,
}
