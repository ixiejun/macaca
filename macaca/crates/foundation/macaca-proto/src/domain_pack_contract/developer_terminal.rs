use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::developer_common::{
    define_developer_command_wrappers, developer_pack_definition, developer_stable_hash,
    DeveloperCommandEnvelope, DeveloperError, DeveloperPackDescriptor, DeveloperPage,
    DeveloperProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVELOPER_TERMINAL_PACK_ID: &str = "pack.developer.terminal.v1";
pub const DEVELOPER_TERMINAL_SERVICE_ID: &str = "service.developer.terminal";

pub const DEVELOPER_TERMINAL_COMMANDS: &[&str] = &[
    "terminal.inspect_provider",
    "terminal.plan_spawn",
    "terminal.spawn_request",
    "terminal.stream_output",
    "terminal.send_stdin",
    "terminal.resize",
    "terminal.inspect_process",
    "terminal.collect_exit",
    "terminal.cancel",
    "terminal.snapshot_workdir",
    "terminal.cleanup_session",
];

/// Sanitized lifecycle events for terminal service trace and replay evidence.
pub const DEVELOPER_TERMINAL_TRACE_EVENTS: &[&str] = &[
    "terminal_pack_declared",
    "terminal_admission_validated",
    "terminal_provider_inspected",
    "terminal_spawn_planned",
    "terminal_spawn_requested",
    "terminal_stream_read",
    "terminal_stdin_sent",
    "terminal_resized",
    "terminal_process_inspected",
    "terminal_exit_collected",
    "terminal_cancelled",
    "terminal_workdir_snapshotted",
    "terminal_session_cleaned",
    "terminal_policy_decision",
    "terminal_unavailable",
    "terminal_snapshot_recorded",
];

const TERMINAL_PERMISSION_SCOPES: &[&str] = &[
    "terminal.provider.inspect",
    "terminal.spawn",
    "terminal.stream.read",
    "terminal.stdin.write",
    "terminal.resize",
    "terminal.process.inspect",
    "terminal.exit.collect",
    "terminal.cancel",
    "terminal.workdir.snapshot",
    "terminal.session.cleanup",
];

const PROCESS_METADATA: &[(&str, &str)] = &[
    ("spawn", "plan_request_split"),
    ("pty", "optional"),
    ("raw_output_in_trace", "false"),
];
const STREAM_METADATA: &[(&str, &str)] =
    &[("streaming", "cursor_bounded"), ("stdin", "policy_bound")];
const SNAPSHOT_METADATA: &[(&str, &str)] =
    &[("snapshot", "handle_only"), ("raw_file_content", "false")];
const MOCK_METADATA: &[(&str, &str)] = &[
    ("deterministic", "true"),
    ("terminal_payloads", "synthetic"),
];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const TERMINAL_PROVIDER_CLASSES: &[DeveloperProviderClass<'_>] = &[
    DeveloperProviderClass {
        provider_class: "process-runtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PROCESS_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "stream-runtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: STREAM_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "snapshot-runtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SNAPSHOT_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MOCK_METADATA,
    },
    DeveloperProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: UNAVAILABLE_METADATA,
    },
];

