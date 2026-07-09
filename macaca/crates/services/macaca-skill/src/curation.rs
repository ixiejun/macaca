//! Skill curation command and report DTOs.
//!
//! The Skill service owns curation semantics, while runtime-host and future
//! Store/EventLog providers choose the Strategy that executes a run.  These
//! DTOs are intentionally metadata-only: reports carry bounded recommendations,
//! refs, counters, and policy ids, never raw prompts, provider payloads, package
//! bytes, manifests, or full `SKILL.md` bodies.

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::governance::{SkillGovernanceRecord, SkillLifecycleState};
use crate::governance_store::{
    SkillCurationRunRecord, SkillGovernanceSnapshotRefRecord, SkillRollbackRefRecord,
};
use crate::semantic_review::SkillSemanticReviewResult;
use crate::service_contract::{SkillServicePolicyHints, SkillServiceScope};

/// Command for reading curation runner readiness without starting a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCurationStatusCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    /// Configured runner interval used to compute the next eligible run.
    ///
    /// The caller supplies the interval because scheduler/runtime policy owns
    /// wake cadence.  The Skill service reports readiness and last-run state
    /// without hardcoding an application or scheduler profile.
    pub interval_ms: u64,
    /// Optional idle/budget window for operator diagnostics.
    pub idle_budget_ms: Option<u64>,
}

/// Read-only curation status returned to SDK and shell adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCurationStatusResult {
    pub provider_id: String,
    pub available: bool,
    pub interval_ms: u64,
    pub idle_budget_ms: Option<u64>,
    pub idle_for_ms: Option<u64>,
    pub last_run_id: Option<String>,
    pub last_run_finished_at: Option<DateTime<Utc>>,
    pub next_eligible_run_at: Option<DateTime<Utc>>,
    pub unavailable_reason: Option<String>,
    pub captured_at: DateTime<Utc>,
}

/// Command for deterministic, non-destructive curation analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCurationDryRunCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub stale_after_days: i64,
    pub narrow_use_threshold: u64,
}

/// Command for a governed curation run.
///
/// Dry-run mode is recommendation-only.  Apply mode is admitted only when
/// policy and approval refs are present; this first run slice records the run
/// decision and report refs but does not yet perform lifecycle, alias, package,
/// scheduler, or context mutations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCurationRunCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub dry_run: bool,
    pub stale_after_days: i64,
    pub narrow_use_threshold: u64,
    #[serde(default)]
    pub approval_refs: Vec<String>,
    #[serde(default)]
    pub policy_decision_refs: Vec<String>,
    #[serde(default)]
    pub audit_event_ids: Vec<String>,
    #[serde(default)]
    pub policy: SkillServicePolicyHints,
}

impl SkillCurationRunCommand {
    /// Validate trace, thresholds, and approval evidence before a provider run.
    pub fn validate(&self) -> Result<(), String> {
        if self.trace.trace_id.trim().is_empty() {
            return Err("skill curation run requires trace_id".into());
        }
        if self.stale_after_days < 0 {
            return Err("skill curation run requires a non-negative stale threshold".into());
        }
        if !self.dry_run
            && (self.approval_refs.iter().all(|id| id.trim().is_empty())
                || self
                    .policy_decision_refs
                    .iter()
                    .all(|id| id.trim().is_empty()))
        {
            return Err(
                "skill curation apply run requires approval refs and policy decision refs".into(),
            );
        }
        // Fail-closed readiness (2026-07-08 audit P0-3): a non-dry-run curation
        // mutates skill state, so it proceeds only when both entitlement and
        // package readiness are explicitly confirmed. This aligns curation apply
        // with the canonical `mutation.rs` / `proposal_materialization.rs` gate
        // and with proposal_lifecycle/processing, which previously diverged.
        if !self.dry_run
            && (self.policy.entitlement_ready != Some(true)
                || self.policy.package_ready != Some(true))
        {
            tracing::warn!(
                target = "macaca_skill::curation",
                event = "curation_apply_denied",
                reason_code = "readiness_not_confirmed"
            );
            return Err("skill curation apply run requires confirmed entitlement and package readiness".into());
        }
        Ok(())
    }
}

