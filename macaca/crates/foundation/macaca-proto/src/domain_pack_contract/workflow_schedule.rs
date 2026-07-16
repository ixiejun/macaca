use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::workflow_common::{
    define_workflow_command_wrappers, workflow_pack_definition, workflow_stable_hash,
    WorkflowCommandEnvelope, WorkflowError, WorkflowPackDescriptor, WorkflowPage,
    WorkflowProviderClass,
};

pub const WORKFLOW_SCHEDULE_PACK_ID: &str = "pack.workflow.schedule.v1";
pub const WORKFLOW_SCHEDULE_SERVICE_ID: &str = "service.workflow.schedule";

pub const WORKFLOW_SCHEDULE_COMMANDS: &[&str] = &[
    "workflow_schedule.create",
    "workflow_schedule.update",
    "workflow_schedule.pause",
    "workflow_schedule.resume",
    "workflow_schedule.delete",
    "workflow_schedule.inspect",
    "workflow_schedule.preview",
    "workflow_schedule.next_occurrences",
    "workflow_schedule.fire_due",
    "workflow_schedule.backfill",
    "workflow_schedule.cancel_trigger",
    "workflow_schedule.get_history",
    "workflow_schedule.snapshot",
    "workflow_schedule.inspect_provider",
];

const SCHEDULE_PERMISSION_SCOPES: &[&str] = &[
    "workflow.schedule.read",
    "workflow.schedule.write",
    "workflow.schedule.control",
    "workflow.schedule.fire",
    "workflow.schedule.backfill",
    "workflow.schedule.admin",
];

const DURABLE_METADATA: &[(&str, &str)] = &[
    ("recurrence", "true"),
    ("timezone_dst", "true"),
    ("trigger_history", "bounded"),
    ("raw_action_payloads_in_trace", "false"),
];
const REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_engine", "true"),
    ("backfill", "policy_bound"),
    ("provider_payloads_in_trace", "false"),
];
const PLUGIN_METADATA: &[(&str, &str)] =
    &[("plugin", "true"), ("recurrence_conformance", "required")];
