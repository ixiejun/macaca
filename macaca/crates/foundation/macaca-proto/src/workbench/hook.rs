//! Provider-neutral contracts for `service.hook`.
//!
//! The hook lifecycle service owns generic pre/post execution interception for
//! tools, turns, sessions, and application lifecycle events.  Hook descriptors
//! are catalog records, not executable code.  Runtime-host providers decide how
//! local, plugin, script, WASM, or remote adapters are invoked, and every result
//! is bounded so model-visible mutations remain trace-linked and auditable.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BoundedSummary, WorkbenchServiceDescriptor};

pub const SERVICE_ID: &str = "service.hook";
pub const CAPABILITY_ID: &str = "capability.hook";
pub const CATALOG_LIST_COMMAND: &str = "hook.catalog.list";
pub const POLICY_RESOLVE_COMMAND: &str = "hook.policy.resolve";
pub const PRE_TOOL_RUN_COMMAND: &str = "hook.pre_tool.run";
pub const POST_TOOL_RUN_COMMAND: &str = "hook.post_tool.run";
pub const SESSION_RUN_COMMAND: &str = "hook.session.run";
pub const TURN_RUN_COMMAND: &str = "hook.turn.run";
pub const RESULT_AUDIT_COMMAND: &str = "hook.result.audit";

pub const COMMANDS: &[&str] = &[
    CATALOG_LIST_COMMAND,
    POLICY_RESOLVE_COMMAND,
    PRE_TOOL_RUN_COMMAND,
    POST_TOOL_RUN_COMMAND,
    SESSION_RUN_COMMAND,
    TURN_RUN_COMMAND,
    RESULT_AUDIT_COMMAND,
];

/// Hook execution stage used for deterministic chain selection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookStage {
    PreTool,
    PostTool,
    Session,
    Turn,
    Application,
}

/// Administrative source class for managed-only policy filtering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookSourceClass {
    Managed,
    User,
    Project,
    Session,
    Plugin,
}

/// Adapter family that would execute a hook descriptor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookAdapterKind {
    Local,
    Mock,
    Plugin,
    Script,
    Wasm,
    Remote,
    Unavailable,
}

/// Bounded policy outcome one hook may contribute to a lifecycle chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookAction {
    Continue,
    RewriteInput,
    Block,
    AddContext,
    RequestStop,
    ReplaceOutput,
}

/// Scope constraints attached to descriptors and run commands.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HookScope {
    pub application_id: Option<String>,
    pub session_id: Option<String>,
    pub thread_ref: Option<String>,
    pub turn_ref: Option<String>,
    pub tool_family: Option<String>,
    pub command_name: Option<String>,
}

/// Catalog descriptor for one managed lifecycle hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookDescriptor {
    pub hook_id: String,
    pub stage: HookStage,
    pub source_class: HookSourceClass,
    pub adapter_kind: HookAdapterKind,
    pub priority: i32,
    pub managed: bool,
    pub enabled: bool,
    pub scope: HookScope,
    pub default_action: HookAction,
    pub feedback: Option<BoundedSummary>,
    pub trace_schema: String,
    pub metadata: BTreeMap<String, String>,
}

/// Query filters for bounded catalog reads.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HookCatalogList {
    pub stage: Option<HookStage>,
    pub source_class: Option<HookSourceClass>,
    pub include_disabled: bool,
    pub limit: Option<usize>,
}

/// Executable Specification input used before hook dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookPolicyResolve {
    pub scope: HookScope,
    pub stage: HookStage,
    pub managed_only: bool,
    pub include_unavailable_adapters: bool,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookPolicyResolution {
    pub active_hooks: Vec<HookDescriptor>,
    pub ignored_hooks: Vec<HookDescriptor>,
    pub managed_only: bool,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookPreToolRun {
    pub scope: HookScope,
    pub managed_only: bool,
    pub input_summary: Option<BoundedSummary>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookPostToolRun {
    pub scope: HookScope,
    pub managed_only: bool,
    pub output_summary: Option<BoundedSummary>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookLifecycleRun {
    pub scope: HookScope,
    pub stage: HookStage,
    pub managed_only: bool,
    pub lifecycle_summary: Option<BoundedSummary>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookResultAuditRead {
    pub audit_ref: String,
}

/// Per-hook execution record.  Feedback may be model-visible only through this
/// bounded summary and the trace/audit refs attached to the aggregate result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookOutcome {
    pub hook_id: String,
    pub source_class: HookSourceClass,
    pub stage: HookStage,
    pub action: HookAction,
    pub status: String,
    pub duration_ms: u64,
    pub feedback: Option<BoundedSummary>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookRunResult {
    pub decision: HookAction,
    pub outcomes: Vec<HookOutcome>,
    pub rewritten_input: Option<BoundedSummary>,
    pub additional_context: Option<BoundedSummary>,
    pub replacement_output: Option<BoundedSummary>,
    pub audit_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookAuditRecord {
    pub audit_ref: String,
    pub stage: HookStage,
    pub decision: HookAction,
    pub hook_ids: Vec<String>,
    pub trace_id: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookEvent {
    pub event_ref: String,
    pub audit_ref: String,
    pub stage: HookStage,
    pub decision: HookAction,
    pub trace_id: String,
    pub emitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookServiceResponse {
    Catalog {
        hooks: Vec<HookDescriptor>,
        truncated: bool,
    },
    Policy(HookPolicyResolution),
    Run(HookRunResult),
    Audit(HookAuditRecord),
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.hook.events.v1",
        "service.hook.audit.v1",
        "service.hook.snapshot.v1",
    )
}
