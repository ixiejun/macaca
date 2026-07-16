use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::workflow_common::{
    define_workflow_command_wrappers, workflow_pack_definition, workflow_stable_hash,
    WorkflowCommandEnvelope, WorkflowError, WorkflowPackDescriptor, WorkflowPage,
    WorkflowProviderClass,
};

pub const WORKFLOW_TASK_PACK_ID: &str = "pack.workflow.task.v1";
pub const WORKFLOW_TASK_SERVICE_ID: &str = "service.workflow.task";

pub const WORKFLOW_TASK_COMMANDS: &[&str] = &[
    "workflow_task.create",
    "workflow_task.update",
    "workflow_task.patch_metadata",
    "workflow_task.enqueue",
    "workflow_task.claim",
    "workflow_task.heartbeat",
    "workflow_task.release",
    "workflow_task.record_progress",
    "workflow_task.record_checkpoint",
    "workflow_task.attach_artifact",
    "workflow_task.complete",
    "workflow_task.fail",
    "workflow_task.cancel",
    "workflow_task.skip",
    "workflow_task.get",
    "workflow_task.list",
    "workflow_task.get_history",
    "workflow_task.snapshot",
    "workflow_task.inspect_provider",
];

const TASK_PERMISSION_SCOPES: &[&str] = &[
    "workflow.task.read",
    "workflow.task.write",
    "workflow.task.queue",
    "workflow.task.claim",
    "workflow.task.progress",
    "workflow.task.complete",
    "workflow.task.admin",
];

const DURABLE_METADATA: &[(&str, &str)] = &[
    ("queues", "true"),
    ("leases", "true"),
    ("event_history", "bounded"),
    ("raw_payloads_in_trace", "false"),
];
const REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_engine", "true"),
    ("adapter_required", "service_runtime"),
    ("provider_payloads_in_trace", "false"),
];
const PLUGIN_METADATA: &[(&str, &str)] = &[("plugin", "true"), ("conformance", "required")];
const MOCK_METADATA: &[(&str, &str)] = &[("deterministic", "true"), ("task_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const TASK_PROVIDER_CLASSES: &[WorkflowProviderClass<'_>] = &[
    WorkflowProviderClass {
        provider_class: "durable-task-engine",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: DURABLE_METADATA,
    },
    WorkflowProviderClass {
        provider_class: "remote-workflow-engine",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: REMOTE_METADATA,
    },
    WorkflowProviderClass {
        provider_class: "plugin",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PLUGIN_METADATA,
    },
    WorkflowProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MOCK_METADATA,
    },
    WorkflowProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: UNAVAILABLE_METADATA,
    },
];

