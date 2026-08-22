//! Provider-neutral workflow schedule admission and lifecycle semantics.
//!
//! This module is deliberately an executable Specification boundary.  It validates
//! bounded schedule references, permissions, resource reservations, and approval
//! evidence before a runtime provider is called.  It does not parse cron/RRULE
//! expressions, resolve time zones, create tasks, or select a concrete provider.
//! Those responsibilities remain replaceable Strategies behind the service runtime.
//!
//! The data returned here is safe to retain as a Memento: it contains references,
//! counters, hashes, and policy decisions, never action payloads or provider output.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::workflow_common::{workflow_stable_hash, WorkflowPage};
use super::workflow_schedule::{
    ScheduleBackfillRequest, ScheduleRecurrence, ScheduleTimezonePolicy, ScheduleTriggerRecord,
    WorkflowSchedule, WorkflowScheduleSpec, WorkflowScheduleState,
};
use crate::audit_redaction;

const MAX_REFERENCE: usize = 256;
const MAX_PAGE_SIZE: u32 = 256;

/// Resource counters reserved before a provider may materialize schedule work.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleResourceReservation {
    pub active_schedules: u32,
    pub pending_triggers: u32,
    pub backfill_triggers: u32,
    pub preview_occurrences: u32,
    pub history_records: u32,
    pub retained_snapshots: u32,
}

/// Configured resource ceilings for one admission decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleResourceLimits {
    pub active_schedules: u32,
    pub pending_triggers: u32,
    pub backfill_triggers: u32,
    pub preview_occurrences: u32,
    pub history_records: u32,
    pub retained_snapshots: u32,
}

/// Policy facts required by all schedule commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulePolicyContext {
    pub declared_permissions: BTreeSet<String>,
    pub policy_allows: bool,
    pub provider_available: bool,
    pub high_frequency_allowed: bool,
    pub critical_target_allowed: bool,
    pub limits: ScheduleResourceLimits,
    pub current: ScheduleResourceReservation,
}

/// Approval evidence is reference-only and can be replayed without secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleApprovalEvidence {
    pub approval_ref: String,
    pub authority_ref: String,
    pub reason_ref: String,
}

/// Stable reason returned by the preflight Specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleAdmissionFailure {
    PermissionNotDeclared,
    PolicyDenied,
    ProviderUnavailable,
    InvalidReference,
    InvalidRecurrence,
    InvalidTimezone,
    DstUnresolved,
    MisfireBlocked,
    OverlapBlocked,
    BackfillTooLarge,
    QuotaExceeded,
    ApprovalRequired,
    SchedulePaused,
    VersionMismatch,
    TriggerConflict,
}

/// A bounded, trace-addressable trigger proof created by the State machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleTriggerEvidence {
    pub trigger_ref: String,
    pub schedule_ref: String,
    pub idempotency_key: String,
    pub scheduled_epoch_ms: u64,
    pub logical_epoch_ms: u64,
    pub action_ref: String,
    pub policy_evidence_ref: String,
}

/// Validate permissions and schedule semantics before any provider side effect.
pub fn preflight_schedule(
    spec: &WorkflowScheduleSpec,
    context: &SchedulePolicyContext,
    required_permission: &str,
) -> Result<(), ScheduleAdmissionFailure> {
    if !context.declared_permissions.contains(required_permission) {
        return Err(ScheduleAdmissionFailure::PermissionNotDeclared);
    }
    if !context.policy_allows {
        return Err(ScheduleAdmissionFailure::PolicyDenied);
    }
    if !context.provider_available {
        return Err(ScheduleAdmissionFailure::ProviderUnavailable);
    }
    validate_spec(spec, context)
}

/// Validate recurrence, time-zone, misfire, overlap, target, and frequency policy.
pub fn validate_spec(
    spec: &WorkflowScheduleSpec,
    context: &SchedulePolicyContext,
) -> Result<(), ScheduleAdmissionFailure> {
    if !bounded(&spec.spec_ref) || !bounded(&spec.action_ref) {
        return Err(ScheduleAdmissionFailure::InvalidReference);
    }
    validate_recurrence(&spec.recurrence)?;
    validate_timezone(&spec.timezone_policy)?;
    validate_misfire(&spec.misfire_policy)?;
    validate_overlap(&spec.overlap_policy)?;
    if spec.interval_is_high_frequency() && !context.high_frequency_allowed {
        return Err(ScheduleAdmissionFailure::PolicyDenied);
    }
    if context.current.active_schedules >= context.limits.active_schedules {
        return Err(ScheduleAdmissionFailure::QuotaExceeded);
    }
    Ok(())
}

