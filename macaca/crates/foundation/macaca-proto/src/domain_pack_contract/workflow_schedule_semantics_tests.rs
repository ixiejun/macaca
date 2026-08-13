//! Deterministic tests for workflow schedule Specification and State semantics.
//!
//! These tests intentionally use synthetic references.  They prove that denied,
//! unavailable, malformed, over-quota, paused, and approval-gated requests stop
//! before a provider can create a task or invoke a service command.

use std::collections::{BTreeMap, BTreeSet};

use super::workflow_schedule::*;
use super::workflow_schedule_semantics::*;

fn context() -> SchedulePolicyContext {
    SchedulePolicyContext {
        declared_permissions: BTreeSet::from(["workflow.schedule.write".into()]),
        policy_allows: true,
        provider_available: true,
        high_frequency_allowed: true,
        critical_target_allowed: true,
        limits: ScheduleResourceLimits {
            active_schedules: 4,
            pending_triggers: 10,
            backfill_triggers: 20,
            preview_occurrences: 20,
            history_records: 20,
            retained_snapshots: 4,
        },
        current: ScheduleResourceReservation::default(),
    }
}

fn spec() -> WorkflowScheduleSpec {
    WorkflowScheduleSpec {
        spec_ref: "spec:1".into(),
        recurrence: ScheduleRecurrence {
            recurrence_ref: "recurrence:1".into(),
            kind: "interval".into(),
            expression_ref: "interval".into(),
            interval_ms: Some(60_000),
            ..Default::default()
        },
        timezone_policy: ScheduleTimezonePolicy {
            timezone_ref: "tz:utc".into(),
            dst_gap_strategy: "reject".into(),
            dst_fold_strategy: "earlier".into(),
            local_time_required: false,
        },
        misfire_policy: ScheduleMisfirePolicy {
            policy_ref: "misfire:1".into(),
            strategy: "catch_up".into(),
            catchup_window_ms: 60_000,
            max_catchup_triggers: 3,
        },
        overlap_policy: ScheduleOverlapPolicy {
            policy_ref: "overlap:1".into(),
            strategy: "queue".into(),
            concurrency_group_ref: Some("group:1".into()),
        },
        action_ref: "action:1".into(),
        jitter_ms: None,
    }
}

fn schedule() -> WorkflowSchedule {
    WorkflowSchedule {
        schedule_ref: "schedule:1".into(),
        spec: spec(),
        state: WorkflowScheduleState::Active,
        version: "v1".into(),
        next_trigger_epoch_ms: None,
    }
}

#[test]
fn preflight_rejects_permission_policy_and_unavailable_before_provider() {
    let mut denied = context();
    denied.declared_permissions.clear();
    assert_eq!(
        preflight_schedule(&spec(), &denied, "workflow.schedule.write"),
        Err(ScheduleAdmissionFailure::PermissionNotDeclared)
    );
    let mut policy = context();
    policy.policy_allows = false;
    assert_eq!(
        preflight_schedule(&spec(), &policy, "workflow.schedule.write"),
        Err(ScheduleAdmissionFailure::PolicyDenied)
    );
    let mut unavailable = context();
    unavailable.provider_available = false;
    assert_eq!(
        preflight_schedule(&spec(), &unavailable, "workflow.schedule.write"),
        Err(ScheduleAdmissionFailure::ProviderUnavailable)
    );
}

#[test]
fn validation_covers_recurrence_dst_misfire_and_overlap() {
    let mut value = spec();
    value.recurrence.kind = "unknown".into();
    assert_eq!(
        validate_recurrence(&value.recurrence),
        Err(ScheduleAdmissionFailure::InvalidRecurrence)
    );
    value = spec();
    value.timezone_policy.dst_gap_strategy = "unknown".into();
    assert_eq!(
        validate_timezone(&value.timezone_policy),
        Err(ScheduleAdmissionFailure::DstUnresolved)
    );
    value = spec();
    value.misfire_policy.max_catchup_triggers = 0;
    assert_eq!(
        validate_misfire(&value.misfire_policy),
        Err(ScheduleAdmissionFailure::MisfireBlocked)
    );
    value = spec();
    value.overlap_policy.concurrency_group_ref = None;
    assert_eq!(
        validate_overlap(&value.overlap_policy),
        Err(ScheduleAdmissionFailure::OverlapBlocked)
    );
}

#[test]
fn resource_and_approval_gates_are_bounded() {
    let limits = context().limits;
    let current = ScheduleResourceReservation::default();
    let requested = ScheduleResourceReservation {
        backfill_triggers: 21,
        ..Default::default()
    };
    assert_eq!(
        reserve_resources(current, requested, limits),
        Err(ScheduleAdmissionFailure::QuotaExceeded)
    );
    let request = ScheduleBackfillRequest {
        backfill_ref: "backfill:1".into(),
        schedule_ref: "schedule:1".into(),
        start_epoch_ms: 0,
        end_epoch_ms: 2 * 86_400_000,
        max_triggers: 2,
        approval_ref: None,
    };
    assert_eq!(
        approval_required(&request, false, false, 0, 3, None),
        Err(ScheduleAdmissionFailure::ApprovalRequired)
    );
    let evidence = ScheduleApprovalEvidence {
        approval_ref: "approval:1".into(),
        authority_ref: "principal:1".into(),
        reason_ref: "reason:1".into(),
    };
    assert!(approval_required(&request, false, false, 0, 3, Some(&evidence)).is_ok());
}

#[test]
fn state_trigger_and_idempotency_evidence_are_deterministic() {
    assert!(transition_schedule(
        WorkflowScheduleState::Active,
        WorkflowScheduleState::Paused,
        Some("v1"),
        "v1"
    )
    .is_ok());
    assert_eq!(
        transition_schedule(
            WorkflowScheduleState::Active,
            WorkflowScheduleState::Deleted,
            None,
            "v1"
        ),
        Err(ScheduleAdmissionFailure::VersionMismatch)
    );
    let first = trigger_evidence(&schedule(), "trigger:1", 100, 200, "policy:1").unwrap();
    let second = trigger_evidence(&schedule(), "trigger:2", 100, 200, "policy:1").unwrap();
    assert_eq!(first.idempotency_key, second.idempotency_key);
    assert_eq!(trigger_record(&first).status, "computed");
    let mut paused = schedule();
    paused.state = WorkflowScheduleState::Paused;
    assert_eq!(
        trigger_evidence(&paused, "trigger:1", 100, 200, "policy:1"),
        Err(ScheduleAdmissionFailure::SchedulePaused)
    );
}

#[test]
fn filtered_pages_and_redaction_never_echo_sensitive_values() {
    let schedules = vec![
        schedule(),
        WorkflowSchedule {
            schedule_ref: "schedule:2".into(),
            ..schedule()
        },
    ];
    let page = filtered_schedule_page(&schedules, None, 1);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.next_cursor.as_deref(), Some("cursor:1"));
    let metadata = BTreeMap::from([
        ("safe_ref".into(), "value:1".into()),
        ("payload".into(), "raw payload".into()),
        ("secret_token".into(), "credential".into()),
    ]);
    let redacted = redacted_schedule_metadata(&metadata);
    assert_eq!(redacted.len(), 1);
    assert!(redacted.values().all(|value| value.starts_with("ref:")));
}
