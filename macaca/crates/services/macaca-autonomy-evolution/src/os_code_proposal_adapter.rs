//! Non-mutating OS-code evolution proposal adapter.
//!
//! This module is a target-adapter Strategy for future Macaca source-code
//! evolution. It deliberately stops at governed proposal metadata: OpenSpec,
//! Superpowers, GitNexus, expected tests, release-gate refs, and rollback refs.
//! It never writes files, executes commands, applies patches, or commits code.

use chrono::{DateTime, Utc};
use macaca_proto::{MacacaError, MacacaResult, TraceContext};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{EvolutionScope, AUTONOMY_EVOLUTION_SERVICE_ID};

const MAX_REF_LEN: usize = 160;
const MAX_REFS: usize = 8;
const MAX_TEXT_LEN: usize = 240;
const BLAST_RADIUS_QUARANTINE_THRESHOLD: u32 = 80;

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

/// Replaceable Strategy for OS-code evolution proposal generation.
pub trait OsCodeEvolutionProposalAdapter: Send + Sync {
    fn evaluate(
        &self,
        command: OsCodeEvolutionProposalCommand,
    ) -> MacacaResult<OsCodeEvolutionProposalResult>;
}

/// Default non-mutating adapter used by the built-in evolution service.
#[derive(Debug, Default, Clone)]
pub struct DefaultOsCodeEvolutionProposalAdapter;

impl OsCodeEvolutionProposalAdapter for DefaultOsCodeEvolutionProposalAdapter {
    fn evaluate(
        &self,
        command: OsCodeEvolutionProposalCommand,
    ) -> MacacaResult<OsCodeEvolutionProposalResult> {
        validate_command_shape(&command)?;
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            proposal_id = command.proposal_id.as_str(),
            run_id = command.run_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "os-code evolution proposal adapter evaluation started"
        );

        let findings = gate_findings(&command);
        let missing_evidence = findings
            .iter()
            .filter(|finding| !finding.accepted && finding.reason_code.starts_with("missing_"))
            .map(|finding| finding.gate.clone())
            .collect::<Vec<_>>();
        let decision = decision_for(&command, &findings, &missing_evidence);
        if decision != OsCodeEvolutionProposalDecision::ReadyForReview {
            warn!(
                service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
                proposal_id = command.proposal_id.as_str(),
                decision = ?decision,
                "os-code evolution proposal adapter did not mark proposal ready"
            );
        }

        Ok(result(command, findings, missing_evidence, decision))
    }
}

fn validate_command_shape(command: &OsCodeEvolutionProposalCommand) -> MacacaResult<()> {
    if command.proposal_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "os-code proposal requires proposal_id".into(),
        ));
    }
    if command.run_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "os-code proposal requires run_id".into(),
        ));
    }
    if command.actor_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "os-code proposal requires actor_id".into(),
        ));
    }
    if command.trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "os-code proposal requires trace_id".into(),
        ));
    }
    if command.policy_decision_refs.is_empty() {
        return Err(MacacaError::Config(
            "os-code proposal requires policy decision refs".into(),
        ));
    }
    if command.audit_refs.is_empty() {
        return Err(MacacaError::Config(
            "os-code proposal requires audit refs".into(),
        ));
    }
    Ok(())
}

fn gate_findings(command: &OsCodeEvolutionProposalCommand) -> Vec<OsCodeEvolutionProposalFinding> {
    vec![
        refs_gate(
            "openspec_proposal_refs",
            "openspec_proposal_refs_present",
            "missing_openspec_proposal_refs",
            &command.evidence.openspec_proposal_refs,
        ),
        refs_gate(
            "openspec_design_refs",
            "openspec_design_refs_present",
            "missing_openspec_design_refs",
            &command.evidence.openspec_design_refs,
        ),
        refs_gate(
            "openspec_tasks_refs",
            "openspec_tasks_refs_present",
            "missing_openspec_tasks_refs",
            &command.evidence.openspec_tasks_refs,
        ),
        refs_gate(
            "superpowers_design_refs",
            "superpowers_design_refs_present",
            "missing_superpowers_design_refs",
            &command.evidence.superpowers_design_refs,
        ),
        refs_gate(
            "superpowers_plan_refs",
            "superpowers_plan_refs_present",
            "missing_superpowers_plan_refs",
            &command.evidence.superpowers_plan_refs,
        ),
        refs_gate(
            "gitnexus_impact_refs",
            "gitnexus_impact_refs_present",
            "missing_gitnexus_impact_refs",
            &command.evidence.gitnexus_impact_refs,
        ),
        refs_gate(
            "expected_test_refs",
            "expected_test_refs_present",
            "missing_expected_test_refs",
            &command.evidence.expected_test_refs,
        ),
        refs_gate(
            "release_gate_refs",
            "release_gate_refs_present",
            "missing_release_gate_refs",
            &command.evidence.release_gate_refs,
        ),
        refs_gate(
            "rollback_refs",
            "rollback_refs_present",
            "missing_rollback_refs",
            &command.evidence.rollback_refs,
        ),
        mutation_gate(command),
        blast_radius_gate(command),
    ]
}

fn refs_gate(
    gate: &str,
    accepted_code: &str,
    missing_code: &str,
    refs: &[String],
) -> OsCodeEvolutionProposalFinding {
    if refs.is_empty() {
        finding(gate, false, missing_code, Vec::new())
    } else {
        finding(gate, true, accepted_code, bounded_refs(refs))
    }
}