const MOCK_METADATA: &[(&str, &str)] =
    &[("deterministic_clock", "true"), ("fixtures", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const SCHEDULE_PROVIDER_CLASSES: &[WorkflowProviderClass<'_>] = &[
    WorkflowProviderClass {
        provider_class: "durable-scheduler",
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

/// Build the schedule descriptor without binding recurrence or workflow-engine providers.
pub fn workflow_schedule_pack_definition() -> DomainPackDefinition {
    workflow_pack_definition(WorkflowPackDescriptor {
        pack_id: WORKFLOW_SCHEDULE_PACK_ID,
        child_change_id: "openspec:add-pack-workflow-schedule",
        docs_slug: "schedule",
        sdk_slug: "schedule",
        service_id: WORKFLOW_SCHEDULE_SERVICE_ID,
        commands: WORKFLOW_SCHEDULE_COMMANDS,
        permission_scopes: SCHEDULE_PERMISSION_SCOPES,
        provider_classes: SCHEDULE_PROVIDER_CLASSES,
        health_probe: "workflow_schedule.inspect_provider",
        unavailable_reason: "workflow_schedule_provider_not_installed",
        replay_schema: "workflow.schedule.replay.v1",
        data_classification: "workflow_schedule_reference_metadata",
        retention_policy: "schedule_recurrence_timezone_misfire_overlap_backfill_trigger_history_and_snapshot_metadata_by_reference",
        redaction_policy: "raw_action_payloads_prompts_provider_payloads_schedule_private_metadata_credentials_and_unbounded_trigger_histories_redacted",
        timeout_ms: 180_000,
        budget_units: 10,
        examples: &[
            "Declare `pack.workflow.schedule.v1` as optional until a schedule provider is installed.",
            "Use schedule, recurrence, trigger, and backfill references instead of raw action payloads.",
        ],
        migration_notes: &[
            "Schedule commands become callable only after an approved workflow schedule provider registers matching schemas.",
            "Calendar invites, foundation time, task execution, approval, review, delegation, and recovery remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowSchedule {
    pub schedule_ref: String,
    pub spec: WorkflowScheduleSpec,
    pub state: WorkflowScheduleState,
    pub version: String,
    pub next_trigger_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowScheduleSpec {
    pub spec_ref: String,
    pub recurrence: ScheduleRecurrence,
    pub timezone_policy: ScheduleTimezonePolicy,
    pub misfire_policy: ScheduleMisfirePolicy,
    pub overlap_policy: ScheduleOverlapPolicy,
    pub action_ref: String,
    pub jitter_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleRecurrence {
    pub recurrence_ref: String,
    pub kind: String,
    pub expression_ref: String,
    pub interval_ms: Option<u64>,
    pub rrule_ref: Option<String>,
    pub exclusion_set_ref: Option<String>,
}

impl ScheduleRecurrence {
    /// Validate recurrence shape without evaluating a calendar engine.
    pub fn has_declared_rule(&self) -> bool {
        !self.kind.is_empty()
            && (!self.expression_ref.is_empty()
                || self.interval_ms.is_some()
                || self.rrule_ref.is_some())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleTimezonePolicy {
    pub timezone_ref: String,
    pub dst_gap_strategy: String,
    pub dst_fold_strategy: String,
    pub local_time_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleMisfirePolicy {
    pub policy_ref: String,
    pub strategy: String,
    pub catchup_window_ms: u64,
    pub max_catchup_triggers: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleOverlapPolicy {
    pub policy_ref: String,
    pub strategy: String,
    pub concurrency_group_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleTriggerRecord {
    pub trigger_ref: String,
    pub schedule_ref: String,
    pub scheduled_epoch_ms: u64,
    pub logical_epoch_ms: u64,
    pub idempotency_key: String,
    pub action_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleBackfillRequest {
    pub backfill_ref: String,
    pub schedule_ref: String,
    pub start_epoch_ms: u64,
    pub end_epoch_ms: u64,
    pub max_triggers: u32,
    pub approval_ref: Option<String>,
}

impl ScheduleBackfillRequest {
    /// Backfills must be bounded before a provider can materialize triggers.
    pub fn is_bounded(&self) -> bool {
        self.end_epoch_ms >= self.start_epoch_ms && self.max_triggers > 0
    }
}

pub type WorkflowScheduleError = WorkflowError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowScheduleState {
    #[default]
    Draft,
    Active,
    Paused,
    Deleted,
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowScheduleResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    InvalidRecurrence,
    InvalidTimezone,
    DstUnresolved,
    MisfireBlocked,
    OverlapBlocked,
    BackfillTooLarge,
    SchedulePaused,
    TriggerConflict,
    QuotaExceeded,
    ProviderFailure,
    VersionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowScheduleResultEnvelope<T> {
    pub status: WorkflowScheduleResultStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<WorkflowPage<T>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkflowScheduleError>,
}

define_workflow_command_wrappers!(
    WorkflowScheduleCreateCommand,
    WorkflowScheduleUpdateCommand,
    WorkflowSchedulePauseCommand,
    WorkflowScheduleResumeCommand,
    WorkflowScheduleDeleteCommand,
    WorkflowScheduleInspectCommand,
    WorkflowSchedulePreviewCommand,
    WorkflowScheduleNextOccurrencesCommand,
    WorkflowScheduleFireDueCommand,
    WorkflowScheduleBackfillCommand,
    WorkflowScheduleCancelTriggerCommand,
    WorkflowScheduleGetHistoryCommand,
    WorkflowScheduleSnapshotCommand,
    WorkflowScheduleInspectProviderCommand,
);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowScheduleDescriptorHashes {
    pub descriptor_hash: String,
    pub commands_hash: String,
    pub permissions_hash: String,
    pub providers_hash: String,
    pub recurrence_hash: String,
    pub dst_hash: String,
    pub misfire_hash: String,
    pub backfill_hash: String,
}

pub fn workflow_schedule_descriptor_hashes() -> WorkflowScheduleDescriptorHashes {
    WorkflowScheduleDescriptorHashes {
        descriptor_hash: workflow_stable_hash(&workflow_schedule_pack_definition()),
        commands_hash: workflow_stable_hash(WORKFLOW_SCHEDULE_COMMANDS),
        permissions_hash: workflow_stable_hash(SCHEDULE_PERMISSION_SCOPES),
        providers_hash: workflow_stable_hash(SCHEDULE_PROVIDER_CLASSES),
        recurrence_hash: workflow_stable_hash(&ScheduleRecurrence {
            recurrence_ref: "recurrence:schema".into(),
            kind: "rrule".into(),
            expression_ref: "expression:bounded".into(),
            ..Default::default()
        }),
        dst_hash: workflow_stable_hash(&ScheduleTimezonePolicy {
            timezone_ref: "timezone:zone".into(),
            dst_gap_strategy: "reject".into(),
            dst_fold_strategy: "earlier".into(),
            local_time_required: true,
        }),
        misfire_hash: workflow_stable_hash(&ScheduleMisfirePolicy {
            policy_ref: "misfire:schema".into(),
            strategy: "skip".into(),
            catchup_window_ms: 60_000,
            max_catchup_triggers: 10,
        }),
        backfill_hash: workflow_stable_hash(&ScheduleBackfillRequest {
            backfill_ref: "backfill:schema".into(),
            max_triggers: 10,
            ..Default::default()
        }),
    }
}
