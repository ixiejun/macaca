//! Provider-neutral DTOs for the live autonomy evolution orchestrator.
//!
//! The live orchestrator is intentionally a thin coordination surface. These
//! types carry leases, idempotency keys, bounded evidence references, policy
//! references, and phase checkpoints so providers can resume and audit the loop
//! without storing raw prompts, provider payloads, manifests, package bytes, or
//! application-specific workflow data.

use crate::TraceContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    EvolutionAdmissionCandidate, EvolutionBenchmarkMeasurement, EvolutionReleaseAction,
    EvolutionReleasePolicyInput, EvolutionScope, EvolutionTargetType,
    AUTONOMY_EVOLUTION_SERVICE_ID,
};

pub const AUTONOMY_EVOLUTION_LIVE_TICK_COMMAND: &str = "autonomy.evolution.live.tick";
pub const AUTONOMY_EVOLUTION_LIVE_AUDIT_COMMAND: &str = "autonomy.evolution.live.audit";

const MAX_REF_LEN: usize = 160;
const MAX_REFS: usize = 8;

/// Coarse phase labels used by live tick checkpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionLivePhase {
    Discovery,
    Transition,
    Admission,
    Benchmark,
    ReleaseSafety,
    TargetDispatch,
    AuditLedger,
}

/// Final status for a live tick or one phase checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionLivePhaseStatus {
    Accepted,
    Denied,
    Unavailable,
    Inconclusive,
    Rejected,
    CanaryRunning,
    Promoted,
    RolledBack,
}

/// Bounded target adapter dispatch input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionLiveAdapterDispatch {
    pub adapter_id: String,
    pub supported: bool,
    pub evidence_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
}

/// One phase checkpoint emitted by the live orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionLivePhaseCheckpoint {
    pub phase: EvolutionLivePhase,
    pub status: EvolutionLivePhaseStatus,
    pub accepted: bool,
    pub reason_codes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl EvolutionLivePhaseCheckpoint {
    /// Build a sanitized checkpoint for one orchestration phase.
    pub fn new(
        phase: EvolutionLivePhase,
        status: EvolutionLivePhaseStatus,
        accepted: bool,
        reason_codes: Vec<String>,
        evidence_refs: Vec<String>,
    ) -> Self {
        Self {
            phase,
            status,
            accepted,
            reason_codes: bounded_values(&reason_codes),
            evidence_refs: bounded_values(&evidence_refs),
            captured_at: Utc::now(),
        }
    }
}

/// Command for one bounded live orchestration tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionLiveTickCommand {
    pub run_id: String,
    pub actor_id: String,
    pub lease_id: String,
    pub idempotency_key: String,
    pub trace: TraceContext,
    pub scope: EvolutionScope,
    pub candidate: EvolutionAdmissionCandidate,
    pub baseline: EvolutionBenchmarkMeasurement,
    pub candidate_measurement: EvolutionBenchmarkMeasurement,
    pub release_action: EvolutionReleaseAction,
    pub release_policy: EvolutionReleasePolicyInput,
    pub adapter_dispatch: EvolutionLiveAdapterDispatch,
    pub observer_evidence_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub replay_after_sequence: Option<u64>,
}

/// Result returned by a live orchestration tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionLiveTickResult {
    pub service_id: String,
    pub run_id: String,
    pub target_type: EvolutionTargetType,
    pub idempotency_key: String,
    pub lease_id: String,
    pub sequence: u64,
    pub accepted: bool,
    pub status: EvolutionLivePhaseStatus,
    pub trace: TraceContext,
    pub phases: Vec<EvolutionLivePhaseCheckpoint>,
    pub reason_codes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub adapter_dispatch_performed: bool,
    pub source_mutation_performed: bool,
    pub captured_at: DateTime<Utc>,
}

impl EvolutionLiveTickResult {
    /// Build a fail-closed result before any side effect is dispatched.
    pub fn denied(
        command: &EvolutionLiveTickCommand,
        sequence: u64,
        status: EvolutionLivePhaseStatus,
        reason: impl Into<String>,
    ) -> Self {
        let reason = reason.into();
        Self {
            service_id: AUTONOMY_EVOLUTION_SERVICE_ID.into(),
            run_id: command.run_id.clone(),
            target_type: command.candidate.target_type.clone(),
            idempotency_key: command.idempotency_key.clone(),
            lease_id: command.lease_id.clone(),
            sequence,
            accepted: false,
            status: status.clone(),
            trace: command.trace.clone(),
            phases: vec![EvolutionLivePhaseCheckpoint::new(
                EvolutionLivePhase::Discovery,
                status,
                false,
                vec![reason.clone()],
                command.observer_evidence_refs.clone(),
            )],
            reason_codes: vec![bounded_value(reason)],
            evidence_refs: bounded_values(&command.observer_evidence_refs),
            policy_decision_refs: bounded_values(&command.policy_decision_refs),
            audit_refs: bounded_values(&command.audit_refs),
            rollback_refs: Vec::new(),
            adapter_dispatch_performed: false,
            source_mutation_performed: false,
            captured_at: Utc::now(),
        }
    }

    /// Build a fail-closed unavailable result for absent providers.
    pub fn unavailable(command: &EvolutionLiveTickCommand, reason: impl Into<String>) -> Self {
        Self::denied(command, 0, EvolutionLivePhaseStatus::Unavailable, reason)
    }
}

/// Audit command for replaying live orchestrator checkpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionLiveAuditCommand {
    pub run_id: String,
    pub trace: TraceContext,
    pub scope: EvolutionScope,
    pub after_sequence: Option<u64>,
    pub limit: usize,
}

/// Audit result reconstructed from live checkpoints or ledger-backed records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionLiveAuditResult {
    pub service_id: String,
    pub run_id: String,
    pub trace: TraceContext,
    pub checkpoints: Vec<EvolutionLiveTickResult>,
    pub next_after_sequence: Option<u64>,
    pub diagnostics: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl EvolutionLiveAuditResult {
    /// Build a structured unavailable audit result when the service is absent.
    pub fn unavailable(command: &EvolutionLiveAuditCommand, reason: impl Into<String>) -> Self {
        Self {
            service_id: AUTONOMY_EVOLUTION_SERVICE_ID.into(),
            run_id: command.run_id.clone(),
            trace: command.trace.clone(),
            checkpoints: Vec::new(),
            next_after_sequence: command.after_sequence,
            diagnostics: vec![bounded_value(reason.into())],
            captured_at: Utc::now(),
        }
    }
}

pub(crate) fn bounded_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .take(MAX_REFS)
        .map(|value| bounded_value(value.clone()))
        .collect()
}

pub(crate) fn bounded_value(value: String) -> String {
    if value.len() > MAX_REF_LEN {
        let mut bounded = value;
        bounded.truncate(MAX_REF_LEN);
        bounded
    } else {
        value
    }
}
