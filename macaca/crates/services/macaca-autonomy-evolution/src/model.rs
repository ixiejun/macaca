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
pub const AUTONOMY_EVOLUTION_ADMISSION_COMMAND: &str = "autonomy.evolution.admit_candidate";
pub const AUTONOMY_EVOLUTION_BENCHMARK_COMMAND: &str = "autonomy.evolution.benchmark.paired";
pub const AUTONOMY_EVOLUTION_OS_CODE_PROPOSAL_COMMAND: &str =
    "autonomy.evolution.os_code.proposal.evaluate";
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

/// Final admission decision emitted by executable quality gates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdmissionDecision {
    Accepted,
    Denied,
    NeedsEvidence,
    Quarantined,
}

/// Metadata-only candidate shape used by admission Specifications.
///
/// This structure intentionally carries bounded summaries and evidence
/// references instead of package bytes or raw Skill bodies. The admission
/// service judges whether later target adapters may continue, but it does not
/// own Skill file mutation or application-specific workflow logic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionAdmissionCandidate {
    pub candidate_id: String,
    pub target_type: EvolutionTargetType,
    pub package_name: String,
    pub trigger_descriptions: Vec<String>,
    pub skill_summary: String,
    pub resource_categories: Vec<String>,
    pub quick_validation_refs: Vec<String>,
    pub forward_test_refs: Vec<String>,
    pub duplicate_candidate_refs: Vec<String>,
    pub metadata_stale: bool,
}

/// Command for evaluating one candidate through service-owned admission gates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionAdmissionCommand {
    pub actor_id: String,
    pub trace: TraceContext,
    pub scope: EvolutionScope,
    pub candidate: EvolutionAdmissionCandidate,
    pub evidence_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
}

/// Bounded diagnostic for one executable admission gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionAdmissionFinding {
    pub gate: String,
    pub decision: AdmissionDecision,
    pub reason_code: String,
    pub evidence_refs: Vec<String>,
}

/// Result returned by admission providers and SDK clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionAdmissionResult {
    pub candidate_id: String,
    pub target_type: EvolutionTargetType,
    pub decision: AdmissionDecision,
    pub trace: TraceContext,
    pub findings: Vec<EvolutionAdmissionFinding>,
    pub missing_evidence: Vec<String>,
    pub summary_reason: Option<String>,
    pub policy_decision_refs: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl EvolutionAdmissionResult {
    /// Build a fail-closed result when the admission service is absent.
    pub fn unavailable(command: &EvolutionAdmissionCommand, reason: impl Into<String>) -> Self {
        Self {
            candidate_id: command.candidate.candidate_id.clone(),
            target_type: command.candidate.target_type.clone(),
            decision: AdmissionDecision::Denied,
            trace: command.trace.clone(),
            findings: Vec::new(),
            missing_evidence: Vec::new(),
            summary_reason: Some(reason.into()),
            policy_decision_refs: command.policy_decision_refs.clone(),
            captured_at: Utc::now(),
        }
    }
}

/// Final decision emitted by normalized paired benchmark scoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BenchmarkDecision {
    Passed,
    Failed,
    Inconclusive,
}

/// Standard metric schema used for baseline/candidate comparisons.
///
/// All fields are normalized counters or scores. The service receives these
/// values from evidence collectors; it does not execute workloads, inspect raw
/// artifacts, or infer domain-specific quality from application content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionBenchmarkMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub elapsed_ms: u64,
    pub tool_calls: u32,
    pub tool_results: u32,
    pub retry_count: u32,
    pub failure_recovery_count: u32,
    pub quality_score: u32,
    pub human_intervention_count: u32,
    pub policy_decision_count: u32,
    pub activation_count: u32,
    pub use_count: u32,
    pub success_count: u32,
}

/// One side of a paired benchmark comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionBenchmarkMeasurement {
    pub measurement_id: String,
    pub task_family_id: String,
    pub target_type: EvolutionTargetType,
    pub metrics: Option<EvolutionBenchmarkMetrics>,
    pub evidence_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub regression_reason_codes: Vec<String>,
}

/// Typed command for normalized baseline-versus-candidate scoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionBenchmarkCommand {
    pub benchmark_id: String,
    pub run_id: String,
    pub actor_id: String,
    pub trace: TraceContext,
    pub scope: EvolutionScope,
    pub baseline: EvolutionBenchmarkMeasurement,
    pub candidate: EvolutionBenchmarkMeasurement,
    pub policy_decision_refs: Vec<String>,
}

/// Bounded numeric delta snapshot returned by the default scoring Strategy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionBenchmarkScoreDelta {
    pub total_token_delta: i64,
    pub elapsed_ms_delta: i64,
    pub tool_call_delta: i64,
    pub retry_delta: i64,
    pub quality_delta: i64,
    pub human_intervention_delta: i64,
    pub success_count_delta: i64,
    pub efficiency_improved: bool,
    pub quality_preserved: bool,
}

/// Result returned by benchmark providers and SDK clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionBenchmarkResult {
    pub benchmark_id: String,
    pub run_id: String,
    pub task_family_id: String,
    pub target_type: EvolutionTargetType,
    pub decision: BenchmarkDecision,
    pub trace: TraceContext,
    pub score_delta: EvolutionBenchmarkScoreDelta,
    pub reason_codes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl EvolutionBenchmarkResult {
    /// Build a fail-closed benchmark result when the service is absent.
    pub fn unavailable(command: &EvolutionBenchmarkCommand, reason: impl Into<String>) -> Self {
        Self {
            benchmark_id: command.benchmark_id.clone(),
            run_id: command.run_id.clone(),
            task_family_id: command.baseline.task_family_id.clone(),
            target_type: command.candidate.target_type.clone(),
            decision: BenchmarkDecision::Inconclusive,
            trace: command.trace.clone(),
            score_delta: EvolutionBenchmarkScoreDelta::default(),
            reason_codes: vec![reason.into()],
            evidence_refs: bounded_pair_refs(
                &command.baseline.evidence_refs,
                &command.candidate.evidence_refs,
            ),
            artifact_refs: bounded_pair_refs(
                &command.baseline.artifact_refs,
                &command.candidate.artifact_refs,
            ),
            policy_decision_refs: command.policy_decision_refs.clone(),
            captured_at: Utc::now(),
        }
    }
}

fn bounded_pair_refs(left: &[String], right: &[String]) -> Vec<String> {
    left.iter()
        .chain(right.iter())
        .take(8)
        .map(|value| {
            if value.len() > 160 {
                let mut bounded = value.clone();
                bounded.truncate(160);
                bounded
            } else {
                value.clone()
            }
        })
        .collect()
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