/// Build the terminal descriptor without spawning processes or binding host shell APIs.
pub fn developer_terminal_pack_definition() -> DomainPackDefinition {
    developer_pack_definition(DeveloperPackDescriptor {
        pack_id: DEVELOPER_TERMINAL_PACK_ID,
        child_change_id: "openspec:add-pack-developer-terminal",
        docs_slug: "terminal",
        sdk_slug: "terminal",
        service_id: DEVELOPER_TERMINAL_SERVICE_ID,
        commands: DEVELOPER_TERMINAL_COMMANDS,
        permission_scopes: TERMINAL_PERMISSION_SCOPES,
        provider_classes: TERMINAL_PROVIDER_CLASSES,
        health_probe: "terminal.inspect_provider",
        unavailable_reason: "developer_terminal_provider_not_installed",
        replay_schema: "developer.terminal.replay.v1",
        data_classification: "developer_terminal_reference_metadata",
        retention_policy: "process_specs_spawn_plans_sessions_stream_cursors_exit_statuses_usage_and_snapshots_by_reference",
        redaction_policy: "raw_credentials_env_values_secret_material_file_content_terminal_output_and_provider_payloads_redacted",
        timeout_ms: 180_000,
        budget_units: 14,
        examples: &[
            "Declare `pack.developer.terminal.v1` as optional until a terminal/process provider is installed.",
            "Use process specs, spawn plans, session refs, stream cursors, and snapshot handles instead of raw output or host commands.",
        ],
        migration_notes: &[
            "Terminal commands become callable only after an approved terminal service provider registers matching schemas.",
            "Host process APIs, shells, PTYs, containers, streams, and cancellation strategies stay behind service adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalScope {
    pub scope_ref: String,
    pub workspace_ref: String,
    pub network_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalProviderCapability {
    pub provider_class: String,
    pub supports_pty: bool,
    pub supports_stdin: bool,
    pub supports_snapshot: bool,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalProcessSpec {
    pub spec_ref: String,
    pub command_hash: String,
    pub argument_vector_hash: String,
    pub env_policy: TerminalEnvironmentPolicy,
    pub workdir_scope: TerminalWorkdirScope,
}

impl TerminalProcessSpec {
    /// Validate plan metadata without spawning or inspecting a host process.
    pub fn is_policy_bound(&self) -> bool {
        !self.spec_ref.is_empty()
            && !self.command_hash.is_empty()
            && !self.env_policy.policy_ref.is_empty()
            && !self.workdir_scope.scope_ref.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalEnvironmentPolicy {
    pub policy_ref: String,
    pub secret_refs_only: bool,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalWorkdirScope {
    pub scope_ref: String,
    pub workspace_ref: String,
    pub write_allowed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalPtyProfile {
    pub profile_ref: String,
    pub rows: u32,
    pub cols: u32,
    pub supports_resize: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSpawnPlan {
    pub plan_ref: String,
    pub spec_ref: String,
    pub resource_budget_ref: String,
    pub approval_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSession {
    pub session_ref: String,
    pub plan_ref: String,
    pub state: String,
    pub process_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalStreamCursor {
    pub cursor_ref: String,
    pub session_ref: String,
    pub stream_kind: String,
    pub offset: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalOutputChunk {
    pub chunk_ref: String,
    pub cursor_ref: String,
    pub redacted_output_ref: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalStdinFrame {
    pub frame_ref: String,
    pub session_ref: String,
    pub input_hash: String,
    pub policy_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSignalIntent {
    pub intent_ref: String,
    pub session_ref: String,
    pub signal_kind: String,
    pub escalation: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalExitStatus {
    pub status_ref: String,
    pub session_ref: String,
    pub exit_code: Option<i32>,
    pub signal_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalResourceUsage {
    pub usage_ref: String,
    pub session_ref: String,
    pub duration_ms: u64,
    pub output_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSnapshotHandle {
    pub snapshot_ref: String,
    pub session_ref: String,
    pub artifact_ref: String,
    pub redaction_profile: String,
}

define_developer_command_wrappers!(
    TerminalInspectProviderCommand,
    TerminalPlanSpawnCommand,
    TerminalSpawnRequestCommand,
    TerminalStreamOutputCommand,
    TerminalSendStdinCommand,
    TerminalResizeCommand,
    TerminalInspectProcessCommand,
    TerminalCollectExitCommand,
    TerminalCancelCommand,
    TerminalSnapshotWorkdirCommand,
    TerminalCleanupSessionCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalResultStatus {
    Success,
    Streaming,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    NotRunning,
    StaleHandle,
    InvalidCommand,
    InvalidWorkdir,
    InvalidEnv,
    StreamTruncated,
    QuotaExceeded,
    Timeout,
    Cancelled,
    ApprovalRequired,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalResultEnvelope<T> {
    pub status: TerminalResultStatus,
    pub data: Option<T>,
    pub page: Option<DeveloperPage<T>>,
    pub error: Option<DeveloperError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub process_spec_hash: String,
    pub spawn_plan_hash: String,
    pub cursor_hash: String,
    pub exit_hash: String,
    pub snapshot_hash: String,
}

pub fn developer_terminal_descriptor_hashes() -> TerminalDescriptorHashes {
    let env = TerminalEnvironmentPolicy {
        policy_ref: "env-policy".into(),
        secret_refs_only: true,
        redaction_profile: "terminal-env-redaction-v1".into(),
    };
    let workdir = TerminalWorkdirScope {
        scope_ref: "workdir".into(),
        workspace_ref: "workspace".into(),
        write_allowed: false,
    };
    TerminalDescriptorHashes {
        command_schema_hash: terminal_stable_hash(&DEVELOPER_TERMINAL_COMMANDS),
        result_schema_hash: terminal_stable_hash(&TerminalResultStatus::Success),
        descriptor_hash: terminal_stable_hash(&developer_terminal_pack_definition()),
        provider_capability_hash: terminal_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        process_spec_hash: terminal_stable_hash(&TerminalProcessSpec {
            spec_ref: "spec".into(),
            command_hash: "command".into(),
            argument_vector_hash: "argv".into(),
            env_policy: env,
            workdir_scope: workdir,
        }),
        spawn_plan_hash: terminal_stable_hash(&TerminalSpawnPlan {
            plan_ref: "spawn".into(),
            spec_ref: "spec".into(),
            resource_budget_ref: "budget".into(),
            approval_required: false,
        }),
        cursor_hash: terminal_stable_hash(&TerminalStreamCursor {
            cursor_ref: "cursor".into(),
            session_ref: "session".into(),
            stream_kind: "stdout".into(),
            offset: 0,
        }),
        exit_hash: terminal_stable_hash(&TerminalExitStatus {
            status_ref: "exit".into(),
            session_ref: "session".into(),
            exit_code: Some(0),
            signal_ref: None,
        }),
        snapshot_hash: terminal_stable_hash(&TerminalSnapshotHandle {
            snapshot_ref: "snapshot".into(),
            session_ref: "session".into(),
            artifact_ref: "artifact".into(),
            redaction_profile: "terminal-snapshot-redaction-v1".into(),
        }),
    }
}

pub fn terminal_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    developer_stable_hash(value)
}
