use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::workflow_common::{
    define_workflow_command_wrappers, workflow_pack_definition, workflow_stable_hash,
    WorkflowCommandEnvelope, WorkflowError, WorkflowPackDescriptor, WorkflowPage,
    WorkflowProviderClass,
};

pub const WORKFLOW_RECOVERY_PACK_ID: &str = "pack.workflow.recovery.v1";
pub const WORKFLOW_RECOVERY_SERVICE_ID: &str = "service.workflow.recovery";

pub const WORKFLOW_RECOVERY_COMMANDS: &[&str] = &[
    "recovery.classify_failure",
    "recovery.list_recovery_points",
    "recovery.retry",
    "recovery.repair_state",
    "recovery.resume",
    "recovery.export_replay",
    "recovery.build_plan",
    "recovery.apply_compensation",
    "recovery.terminalize",
    "recovery.inspect_provider",
];

const RECOVERY_PERMISSION_SCOPES: &[&str] = &[
    "workflow.recovery.read",
    "workflow.recovery.repair",
    "workflow.recovery.resume",
    "workflow.recovery.retry",
    "workflow.recovery.compensate",
    "workflow.recovery.export",
    "workflow.recovery.admin",
];

const DURABLE_METADATA: &[(&str, &str)] = &[
    ("recovery_points", "true"),
    ("retry_budget", "true"),
    ("raw_checkpoint_bytes_in_trace", "false"),
];
const REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_workflow", "true"),
    ("replay_export", "sanitized"),
    ("provider_payloads_in_trace", "false"),
];
const PLUGIN_METADATA: &[(&str, &str)] =
    &[("plugin", "true"), ("recovery_conformance", "required")];