/// Command for recording and returning a curation governance snapshot ref.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCurationSnapshotCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub include_archived: bool,
    #[serde(default)]
    pub lifecycle_filters: Vec<SkillLifecycleState>,
    /// Include rollback memento refs when the provider has them.
    ///
    /// The response carries refs only.  Package bytes and full skill bodies are
    /// never embedded in the DTO.
    pub include_package_mementos: bool,
}

/// Result for a curation governance snapshot command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillCurationSnapshotResult {
    pub snapshot: SkillGovernanceSnapshotRefRecord,
    pub curation_run_refs: Vec<String>,
    pub rollback_refs: Vec<SkillRollbackRefRecord>,
    pub package_memento_refs: Vec<String>,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

/// Command for restoring governance state from a curation rollback memento.
///
/// Rollback is a privileged mutation because it rewinds lifecycle, telemetry,
/// aliases, and report/package refs to a known pre-apply state.  The command
/// therefore requires trace, approval, and policy decision refs before a
/// provider can restore any local or Store/EventLog-backed memento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCurationRollbackCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub rollback_ref: String,
    #[serde(default)]
    pub approval_refs: Vec<String>,
    #[serde(default)]
    pub policy_decision_refs: Vec<String>,
    #[serde(default)]
    pub audit_event_ids: Vec<String>,
    #[serde(default)]
    pub policy: SkillServicePolicyHints,
}

impl SkillCurationRollbackCommand {
    /// Validate rollback admission before restoring a memento.
    pub fn validate(&self) -> Result<(), String> {
        if self.trace.trace_id.trim().is_empty() {
            return Err("skill curation rollback requires trace_id".into());
        }
        if self.rollback_ref.trim().is_empty() {
            return Err("skill curation rollback requires rollback_ref".into());
        }
        if self.approval_refs.iter().all(|id| id.trim().is_empty()) {
            return Err("skill curation rollback requires approval refs".into());
        }
        if self
            .policy_decision_refs
            .iter()
            .all(|id| id.trim().is_empty())
        {
            return Err("skill curation rollback requires policy decision refs".into());
        }
        Ok(())
    }
}

/// Result returned after a provider restores a curation rollback memento.
///
/// The response carries counts and refs only.  It never echoes skill package
/// bytes, full instruction bodies, raw provider payloads, prompts, secrets, or
/// other sensitive artifacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillCurationRollbackResult {
    pub rollback_ref: String,
    pub before_snapshot_ref: String,
    pub after_snapshot_ref: Option<String>,
    pub restored_record_count: u64,
    pub restored_alias_count: u64,
    pub restored_curation_run_count: u64,
    pub restored_rollback_ref_count: u64,
    pub restored_report_refs: Vec<String>,
    pub package_memento_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_event_ids: Vec<String>,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

/// Recommendation action names are intentionally non-destructive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCurationAction {
    Keep,
    Protected,
    WouldQuarantine,
    WouldReduceSize,
    WouldFixMetadata,
    WouldResolveMissingDependency,
    WouldMarkStale,
    WouldArchive,
    WouldReviewForConsolidation,
}

/// Deterministic curation phase that produced a recommendation.
///
/// A recommendation can carry multiple phases because diagnostics are
/// independent.  For example, a skill may be both oversized and missing a
/// dependency.  The action remains the highest-priority non-destructive plan,
/// while phases keep the report audit-friendly without shell-side inference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCurationPhase {
    Keep,
    Protected,
    Quarantine,
    Size,
    InvalidMetadata,
    MissingDependency,
    Stale,
    Archive,
    Consolidation,
}

