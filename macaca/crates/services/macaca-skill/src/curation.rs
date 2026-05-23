//! Skill curation command and report DTOs.
//!
//! The Skill service owns curation semantics, while runtime-host and future
//! Store/EventLog providers choose the Strategy that executes a run.  These
//! DTOs are intentionally metadata-only: reports carry bounded recommendations,
//! refs, counters, and policy ids, never raw prompts, provider payloads, package
//! bytes, manifests, or full `SKILL.md` bodies.

use chrono::{DateTime, Duration, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::governance::{SkillGovernanceRecord, SkillLifecycleState};
use crate::governance_store::SkillCurationRunRecord;
use crate::service_contract::{SkillServicePolicyHints, SkillServiceScope};
use crate::telemetry::SkillUsageTelemetry;

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
        Ok(())
    }
}

/// Recommendation action names are intentionally non-destructive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCurationAction {
    Keep,
    Protected,
    WouldMarkStale,
    WouldArchive,
    WouldReviewForConsolidation,
}

/// One deterministic recommendation in a curation report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillCurationRecommendation {
    pub skill_id: String,
    pub name: String,
    pub action: SkillCurationAction,
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
        deterministic_report(
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
        let dry_run_report = deterministic_report(
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
        let report_ref = Some(format!(
            "store://skill-curation/{}/REPORT.md",
            run_id(&command.trace, started_at)
        ));
        let run = SkillCurationRunRecord {
            run_id: run_id(&command.trace, started_at),
            trace_id: command.trace.trace_id.clone(),
            provider_id: "local-skill-governance".into(),
            dry_run: command.dry_run,
            candidate_count: dry_run_report.recommendations.len() as u64,
            actions: action_refs,
            snapshot_refs: Vec::new(),
            started_at,
            finished_at: Some(finished_at),
            report_ref: report_ref.clone(),
            rollback_ref: None,
            policy_decision_ids: non_empty(&command.policy_decision_refs),
            audit_event_ids: non_empty(&command.audit_event_ids),
        };

        Self {
            run,
            recommendations: dry_run_report.recommendations,
            semantic_analysis_status: dry_run_report.semantic_analysis_status,
            report_ref,
            rollback_ref: None,
            mutated: !command.dry_run,
            captured_at: finished_at,
        }
    }
}

fn deterministic_report(
    records: impl IntoIterator<Item = SkillGovernanceRecord>,
    stale_after_days: i64,
    narrow_use_threshold: u64,
    now: DateTime<Utc>,
) -> SkillCurationDryRunResult {
    let stale_after = Duration::days(stale_after_days.max(0));
    let recommendations = records
        .into_iter()
        .map(|record| recommend_for_record(record, stale_after, narrow_use_threshold, now))
        .collect();

    SkillCurationDryRunResult {
        recommendations,
        semantic_analysis_status:
            "unavailable: deterministic curation did not call an LLM or similarity provider".into(),
        mutated: false,
        captured_at: now,
    }
}

fn recommend_for_record(
    record: SkillGovernanceRecord,
    stale_after: Duration,
    narrow_use_threshold: u64,
    now: DateTime<Utc>,
) -> SkillCurationRecommendation {
    let skill_id = record.provenance.skill_id.clone();
    let name = record.provenance.name.clone();
    let evidence_ids = record.evidence_ids.clone();
    if record.pinned {
        return SkillCurationRecommendation {
            skill_id,
            name,
            action: SkillCurationAction::Protected,
            rationale:
                "skill is pinned, so curation may observe it but must not recommend mutation".into(),
            protected: true,
            confidence: 1.0,
            evidence_ids,
        };
    }

    let last_activity = record
        .telemetry
        .last_activity_at()
        .unwrap_or(record.provenance.created_at);
    let inactive_for = now.signed_duration_since(last_activity);
    let action = if record.lifecycle == SkillLifecycleState::Stale && inactive_for >= stale_after {
        SkillCurationAction::WouldArchive
    } else if inactive_for >= stale_after {
        SkillCurationAction::WouldMarkStale
    } else if should_review_for_consolidation(&record.telemetry, narrow_use_threshold) {
        SkillCurationAction::WouldReviewForConsolidation
    } else {
        SkillCurationAction::Keep
    };
    let rationale = match action {
        SkillCurationAction::Keep => {
            "skill has recent or sufficient usage for this deterministic policy".into()
        }
        SkillCurationAction::WouldMarkStale => {
            "skill has no recent usage beyond the configured stale threshold".into()
        }
        SkillCurationAction::WouldArchive => {
            "skill is already stale and remains inactive beyond the configured threshold".into()
        }
        SkillCurationAction::WouldReviewForConsolidation => {
            "agent-created skill has low usage and should be reviewed for umbrella consolidation"
                .into()
        }
        SkillCurationAction::Protected => "skill is protected".into(),
    };

    SkillCurationRecommendation {
        skill_id,
        name,
        action,
        rationale,
        protected: false,
        confidence: 0.75,
        evidence_ids,
    }
}

fn should_review_for_consolidation(
    telemetry: &SkillUsageTelemetry,
    narrow_use_threshold: u64,
) -> bool {
    let total_effective_task_outcomes = telemetry
        .successful_task_count
        .saturating_add(telemetry.failed_task_count);
    telemetry.use_count <= narrow_use_threshold
        && telemetry.patch_count == 0
        && telemetry.successful_task_count == 0
        && total_effective_task_outcomes <= narrow_use_threshold
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