/// Validate only the recurrence shape; evaluation remains a provider Strategy.
pub fn validate_recurrence(
    recurrence: &ScheduleRecurrence,
) -> Result<(), ScheduleAdmissionFailure> {
    if !bounded(&recurrence.recurrence_ref) || !bounded(&recurrence.kind) {
        return Err(ScheduleAdmissionFailure::InvalidRecurrence);
    }
    let has_rule = recurrence.has_declared_rule();
    let valid_kind = matches!(
        recurrence.kind.as_str(),
        "one_shot" | "interval" | "cron" | "rrule" | "event"
    );
    if !has_rule || !valid_kind {
        return Err(ScheduleAdmissionFailure::InvalidRecurrence);
    }
    if recurrence.kind == "interval" && recurrence.interval_ms.unwrap_or(0) == 0 {
        return Err(ScheduleAdmissionFailure::InvalidRecurrence);
    }
    Ok(())
}

/// Validate DST strategies without assuming a particular time-zone database.
pub fn validate_timezone(
    timezone: &ScheduleTimezonePolicy,
) -> Result<(), ScheduleAdmissionFailure> {
    if timezone.local_time_required && !bounded(&timezone.timezone_ref) {
        return Err(ScheduleAdmissionFailure::InvalidTimezone);
    }
    let gaps = ["reject", "shift_forward", "shift_backward"];
    let folds = ["reject", "earlier", "later"];
    if !gaps.contains(&timezone.dst_gap_strategy.as_str())
        || !folds.contains(&timezone.dst_fold_strategy.as_str())
    {
        return Err(ScheduleAdmissionFailure::DstUnresolved);
    }
    Ok(())
}

/// Validate bounded misfire and catch-up behavior.
pub fn validate_misfire(
    policy: &super::workflow_schedule::ScheduleMisfirePolicy,
) -> Result<(), ScheduleAdmissionFailure> {
    if !bounded(&policy.policy_ref)
        || !matches!(policy.strategy.as_str(), "skip" | "fire_once" | "catch_up")
        || (policy.strategy == "catch_up" && policy.max_catchup_triggers == 0)
    {
        return Err(ScheduleAdmissionFailure::MisfireBlocked);
    }
    Ok(())
}

/// Validate overlap behavior and require a group for queue/reject strategies.
pub fn validate_overlap(
    policy: &super::workflow_schedule::ScheduleOverlapPolicy,
) -> Result<(), ScheduleAdmissionFailure> {
    if !bounded(&policy.policy_ref)
        || !matches!(
            policy.strategy.as_str(),
            "allow" | "skip" | "queue" | "reject"
        )
        || matches!(policy.strategy.as_str(), "queue" | "reject")
            && policy
                .concurrency_group_ref
                .as_deref()
                .is_none_or(|v| !bounded(v))
    {
        return Err(ScheduleAdmissionFailure::OverlapBlocked);
    }
    Ok(())
}

/// Validate and reserve bounded preview/backfill/history resources.
pub fn reserve_resources(
    current: ScheduleResourceReservation,
    requested: ScheduleResourceReservation,
    limits: ScheduleResourceLimits,
) -> Result<ScheduleResourceReservation, ScheduleAdmissionFailure> {
    let next = ScheduleResourceReservation {
        active_schedules: current
            .active_schedules
            .saturating_add(requested.active_schedules),
        pending_triggers: current
            .pending_triggers
            .saturating_add(requested.pending_triggers),
        backfill_triggers: current
            .backfill_triggers
            .saturating_add(requested.backfill_triggers),
        preview_occurrences: current
            .preview_occurrences
            .saturating_add(requested.preview_occurrences),
        history_records: current
            .history_records
            .saturating_add(requested.history_records),
        retained_snapshots: current
            .retained_snapshots
            .saturating_add(requested.retained_snapshots),
    };
    if next.active_schedules > limits.active_schedules
        || next.pending_triggers > limits.pending_triggers
        || next.backfill_triggers > limits.backfill_triggers
        || next.preview_occurrences > limits.preview_occurrences
        || next.history_records > limits.history_records
        || next.retained_snapshots > limits.retained_snapshots
    {
        return Err(ScheduleAdmissionFailure::QuotaExceeded);
    }
    Ok(next)
}

/// Require approval for bounded-but-sensitive schedule operations.
pub fn approval_required(
    backfill: &ScheduleBackfillRequest,
    high_frequency: bool,
    critical_target: bool,
    catchup_triggers: u32,
    max_catchup_triggers: u32,
    evidence: Option<&ScheduleApprovalEvidence>,
) -> Result<(), ScheduleAdmissionFailure> {
    let large_backfill = backfill.max_triggers > 100
        || backfill.is_bounded()
            && backfill
                .end_epoch_ms
                .saturating_sub(backfill.start_epoch_ms)
                > 86_400_000;
    let catchup_flood = catchup_triggers > max_catchup_triggers;
    if !(large_backfill || high_frequency || critical_target || catchup_flood) {
        return Ok(());
    }
    match evidence {
        Some(value)
            if bounded(&value.approval_ref)
                && bounded(&value.authority_ref)
                && bounded(&value.reason_ref) =>
        {
            Ok(())
        }
        _ => Err(ScheduleAdmissionFailure::ApprovalRequired),
    }
}