impl Default for SkillCurationPhase {
    fn default() -> Self {
        Self::Keep
    }
}

/// One deterministic recommendation in a curation report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillCurationRecommendation {
    pub skill_id: String,
    pub name: String,
    pub action: SkillCurationAction,
    #[serde(default)]
    pub phases: Vec<SkillCurationPhase>,
    pub rationale: String,
    pub protected: bool,
    pub confidence: f32,
    pub evidence_ids: Vec<String>,
}

/// Dry-run curation report produced without mutating files or state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillCurationDryRunResult {
    pub recommendations: Vec<SkillCurationRecommendation>,
    pub semantic_analysis_status: String,
    #[serde(default = "SkillSemanticReviewResult::unavailable_default")]
    pub semantic_review: SkillSemanticReviewResult,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

impl SkillCurationDryRunResult {
    /// Build deterministic recommendations from governance records.
    pub fn from_records(
        records: impl IntoIterator<Item = SkillGovernanceRecord>,
        command: &SkillCurationDryRunCommand,
        now: DateTime<Utc>,
    ) -> Self {
        crate::curation_policy::deterministic_report(
            records,
            command.stale_after_days,
            command.narrow_use_threshold,
            now,
        )
    }
}

/// Result of a traced curation run command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillCurationRunResult {
    pub run: SkillCurationRunRecord,
    pub recommendations: Vec<SkillCurationRecommendation>,
    pub semantic_analysis_status: String,
    #[serde(default = "SkillSemanticReviewResult::unavailable_default")]
    pub semantic_review: SkillSemanticReviewResult,
    pub run_json_ref: Option<String>,
    pub report_ref: Option<String>,
    pub rollback_ref: Option<String>,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

impl SkillCurationRunResult {
    /// Build a bounded run result from sanitized governance records.
    pub fn from_records(
        records: impl IntoIterator<Item = SkillGovernanceRecord>,
        command: &SkillCurationRunCommand,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
    ) -> Self {
        let dry_run_report = crate::curation_policy::deterministic_report(
            records,
            command.stale_after_days,
            command.narrow_use_threshold,
            finished_at,
        );
        let action_refs = dry_run_report
            .recommendations
            .iter()
            .map(|recommendation| {
                format!("{}:{:?}", recommendation.skill_id, recommendation.action)
            })
            .collect::<Vec<_>>();
        let run_id = run_id(&command.trace, started_at);
        let run_json_ref = Some(format!("store://skill-curation/{run_id}/run.json"));
        let report_ref = Some(format!("store://skill-curation/{run_id}/REPORT.md"));
        let run = SkillCurationRunRecord {
            run_id,
            trace_id: command.trace.trace_id.clone(),
            provider_id: "local-skill-governance".into(),
            dry_run: command.dry_run,
            candidate_count: dry_run_report.recommendations.len() as u64,
            actions: action_refs,
            snapshot_refs: Vec::new(),
            started_at,
            finished_at: Some(finished_at),
            run_json_ref: run_json_ref.clone(),
            report_ref: report_ref.clone(),
            rollback_ref: None,
            policy_decision_ids: non_empty(&command.policy_decision_refs),
            audit_event_ids: non_empty(&command.audit_event_ids),
        };

        Self {
            run,
            recommendations: dry_run_report.recommendations,
            semantic_analysis_status: dry_run_report.semantic_analysis_status,
            semantic_review: dry_run_report.semantic_review,
            run_json_ref,
            report_ref,
            rollback_ref: None,
            mutated: !command.dry_run,
            captured_at: finished_at,
        }
    }
}

fn run_id(trace: &TraceContext, started_at: DateTime<Utc>) -> String {
    let nanos = started_at
        .timestamp_nanos_opt()
        .unwrap_or_else(|| started_at.timestamp_micros() * 1_000);
    format!("skill-curation-run-{}-{nanos}", trace.trace_id)
}

fn non_empty(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect()
}