/// Build the workflow-task descriptor without binding a concrete workflow engine.
pub fn workflow_task_pack_definition() -> DomainPackDefinition {
    workflow_pack_definition(WorkflowPackDescriptor {
        pack_id: WORKFLOW_TASK_PACK_ID,
        child_change_id: "openspec:add-pack-workflow-task",
        docs_slug: "task",
        sdk_slug: "task",
        service_id: WORKFLOW_TASK_SERVICE_ID,
        commands: WORKFLOW_TASK_COMMANDS,
        permission_scopes: TASK_PERMISSION_SCOPES,
        provider_classes: TASK_PROVIDER_CLASSES,
        health_probe: "workflow_task.inspect_provider",
        unavailable_reason: "workflow_task_provider_not_installed",
        replay_schema: "workflow.task.replay.v1",
        data_classification: "workflow_task_reference_metadata",
        retention_policy: "task_queue_lease_attempt_retry_dependency_progress_checkpoint_artifact_history_and_snapshot_metadata_by_reference",
        redaction_policy: "raw_task_payloads_prompts_artifacts_worker_logs_provider_payloads_credentials_and_unbounded_histories_redacted",
        timeout_ms: 180_000,
        budget_units: 12,
        examples: &[
            "Declare `pack.workflow.task.v1` as optional until a task provider is installed.",
            "Use task, queue, lease, checkpoint, artifact, and history references instead of raw task payloads.",
        ],
        migration_notes: &[
            "Task commands become callable only after an approved workflow task service provider registers matching schemas.",
            "Task boards, shell rendering, planners, reviewers, and business workflows remain outside this provider-neutral DTO contract.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTask {
    pub task_ref: String,
    pub spec: WorkflowTaskSpec,
    pub state: WorkflowTaskState,
    pub version: String,
    pub attempt: Option<TaskAttempt>,
    pub progress: Option<TaskProgress>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTaskSpec {
    pub spec_ref: String,
    pub task_kind: String,
    pub queue: TaskQueueRef,
    pub dependencies: Vec<TaskDependency>,
    pub retry_policy: RetryPolicy,
    pub concurrency_policy: ConcurrencyPolicy,
    pub timeout_ms: u64,
    pub checkpoint_policy_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDependency {
    pub dependency_ref: String,
    pub upstream_task_ref: String,
    pub required_state: WorkflowTaskState,
    pub blocking: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskQueueRef {
    pub queue_ref: String,
    pub priority_class: String,
    pub concurrency_group_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLease {
    pub lease_ref: String,
    pub task_ref: String,
    pub owner_ref: String,
    pub expires_at_epoch_ms: u64,
    pub heartbeat_deadline_epoch_ms: u64,
    pub revoked: bool,
}

impl TaskLease {
    /// Check lease freshness with a deterministic caller-provided time value.
    pub fn is_active_at(&self, now_epoch_ms: u64) -> bool {
        !self.revoked && now_epoch_ms < self.expires_at_epoch_ms
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskAttempt {
    pub attempt_ref: String,
    pub attempt_index: u32,
    pub started_at_epoch_ms: u64,
    pub retry_after_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub policy_ref: String,
    pub max_attempts: u32,
    pub backoff_ms: u64,
    pub retryable_codes: BTreeSet<String>,
}

impl RetryPolicy {
    /// A bounded retry policy prevents hidden infinite autonomous loops.
    pub fn is_bounded(&self) -> bool {
        self.max_attempts > 0 && self.max_attempts <= 100
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcurrencyPolicy {
    pub policy_ref: String,
    pub group_ref: String,
    pub max_in_flight: u32,
    pub overflow_action: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskProgress {
    pub progress_ref: String,
    pub completed_units: u64,
    pub total_units: Option<u64>,
    pub message_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCheckpoint {
    pub checkpoint_ref: String,
    pub task_ref: String,
    pub content_hash: String,
    pub replay_cursor: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskArtifactRef {
    pub artifact_ref: String,
    pub task_ref: String,
    pub artifact_kind: String,
    pub content_hash: String,
    pub redaction_profile: String,
}

pub type WorkflowTaskError = WorkflowError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTaskState {
    #[default]
    Draft,
    Queued,
    Claimed,
    Running,
    Blocked,
    Review,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTaskResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    InvalidState,
    Conflict,
    DependencyBlocked,
    LeaseExpired,
    LeaseRevoked,
    RetryExhausted,
    ConcurrencyBlocked,
    QuotaExceeded,
    ArtifactBlocked,
    ProviderFailure,
    VersionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTaskResultEnvelope<T> {
    pub status: WorkflowTaskResultStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<WorkflowPage<T>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkflowTaskError>,
}

define_workflow_command_wrappers!(
    WorkflowTaskCreateCommand,
    WorkflowTaskUpdateCommand,
    WorkflowTaskPatchMetadataCommand,
    WorkflowTaskEnqueueCommand,
    WorkflowTaskClaimCommand,
    WorkflowTaskHeartbeatCommand,
    WorkflowTaskReleaseCommand,
    WorkflowTaskRecordProgressCommand,
    WorkflowTaskRecordCheckpointCommand,
    WorkflowTaskAttachArtifactCommand,
    WorkflowTaskCompleteCommand,
    WorkflowTaskFailCommand,
    WorkflowTaskCancelCommand,
    WorkflowTaskSkipCommand,
    WorkflowTaskGetCommand,
    WorkflowTaskListCommand,
    WorkflowTaskGetHistoryCommand,
    WorkflowTaskSnapshotCommand,
    WorkflowTaskInspectProviderCommand,
);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTaskDescriptorHashes {
    pub descriptor_hash: String,
    pub commands_hash: String,
    pub permissions_hash: String,
    pub providers_hash: String,
    pub state_machine_hash: String,
    pub retry_hash: String,
    pub lease_hash: String,
    pub dependency_hash: String,
}

pub fn workflow_task_descriptor_hashes() -> WorkflowTaskDescriptorHashes {
    WorkflowTaskDescriptorHashes {
        descriptor_hash: workflow_stable_hash(&workflow_task_pack_definition()),
        commands_hash: workflow_stable_hash(WORKFLOW_TASK_COMMANDS),
        permissions_hash: workflow_stable_hash(TASK_PERMISSION_SCOPES),
        providers_hash: workflow_stable_hash(TASK_PROVIDER_CLASSES),
        state_machine_hash: workflow_stable_hash(&[
            WorkflowTaskState::Draft,
            WorkflowTaskState::Queued,
            WorkflowTaskState::Claimed,
            WorkflowTaskState::Running,
            WorkflowTaskState::Review,
            WorkflowTaskState::Completed,
            WorkflowTaskState::Failed,
            WorkflowTaskState::Cancelled,
            WorkflowTaskState::Skipped,
        ]),
        retry_hash: workflow_stable_hash(&RetryPolicy {
            policy_ref: "retry:schema".into(),
            max_attempts: 3,
            backoff_ms: 1_000,
            retryable_codes: BTreeSet::from(["transient".into()]),
        }),
        lease_hash: workflow_stable_hash(&TaskLease {
            lease_ref: "lease:schema".into(),
            expires_at_epoch_ms: 1,
            ..Default::default()
        }),
        dependency_hash: workflow_stable_hash(&TaskDependency {
            dependency_ref: "dependency:schema".into(),
            required_state: WorkflowTaskState::Completed,
            blocking: true,
            ..Default::default()
        }),
    }
}
