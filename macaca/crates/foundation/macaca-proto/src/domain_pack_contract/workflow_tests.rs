use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::workflow_approval::*;
use super::workflow_common::{WorkflowCommandEnvelope, WorkflowError};
use super::workflow_delegation::*;
use super::workflow_recovery::*;
use super::workflow_review::*;
use super::workflow_schedule::*;
use super::workflow_task::*;
use super::*;

// Workflow tests validate provider-neutral contract shape only. They do not
// start workflow engines, schedulers, approval UIs, agent runtimes, reviewers,
// recovery engines, plugin adapters, remote services, mock providers, or
// unavailable providers. Fixtures use references, hashes, counters, states, and
// bounded diagnostics instead of raw payloads, prompts, artifacts, checkpoint
// bytes, worker logs, comments, credentials, provider payloads, or replay dumps.

#[test]
fn workflow_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            workflow_task_pack_definition(),
            WORKFLOW_TASK_PACK_ID,
            WORKFLOW_TASK_SERVICE_ID,
            WORKFLOW_TASK_COMMANDS,
            "workflow_task_provider_not_installed",
            "durable-task-engine",
            "workflow_task.claim",
        ),
        (
            workflow_schedule_pack_definition(),
            WORKFLOW_SCHEDULE_PACK_ID,
            WORKFLOW_SCHEDULE_SERVICE_ID,
            WORKFLOW_SCHEDULE_COMMANDS,
            "workflow_schedule_provider_not_installed",
            "durable-scheduler",
            "workflow_schedule.backfill",
        ),
        (
            workflow_approval_pack_definition(),
            WORKFLOW_APPROVAL_PACK_ID,
            WORKFLOW_APPROVAL_SERVICE_ID,
            WORKFLOW_APPROVAL_COMMANDS,
            "workflow_approval_provider_not_installed",
            "durable-approval",
            "approval.record_decision",
        ),
        (
            workflow_delegation_pack_definition(),
            WORKFLOW_DELEGATION_PACK_ID,
            WORKFLOW_DELEGATION_SERVICE_ID,
            WORKFLOW_DELEGATION_COMMANDS,
            "workflow_delegation_provider_not_installed",
            "durable-delegation",
            "delegation.accept_delegation",
        ),
        (
            workflow_review_pack_definition(),
            WORKFLOW_REVIEW_PACK_ID,
            WORKFLOW_REVIEW_SERVICE_ID,
            WORKFLOW_REVIEW_COMMANDS,
            "workflow_review_provider_not_installed",
            "durable-review",
            "review.evaluate_gate",
        ),
        (
            workflow_recovery_pack_definition(),
            WORKFLOW_RECOVERY_PACK_ID,
            WORKFLOW_RECOVERY_SERVICE_ID,
            WORKFLOW_RECOVERY_COMMANDS,
            "workflow_recovery_provider_not_installed",
            "durable-recovery",
            "recovery.export_replay",
        ),
    ];

    for (definition, pack_id, service_id, commands, unavailable_reason, provider_class, command) in
        cases
    {
        assert_eq!(definition.pack_id, pack_id);
        assert!(!definition.is_callable());
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert_eq!(
            definition.metadata.parent_pack_id.as_deref(),
            Some("pack.workflow.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/workflow"));
        assert!(definition
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|schemas| schemas.contains(command)));

        let descriptor_commands = definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .expect("workflow descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_workflow_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();

    for (pack_id, provider_class, metadata_key) in [
        (
            WORKFLOW_TASK_PACK_ID,
            "durable-task-engine",
            "raw_payloads_in_trace",
        ),
        (
            WORKFLOW_SCHEDULE_PACK_ID,
            "durable-scheduler",
            "raw_action_payloads_in_trace",
        ),
        (
            WORKFLOW_APPROVAL_PACK_ID,
            "durable-approval",
            "raw_evidence_in_trace",
        ),
        (
            WORKFLOW_DELEGATION_PACK_ID,
            "durable-delegation",
            "raw_work_payloads_in_trace",
        ),
        (
            WORKFLOW_REVIEW_PACK_ID,
            "durable-review",
            "raw_subject_in_trace",
        ),
        (
            WORKFLOW_RECOVERY_PACK_ID,
            "durable-recovery",
            "raw_checkpoint_bytes_in_trace",
        ),
    ] {
        let pack = find_pack(&definitions, pack_id);
        assert_eq!(
            pack.metadata
                .provider_descriptors
                .get(provider_class)
                .and_then(|descriptor| descriptor.metadata.get(metadata_key))
                .map(String::as_str),
            Some("false")
        );
    }
}

