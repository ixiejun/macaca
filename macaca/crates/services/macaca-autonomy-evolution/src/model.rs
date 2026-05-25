//! Provider-neutral DTOs for Autonomy Evolution Control Plane commands.
//!
//! The DTOs store only generic OS identities, bounded evidence references, and
//! sanitized diagnostics. They intentionally avoid raw prompts, provider
//! payloads, manifests, package bytes, or generated Skill bodies so snapshots
//! and audit records can be replayed safely.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

pub const AUTONOMY_EVOLUTION_SERVICE_ID: &str = "service.autonomy_evolution";
pub const AUTONOMY_EVOLUTION_TRANSITION_COMMAND: &str = "autonomy.evolution.transition";
pub const AUTONOMY_EVOLUTION_SNAPSHOT_COMMAND: &str = "autonomy.evolution.snapshot";
pub const AUTONOMY_EVOLUTION_HEALTH_COMMAND: &str = "autonomy.evolution.health";

/// Generic target families that an evolution run may affect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionTargetType {
    SkillPackage,
    ApplicationCapabilityPack,
    TaskPolicy,
    ContextPolicy,
    OsCodeProposal,
    Unsupported(String),
}

/// Stable lifecycle states for an evolution run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionRunState {
    Observed,
    CandidateQueued,
    CandidateClassified,
    ProposalGenerated,
    AdmissionReview,
    Quarantined,
    BenchmarkPrepared,
    BaselineMeasured,
    CandidateMeasured,
    CanaryRunning,
    Promoted,
    ActiveMonitoring,
    Superseded,
    RolledBack,
    Rejected,
    Inconclusive,
}

/// Command-style transition intent accepted by the State machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionRunTransition {
    Observe,
    QueueCandidate,
    ClassifyCandidate,
    GenerateProposal,
    StartAdmissionReview,
    Quarantine,
    PrepareBenchmark,
    RecordBaseline,
    RecordCandidateMeasurement,
    StartCanary,
    Promote,
    StartActiveMonitoring,
    Supersede,
    Rollback,
    Reject,
    MarkInconclusive,
}

/// Provider-neutral scope attached to every command.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionScope {
    pub application_id: Option<String>,
    pub tenant_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
}

/// Transition command emitted by SDK callers or runtime-host adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionTransitionCommand {
    pub run_id: String,
    pub from_state: Option<EvolutionRunState>,
    pub transition: EvolutionRunTransition,
    pub target_type: EvolutionTargetType,
    pub scope: EvolutionScope,
    pub actor_id: String,
    pub trace: TraceContext,
    pub evidence_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub diagnostics: BTreeMap<String, String>,
}

/// Body-free transition result returned by providers and SDK clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionTransitionResult {
    pub run_id: String,
    pub accepted: bool,
    pub previous_state: Option<EvolutionRunState>,
    pub current_state: EvolutionRunState,
    pub transition: EvolutionRunTransition,
    pub target_type: EvolutionTargetType,
    pub trace: TraceContext,
    pub evidence_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub denial_reason: Option<String>,
    pub adapter_dispatch_required: bool,
    pub captured_at: DateTime<Utc>,
}

impl EvolutionTransitionResult {
    /// Build an accepted result after transition validation has passed.
    pub fn accepted(
        command: &EvolutionTransitionCommand,
        previous_state: Option<EvolutionRunState>,
        current_state: EvolutionRunState,
        adapter_dispatch_required: bool,
    ) -> Self {
        Self {
            run_id: command.run_id.clone(),
            accepted: true,
            previous_state,
            current_state,
            transition: command.transition.clone(),
            target_type: command.target_type.clone(),
            trace: command.trace.clone(),
            evidence_refs: command.evidence_refs.clone(),
            policy_decision_refs: command.policy_decision_refs.clone(),
            audit_refs: command.audit_refs.clone(),
            rollback_refs: command.rollback_refs.clone(),
            denial_reason: None,
            adapter_dispatch_required,
            captured_at: Utc::now(),
        }
    }

    /// Build a denied result without mutating the provider read model.
    pub fn denied(
        command: &EvolutionTransitionCommand,
        current_state: EvolutionRunState,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            run_id: command.run_id.clone(),
            accepted: false,
            previous_state: command.from_state.clone(),
            current_state,
            transition: command.transition.clone(),
            target_type: command.target_type.clone(),
            trace: command.trace.clone(),
            evidence_refs: command.evidence_refs.clone(),
            policy_decision_refs: command.policy_decision_refs.clone(),
            audit_refs: command.audit_refs.clone(),
            rollback_refs: command.rollback_refs.clone(),
            denial_reason: Some(reason.into()),
            adapter_dispatch_required: false,
            captured_at: Utc::now(),
        }
    }
}

/// Snapshot command for bounded diagnostic reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionSnapshotCommand {
    pub trace: TraceContext,
    pub scope: EvolutionScope,
    pub run_id: Option<String>,
}

/// Sanitized run record kept in provider snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionRunRecord {
    pub run_id: String,
    pub state: EvolutionRunState,
    pub target_type: EvolutionTargetType,
    pub scope: EvolutionScope,
    pub last_trace_id: String,
    pub evidence_count: usize,
    pub audit_count: usize,
    pub rollback_count: usize,
    pub updated_at: DateTime<Utc>,
}

/// Bounded snapshot used by diagnostics and tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionServiceSnapshot {
    pub service_id: String,
    pub healthy: bool,
    pub unavailable_reason: Option<String>,
    pub records: Vec<EvolutionRunRecord>,
    pub captured_at: DateTime<Utc>,
}

impl EvolutionServiceSnapshot {
    /// Build the structured unavailable snapshot used by Null Object providers.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: AUTONOMY_EVOLUTION_SERVICE_ID.into(),
            healthy: false,
            unavailable_reason: Some(reason.into()),
            records: Vec::new(),
            captured_at: Utc::now(),
        }
    }
}