/// Check that a schedule state transition is legal and versioned.
pub fn transition_schedule(
    from: WorkflowScheduleState,
    to: WorkflowScheduleState,
    expected_version: Option<&str>,
    current_version: &str,
) -> Result<(), ScheduleAdmissionFailure> {
    if expected_version != Some(current_version) {
        return Err(ScheduleAdmissionFailure::VersionMismatch);
    }
    let legal = matches!(
        (from, to),
        (WorkflowScheduleState::Draft, WorkflowScheduleState::Active)
            | (WorkflowScheduleState::Active, WorkflowScheduleState::Paused)
            | (WorkflowScheduleState::Paused, WorkflowScheduleState::Active)
            | (
                WorkflowScheduleState::Active,
                WorkflowScheduleState::Deleted
            )
            | (
                WorkflowScheduleState::Paused,
                WorkflowScheduleState::Deleted
            )
            | (
                WorkflowScheduleState::Active,
                WorkflowScheduleState::Exhausted
            )
    );
    legal
        .then_some(())
        .ok_or(ScheduleAdmissionFailure::SchedulePaused)
}

/// Derive an idempotency key from bounded references and logical time.
pub fn derive_idempotency_key(
    schedule_ref: &str,
    action_ref: &str,
    logical_epoch_ms: u64,
) -> String {
    workflow_stable_hash(&(schedule_ref, action_ref, logical_epoch_ms))
}

/// Build trigger evidence without copying the target action payload.
pub fn trigger_evidence(
    schedule: &WorkflowSchedule,
    trigger_ref: &str,
    scheduled_epoch_ms: u64,
    logical_epoch_ms: u64,
    policy_evidence_ref: &str,
) -> Result<ScheduleTriggerEvidence, ScheduleAdmissionFailure> {
    if !bounded(trigger_ref)
        || !bounded(policy_evidence_ref)
        || schedule.state != WorkflowScheduleState::Active
    {
        return Err(if schedule.state == WorkflowScheduleState::Active {
            ScheduleAdmissionFailure::InvalidReference
        } else {
            ScheduleAdmissionFailure::SchedulePaused
        });
    }
    Ok(ScheduleTriggerEvidence {
        trigger_ref: trigger_ref.into(),
        schedule_ref: schedule.schedule_ref.clone(),
        idempotency_key: derive_idempotency_key(
            &schedule.schedule_ref,
            &schedule.spec.action_ref,
            logical_epoch_ms,
        ),
        scheduled_epoch_ms,
        logical_epoch_ms,
        action_ref: schedule.spec.action_ref.clone(),
        policy_evidence_ref: policy_evidence_ref.into(),
    })
}

/// Convert a trigger proof to the public record while retaining only references.
pub fn trigger_record(evidence: &ScheduleTriggerEvidence) -> ScheduleTriggerRecord {
    ScheduleTriggerRecord {
        trigger_ref: evidence.trigger_ref.clone(),
        schedule_ref: evidence.schedule_ref.clone(),
        scheduled_epoch_ms: evidence.scheduled_epoch_ms,
        logical_epoch_ms: evidence.logical_epoch_ms,
        idempotency_key: evidence.idempotency_key.clone(),
        action_ref: evidence.action_ref.clone(),
        status: "computed".into(),
    }
}

/// Paginate an already authorized schedule projection; hidden rows do not alter shape.
pub fn filtered_schedule_page(
    authorized: &[WorkflowSchedule],
    cursor: Option<usize>,
    page_size: u32,
) -> WorkflowPage<WorkflowSchedule> {
    let start = cursor.unwrap_or(0).min(authorized.len());
    let size = page_size.clamp(1, MAX_PAGE_SIZE) as usize;
    let end = start.saturating_add(size).min(authorized.len());
    WorkflowPage {
        items: authorized[start..end].to_vec(),
        next_cursor: (end < authorized.len()).then(|| format!("cursor:{end}")),
        truncated: end < authorized.len(),
    }
}

/// Redact schedule metadata to the bounded reference fields safe for audit logs.
pub fn redacted_schedule_metadata(metadata: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    metadata
        .iter()
        .filter(|(key, value)| {
            bounded(key)
                && bounded(value)
                && !audit_redaction::is_sensitive_json_key(key)
                && !key.to_ascii_lowercase().contains("provider")
        })
        .map(|(key, value)| (key.clone(), format!("ref:{}", workflow_stable_hash(value))))
        .collect()
}

fn bounded(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_REFERENCE && !value.contains('\n')
}

trait ScheduleSpecFrequency {
    fn interval_is_high_frequency(&self) -> bool;
}

impl ScheduleSpecFrequency for WorkflowScheduleSpec {
    fn interval_is_high_frequency(&self) -> bool {
        self.recurrence
            .interval_ms
            .is_some_and(|value| value < 1_000)
    }
}