fn mutation_gate(command: &OsCodeEvolutionProposalCommand) -> OsCodeEvolutionProposalFinding {
    finding(
        "source_mutation",
        !command.intent.allow_source_mutation,
        if command.intent.allow_source_mutation {
            "source_mutation_not_allowed"
        } else {
            "source_mutation_blocked"
        },
        Vec::new(),
    )
}

fn blast_radius_gate(command: &OsCodeEvolutionProposalCommand) -> OsCodeEvolutionProposalFinding {
    let accepted = command.intent.blast_radius_score <= BLAST_RADIUS_QUARANTINE_THRESHOLD;
    finding(
        "blast_radius",
        accepted,
        if accepted {
            "blast_radius_within_review_threshold"
        } else {
            "blast_radius_requires_quarantine"
        },
        Vec::new(),
    )
}

fn finding(
    gate: &str,
    accepted: bool,
    reason_code: &str,
    evidence_refs: Vec<String>,
) -> OsCodeEvolutionProposalFinding {
    OsCodeEvolutionProposalFinding {
        gate: gate.into(),
        accepted,
        reason_code: reason_code.into(),
        evidence_refs,
    }
}

fn decision_for(
    command: &OsCodeEvolutionProposalCommand,
    findings: &[OsCodeEvolutionProposalFinding],
    missing_evidence: &[String],
) -> OsCodeEvolutionProposalDecision {
    if command.intent.allow_source_mutation {
        return OsCodeEvolutionProposalDecision::Denied;
    }
    if !missing_evidence.is_empty() {
        return OsCodeEvolutionProposalDecision::NeedsEvidence;
    }
    if findings
        .iter()
        .any(|finding| finding.reason_code == "blast_radius_requires_quarantine")
    {
        return OsCodeEvolutionProposalDecision::Quarantined;
    }
    OsCodeEvolutionProposalDecision::ReadyForReview
}

fn result(
    command: OsCodeEvolutionProposalCommand,
    findings: Vec<OsCodeEvolutionProposalFinding>,
    missing_evidence: Vec<String>,
    decision: OsCodeEvolutionProposalDecision,
) -> OsCodeEvolutionProposalResult {
    let mut reason_codes = findings
        .iter()
        .filter(|finding| !finding.accepted)
        .map(|finding| finding.reason_code.clone())
        .collect::<Vec<_>>();
    if decision == OsCodeEvolutionProposalDecision::ReadyForReview {
        reason_codes.push("proposal_ready_for_review".into());
    }
    info!(
        service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
        proposal_id = command.proposal_id.as_str(),
        run_id = command.run_id.as_str(),
        decision = ?decision,
        "os-code evolution proposal adapter evaluation completed"
    );
    OsCodeEvolutionProposalResult {
        proposal_id: sanitize_ref(&command.proposal_id),
        run_id: sanitize_ref(&command.run_id),
        decision,
        trace: command.trace.clone(),
        bundle: bundle(&command),
        findings,
        missing_evidence,
        reason_codes,
        source_mutation_performed: false,
        policy_decision_refs: bounded_refs(&command.policy_decision_refs),
        audit_refs: bounded_refs(&command.audit_refs),
        captured_at: Utc::now(),
    }
}

fn bundle(command: &OsCodeEvolutionProposalCommand) -> OsCodeEvolutionProposalBundle {
    OsCodeEvolutionProposalBundle {
        proposal_ref: Some(sanitize_ref(&command.proposal_id)),
        title: sanitize_text(&command.intent.title),
        summary: sanitize_text(&command.intent.summary),
        affected_capability_refs: bounded_refs(&command.intent.affected_capability_refs),
        requested_change_refs: bounded_refs(&command.intent.requested_change_refs),
        openspec_refs: bounded_refs(
            &command
                .evidence
                .openspec_proposal_refs
                .iter()
                .chain(command.evidence.openspec_design_refs.iter())
                .chain(command.evidence.openspec_tasks_refs.iter())
                .cloned()
                .collect::<Vec<_>>(),
        ),
        superpowers_refs: bounded_refs(
            &command
                .evidence
                .superpowers_design_refs
                .iter()
                .chain(command.evidence.superpowers_plan_refs.iter())
                .cloned()
                .collect::<Vec<_>>(),
        ),
        gitnexus_refs: bounded_refs(&command.evidence.gitnexus_impact_refs),
        expected_test_refs: bounded_refs(&command.evidence.expected_test_refs),
        release_gate_refs: bounded_refs(&command.evidence.release_gate_refs),
        rollback_refs: bounded_refs(&command.evidence.rollback_refs),
    }
}

fn bounded_refs(values: &[String]) -> Vec<String> {
    values
        .iter()
        .take(MAX_REFS)
        .map(|value| sanitize_ref(value))
        .collect()
}

fn sanitize_text(value: &str) -> String {
    let mut sanitized = sanitize_ref(value);
    if sanitized.len() > MAX_TEXT_LEN {
        sanitized.truncate(MAX_TEXT_LEN);
    }
    sanitized
}

fn sanitize_ref(value: &str) -> String {
    let mut sanitized = value
        .replace("raw_prompt", "sanitized")
        .replace("raw_provider_payload", "sanitized")
        .replace("credential", "redacted")
        .replace("secret", "redacted")
        .replace("private_key", "redacted")
        .replace("raw_signature", "redacted");
    if sanitized.len() > MAX_REF_LEN {
        sanitized.truncate(MAX_REF_LEN);
    }
    sanitized
}