#[test]
fn workflow_command_and_result_dtos_are_serde_compatible() {
    let envelope = WorkflowCommandEnvelope {
        subject_ref: "workflow:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "synthetic".into())]),
        cursor: None,
        page_size: Some(10),
        idempotency_key: Some("idem-workflow".into()),
        expected_version: Some("v1".into()),
    };

    let values = [
        serde_json::to_value(WorkflowTaskCreateCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(WorkflowSchedulePreviewCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(ApprovalRecordDecisionCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(DelegationAcceptDelegationCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(ReviewEvaluateGateCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(RecoveryExportReplayCommand { request: envelope }).unwrap(),
        serde_json::to_value(WorkflowTaskResultEnvelope::<WorkflowTask> {
            status: WorkflowTaskResultStatus::LeaseExpired,
            data: None,
            page: None,
            error: Some(WorkflowError {
                code: "lease_expired".into(),
                message: "synthetic lease expired".into(),
                retryable: true,
                trace_safe_detail: Some("lease:ref".into()),
            }),
        })
        .unwrap(),
        serde_json::to_value(WorkflowScheduleResultEnvelope::<WorkflowSchedule> {
            status: WorkflowScheduleResultStatus::DstUnresolved,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(WorkflowApprovalResultEnvelope::<ApprovalRequest> {
            status: WorkflowApprovalResultStatus::EligibilityRevoked,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(WorkflowDelegationResultEnvelope::<DelegationRequest> {
            status: WorkflowDelegationResultStatus::CapacityExhausted,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(WorkflowReviewResultEnvelope::<ReviewFinding> {
            status: WorkflowReviewResultStatus::BlockingFindings,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(WorkflowRecoveryResultEnvelope::<FailureRecord> {
            status: WorkflowRecoveryResultStatus::RetryBudgetExhausted,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn workflow_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        hash_values(&workflow_task_descriptor_hashes()),
        hash_values(&workflow_schedule_descriptor_hashes()),
        hash_values(&workflow_approval_descriptor_hashes()),
        hash_values(&workflow_delegation_descriptor_hashes()),
        hash_values(&workflow_review_descriptor_hashes()),
        hash_values(&workflow_recovery_descriptor_hashes()),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert!(unique.len() >= 5);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn workflow_validation_helpers_are_provider_neutral() {
    let lease = TaskLease {
        lease_ref: "lease".into(),
        expires_at_epoch_ms: 10,
        ..Default::default()
    };
    let retry = super::workflow_task::RetryPolicy {
        policy_ref: "retry".into(),
        max_attempts: 3,
        backoff_ms: 1_000,
        retryable_codes: BTreeSet::from(["transient".into()]),
    };
    let recurrence = ScheduleRecurrence {
        recurrence_ref: "recurrence".into(),
        kind: "rrule".into(),
        expression_ref: "FREQ=DAILY".into(),
        ..Default::default()
    };
    let backfill = ScheduleBackfillRequest {
        start_epoch_ms: 1,
        end_epoch_ms: 2,
        max_triggers: 1,
        ..Default::default()
    };
    let gate = ApprovalDecisionGate {
        gate_ref: "gate".into(),
        required_outcome: "approved".into(),
        decision_ref: Some("decision".into()),
        ..Default::default()
    };
    let delegation_lease = DelegationLease {
        expires_at_epoch_ms: 10,
        ..Default::default()
    };
    let closure_gate = ReviewClosureGate {
        outcome_ref: Some("outcome".into()),
        ..Default::default()
    };
    let resume = ResumePlan {
        recovery_point_ref: "recovery-point".into(),
        compatibility_checked: true,
        ..Default::default()
    };

    assert!(lease.is_active_at(1));
    assert!(retry.is_bounded());
    assert!(recurrence.has_declared_rule());
    assert!(backfill.is_bounded());
    assert!(gate.is_satisfied());
    assert!(delegation_lease.is_active_at(1));
    assert!(closure_gate.can_close());
    assert!(resume.can_resume());
}

#[test]
fn workflow_task_and_schedule_fixtures_cover_contract_shapes() {
    let task = WorkflowTask {
        task_ref: "task:1".into(),
        spec: WorkflowTaskSpec {
            spec_ref: "task-spec:1".into(),
            task_kind: "synthetic".into(),
            queue: TaskQueueRef {
                queue_ref: "queue:default".into(),
                priority_class: "normal".into(),
                concurrency_group_ref: Some("group:io".into()),
            },
            dependencies: vec![TaskDependency {
                dependency_ref: "dependency:1".into(),
                upstream_task_ref: "task:0".into(),
                required_state: WorkflowTaskState::Completed,
                blocking: true,
            }],
            retry_policy: super::workflow_task::RetryPolicy {
                policy_ref: "retry:bounded".into(),
                max_attempts: 3,
                backoff_ms: 1_000,
                retryable_codes: BTreeSet::from(["transient".into()]),
            },
            concurrency_policy: ConcurrencyPolicy {
                policy_ref: "concurrency:bounded".into(),
                group_ref: "group:io".into(),
                max_in_flight: 2,
                overflow_action: "queue".into(),
            },
            timeout_ms: 60_000,
            checkpoint_policy_ref: "checkpoint:every-progress".into(),
        },
        state: WorkflowTaskState::Queued,
        version: "v1".into(),
        attempt: Some(TaskAttempt {
            attempt_ref: "attempt:1".into(),
            attempt_index: 1,
            started_at_epoch_ms: 1,
            retry_after_epoch_ms: Some(2),
        }),
        progress: Some(TaskProgress {
            progress_ref: "progress:1".into(),
            completed_units: 1,
            total_units: Some(10),
            message_ref: Some("message:progress".into()),
        }),
    };
    let checkpoint = TaskCheckpoint {
        checkpoint_ref: "checkpoint:1".into(),
        task_ref: task.task_ref.clone(),
        content_hash: "hash:checkpoint".into(),
        replay_cursor: "cursor:1".into(),
        schema_version: "v1".into(),
    };
    let artifact = TaskArtifactRef {
        artifact_ref: "artifact:1".into(),
        task_ref: task.task_ref.clone(),
        artifact_kind: "report".into(),
        content_hash: "hash:artifact".into(),
        redaction_profile: "default".into(),
    };

    let schedule = WorkflowSchedule {
        schedule_ref: "schedule:1".into(),
        spec: WorkflowScheduleSpec {
            spec_ref: "schedule-spec:1".into(),
            recurrence: ScheduleRecurrence {
                recurrence_ref: "recurrence:1".into(),
                kind: "rrule".into(),
                expression_ref: "FREQ=HOURLY".into(),
                interval_ms: None,
                rrule_ref: Some("rrule:1".into()),
                exclusion_set_ref: Some("exclusion:holidays".into()),
            },
            timezone_policy: ScheduleTimezonePolicy {
                timezone_ref: "timezone:utc".into(),
                dst_gap_strategy: "skip".into(),
                dst_fold_strategy: "first".into(),
                local_time_required: false,
            },
            misfire_policy: ScheduleMisfirePolicy {
                policy_ref: "misfire:bounded".into(),
                strategy: "fire_once".into(),
                catchup_window_ms: 3_600_000,
                max_catchup_triggers: 1,
            },
            overlap_policy: ScheduleOverlapPolicy {
                policy_ref: "overlap:skip".into(),
                strategy: "skip".into(),
                concurrency_group_ref: Some("group:io".into()),
            },
            action_ref: "action:task-enqueue".into(),
            jitter_ms: Some(1_000),
        },
        state: WorkflowScheduleState::Active,
        version: "v1".into(),
        next_trigger_epoch_ms: Some(10),
    };
    let trigger = ScheduleTriggerRecord {
        trigger_ref: "trigger:1".into(),
        schedule_ref: schedule.schedule_ref.clone(),
        scheduled_epoch_ms: 10,
        logical_epoch_ms: 10,
        idempotency_key: "schedule:1:10".into(),
        action_ref: "action:task-enqueue".into(),
        status: "pending".into(),
    };

    assert_eq!(task.spec.dependencies.len(), 1);
    assert!(task.spec.retry_policy.is_bounded());
    assert_eq!(task.spec.concurrency_policy.max_in_flight, 2);
    assert_eq!(checkpoint.schema_version, "v1");
    assert_eq!(artifact.redaction_profile, "default");
    assert!(schedule.spec.recurrence.has_declared_rule());
    assert_eq!(schedule.spec.misfire_policy.max_catchup_triggers, 1);
    assert_eq!(schedule.spec.overlap_policy.strategy, "skip");
    assert_eq!(trigger.idempotency_key, "schedule:1:10");
}

fn hash_values<T: Serialize>(value: &T) -> Vec<String> {
    serde_json::to_value(value)
        .expect("descriptor hashes serialize")
        .as_object()
        .expect("descriptor hashes are object-shaped")
        .values()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect()
}

fn find_pack<'a>(
    definitions: &'a [DomainPackDefinition],
    pack_id: &str,
) -> &'a DomainPackDefinition {
    definitions
        .iter()
        .find(|definition| definition.pack_id == pack_id)
        .expect("industrial catalog includes specialized workflow descriptor")
}