const MOCK_METADATA: &[(&str, &str)] = &[("deterministic", "true"), ("failures", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const RECOVERY_PROVIDER_CLASSES: &[WorkflowProviderClass<'_>] = &[
    WorkflowProviderClass {
        provider_class: "durable-recovery",
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

/// Build the recovery descriptor without binding a concrete workflow recovery engine.
pub fn workflow_recovery_pack_definition() -> DomainPackDefinition {
    workflow_pack_definition(WorkflowPackDescriptor {
        pack_id: WORKFLOW_RECOVERY_PACK_ID,
        child_change_id: "openspec:add-pack-workflow-recovery",
        docs_slug: "recovery",
        sdk_slug: "recovery",
        service_id: WORKFLOW_RECOVERY_SERVICE_ID,
        commands: WORKFLOW_RECOVERY_COMMANDS,
        permission_scopes: RECOVERY_PERMISSION_SCOPES,
        provider_classes: RECOVERY_PROVIDER_CLASSES,
        health_probe: "recovery.inspect_provider",
        unavailable_reason: "workflow_recovery_provider_not_installed",
        replay_schema: "workflow.recovery.replay.v1",
        data_classification: "workflow_recovery_reference_metadata",
        retention_policy: "failure_recovery_point_retry_plan_repair_compensation_resume_and_replay_metadata_by_reference",
        redaction_policy: "raw_checkpoint_bytes_prompts_provider_payloads_credentials_replay_payloads_package_bytes_and_unbounded_logs_redacted",
        timeout_ms: 180_000,
        budget_units: 10,
        examples: &[
            "Declare `pack.workflow.recovery.v1` as optional until a recovery provider is installed.",
            "Use failure, recovery point, repair, compensation, resume, and replay references instead of raw checkpoint bytes.",
        ],
        migration_notes: &[
            "Recovery commands become callable only after an approved recovery service provider registers matching schemas.",
            "Application-specific repair semantics stay with the owning application or service; this pack carries generic recovery evidence.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureRecord {
    pub failure_ref: String,
    pub origin_service_ref: String,
    pub failure_class: String,
    pub reason_code: String,
    pub retryable: bool,
    pub trace_ref: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPoint {
    pub point_ref: String,
    pub owner_service_ref: String,
    pub checkpoint_ref: String,
    pub integrity_hash: String,
    pub compatibility_version: String,
    pub replay_cursor: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub policy_ref: String,
    pub max_attempts: u32,
    pub backoff_ms: u64,
    pub terminal_on_exhaustion: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPlan {
    pub plan_ref: String,
    pub failure_ref: String,
    pub recovery_point_ref: Option<String>,
    pub actions: Vec<RepairAction>,
    pub retry_policy: Option<RetryPolicy>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairAction {
    pub action_ref: String,
    pub action_kind: String,
    pub target_ref: String,
    pub policy_ref: String,
    pub compensation_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompensationRef {
    pub compensation_ref: String,
    pub original_action_ref: String,
    pub order_index: u32,
    pub status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumePlan {
    pub resume_ref: String,
    pub recovery_point_ref: String,
    pub target_service_ref: String,
    pub replay_cursor: String,
    pub compatibility_checked: bool,
}

impl ResumePlan {
    /// Resume is valid only after compatibility has been checked explicitly.
    pub fn can_resume(&self) -> bool {
        self.compatibility_checked && !self.recovery_point_ref.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayExport {
    pub export_ref: String,
    pub trace_ref: String,
    pub redacted_bundle_ref: String,
    pub event_count: u64,
    pub payloads_redacted: bool,
}

pub type WorkflowRecoveryError = WorkflowError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRecoveryResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    Failure,
    CorruptedCheckpoint,
    RetryBudgetExhausted,
    IncompatibleCheckpoint,
    Terminalized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRecoveryResultEnvelope<T> {
    pub status: WorkflowRecoveryResultStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<WorkflowPage<T>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkflowRecoveryError>,
}

define_workflow_command_wrappers!(
    RecoveryClassifyFailureCommand,
    RecoveryListRecoveryPointsCommand,
    RecoveryRetryCommand,
    RecoveryRepairStateCommand,
    RecoveryResumeCommand,
    RecoveryExportReplayCommand,
    RecoveryBuildPlanCommand,
    RecoveryApplyCompensationCommand,
    RecoveryTerminalizeCommand,
    RecoveryInspectProviderCommand,
);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRecoveryDescriptorHashes {
    pub descriptor_hash: String,
    pub commands_hash: String,
    pub permissions_hash: String,
    pub providers_hash: String,
    pub failure_hash: String,
    pub recovery_point_hash: String,
    pub replay_export_hash: String,
}

pub fn workflow_recovery_descriptor_hashes() -> WorkflowRecoveryDescriptorHashes {
    WorkflowRecoveryDescriptorHashes {
        descriptor_hash: workflow_stable_hash(&workflow_recovery_pack_definition()),
        commands_hash: workflow_stable_hash(WORKFLOW_RECOVERY_COMMANDS),
        permissions_hash: workflow_stable_hash(RECOVERY_PERMISSION_SCOPES),
        providers_hash: workflow_stable_hash(RECOVERY_PROVIDER_CLASSES),
        failure_hash: workflow_stable_hash(&FailureRecord {
            failure_ref: "failure:record".into(),
            origin_service_ref: "service:origin".into(),
            failure_class: "transient".into(),
            reason_code: "provider_unavailable".into(),
            retryable: true,
            ..Default::default()
        }),
        recovery_point_hash: workflow_stable_hash(&RecoveryPoint {
            point_ref: "recovery:point".into(),
            owner_service_ref: "service:owner".into(),
            checkpoint_ref: "checkpoint:ref".into(),
            integrity_hash: "integrity:hash".into(),
            compatibility_version: "v1".into(),
            replay_cursor: "cursor:1".into(),
        }),
        replay_export_hash: workflow_stable_hash(&ReplayExport {
            export_ref: "replay:export".into(),
            trace_ref: "trace:ref".into(),
            redacted_bundle_ref: "bundle:redacted".into(),
            event_count: 1,
            payloads_redacted: true,
        }),
    }
}
