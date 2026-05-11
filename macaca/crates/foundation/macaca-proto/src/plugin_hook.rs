//! Protocol-level Plugin Hook Bus v1 contracts.
//!
//! Hook contracts are intentionally data-only.  They let plugins subscribe to
//! lifecycle and execution events without receiving runtime objects, secrets,
//! raw credentials, private keys, package bytes, or unbounded prompt/memory
//! bodies.  Runtime-host owns hook ordering, timeout handling, failure policy,
//! trace/audit emission, and future host proxy dispatch.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{KernelServiceId, PluginId, ServiceScope, TraceContext, TraceSchemaRef};

/// Stable service id used for the Plugin Hook Bus service.
pub const PLUGIN_HOOK_BUS_SERVICE_ID: &str = "macaca.plugin.hook_bus";

/// Command names accepted by the Plugin Hook Bus service.
pub const PLUGIN_HOOK_REGISTER_COMMAND: &str = "plugin.hook.register";
pub const PLUGIN_HOOK_DEACTIVATE_COMMAND: &str = "plugin.hook.deactivate";
pub const PLUGIN_HOOK_QUERY_COMMAND: &str = "plugin.hook.query";
pub const PLUGIN_HOOK_INVOKE_COMMAND: &str = "plugin.hook.invoke";
pub const PLUGIN_HOOK_SNAPSHOT_COMMAND: &str = "plugin.hook.snapshot";

/// Return the canonical service id as a typed kernel service id.
pub fn plugin_hook_bus_service_id() -> KernelServiceId {
    KernelServiceId::new(PLUGIN_HOOK_BUS_SERVICE_ID)
}

/// Stable name for one OS hook point.
///
/// The closed variants cover the initial Hook Bus v1 OS surface.  `Custom`
/// keeps the protocol extensible for future services without forcing kernel or
/// protocol source edits for every optional module.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHookName {
    BeforeAgentStart,
    AfterAgentEnd,
    BeforePromptBuild,
    AfterContextAssemble,
    BeforeToolCall,
    AfterToolCall,
    BeforeLlmCall,
    AfterLlmCall,
    BeforeMemoryIngest,
    AfterMemoryIngest,
    BeforeGatewayDispatch,
    AfterGatewaySend,
    BeforeApprovalRequest,
    AfterApprovalResponse,
    SessionStart,
    SessionEnd,
    TaskStarted,
    TaskCompleted,
    ApplicationStart,
    ApplicationStop,
    Custom(String),
}

impl fmt::Display for PluginHookName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeforeAgentStart => f.write_str("before_agent_start"),
            Self::AfterAgentEnd => f.write_str("after_agent_end"),
            Self::BeforePromptBuild => f.write_str("before_prompt_build"),
            Self::AfterContextAssemble => f.write_str("after_context_assemble"),
            Self::BeforeToolCall => f.write_str("before_tool_call"),
            Self::AfterToolCall => f.write_str("after_tool_call"),
            Self::BeforeLlmCall => f.write_str("before_llm_call"),
            Self::AfterLlmCall => f.write_str("after_llm_call"),
            Self::BeforeMemoryIngest => f.write_str("before_memory_ingest"),
            Self::AfterMemoryIngest => f.write_str("after_memory_ingest"),
            Self::BeforeGatewayDispatch => f.write_str("before_gateway_dispatch"),
            Self::AfterGatewaySend => f.write_str("after_gateway_send"),
            Self::BeforeApprovalRequest => f.write_str("before_approval_request"),
            Self::AfterApprovalResponse => f.write_str("after_approval_response"),
            Self::SessionStart => f.write_str("session_start"),
            Self::SessionEnd => f.write_str("session_end"),
            Self::TaskStarted => f.write_str("task_started"),
            Self::TaskCompleted => f.write_str("task_completed"),
            Self::ApplicationStart => f.write_str("application_start"),
            Self::ApplicationStop => f.write_str("application_stop"),
            Self::Custom(value) => f.write_str(value),
        }
    }
}

/// Safety semantics attached to a hook subscription.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHookKind {
    Observer,
    Mutating,
    Blocking,
    Approval,
}

impl fmt::Display for PluginHookKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Observer => f.write_str("observer"),
            Self::Mutating => f.write_str("mutating"),
            Self::Blocking => f.write_str("blocking"),
            Self::Approval => f.write_str("approval"),
        }
    }
}

/// Failure behavior used when a hook times out, returns an invalid result, or
/// reaches a missing execution host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHookFailurePolicy {
    FailOpen,
    FailClosed,
    RequireApproval,
}

/// Timeout strategy attached to a hook descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHookTimeoutPolicy {
    Default,
    Millis(u64),
    Disabled,
}

impl PluginHookTimeoutPolicy {
    /// Convert a policy into a bounded millisecond value used by hosts.
    pub fn as_millis(&self, default_millis: u64) -> Option<u64> {
        match self {
            Self::Default => Some(default_millis),
            Self::Millis(value) => Some(*value),
            Self::Disabled => None,
        }
    }
}

/// Result schema metadata declared by a hook descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHookSchema {
    pub name: String,
    pub schema_ref: Option<String>,
    pub inline_schema: Option<serde_json::Value>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginHookSchema {
    /// Build a schema descriptor from a stable name and optional reference.
    pub fn referenced(name: impl Into<String>, schema_ref: impl Into<String>) -> Self {
        Self {
            name: name.into().trim().to_string(),
            schema_ref: Some(schema_ref.into().trim().to_string()),
            inline_schema: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Hook descriptor registered by one plugin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHookDescriptor {
    pub plugin_id: PluginId,
    pub hook_name: PluginHookName,
    pub kind: PluginHookKind,
    pub priority: i32,
    pub timeout_policy: PluginHookTimeoutPolicy,
    pub failure_policy: PluginHookFailurePolicy,
    pub required_permissions: Vec<String>,
    pub resource_hints: Vec<String>,
    pub scopes: Vec<ServiceScope>,
    pub input_schema: Option<PluginHookSchema>,
    pub result_schema: Option<PluginHookSchema>,
    pub trace_schema: TraceSchemaRef,
    pub metadata: BTreeMap<String, String>,
}

impl PluginHookDescriptor {
    /// Build an observer descriptor with fail-open semantics.
    pub fn observer(
        plugin_id: PluginId,
        hook_name: PluginHookName,
        priority: i32,
        trace_schema: TraceSchemaRef,
    ) -> Self {
        Self {
            plugin_id,
            hook_name,
            kind: PluginHookKind::Observer,
            priority,
            timeout_policy: PluginHookTimeoutPolicy::Default,
            failure_policy: PluginHookFailurePolicy::FailOpen,
            required_permissions: Vec::new(),
            resource_hints: Vec::new(),
            scopes: vec![ServiceScope::Global],
            input_schema: None,
            result_schema: None,
            trace_schema,
            metadata: BTreeMap::new(),
        }
    }
}

/// Active ownership record stored by the hook registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHookRegistration {
    pub plugin_id: PluginId,
    pub hook_name: PluginHookName,
    pub kind: PluginHookKind,
    pub active: bool,
    pub registered_at: DateTime<Utc>,
    pub descriptor: PluginHookDescriptor,
}

/// Minimal and bounded context delivered to a hook invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHookContext {
    pub application_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
    pub task_id: Option<String>,
    pub scope: ServiceScope,
    pub metadata: BTreeMap<String, String>,
}

impl PluginHookContext {
    /// Build a global context for integration points that do not have richer
    /// scoped identity yet.
    pub fn global() -> Self {
        Self {
            application_id: None,
            session_id: None,
            agent_name: None,
            task_id: None,
            scope: ServiceScope::Global,
            metadata: BTreeMap::new(),
        }
    }
}

/// Command envelope for invoking one hook point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHookInvokeCommand {
    pub hook_name: PluginHookName,
    pub expected_kind: Option<PluginHookKind>,
    pub context: PluginHookContext,
    pub payload: serde_json::Value,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

/// Command for registering hook descriptors owned by one plugin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHookRegisterCommand {
    pub plugin_id: PluginId,
    pub descriptors: Vec<PluginHookDescriptor>,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

/// Command for deactivating all hook subscriptions owned by one plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHookDeactivateCommand {
    pub plugin_id: PluginId,
    pub trace: TraceContext,
    pub reason: String,
}

/// Query command for deterministic hook lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHookQueryCommand {
    pub hook_name: Option<PluginHookName>,
    pub plugin_id: Option<PluginId>,
    pub include_inactive: bool,
    pub trace: TraceContext,
}

/// Snapshot command used by diagnostics and tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHookSnapshotCommand {
    pub trace: TraceContext,
}

/// Blocking decision produced by a blocking or approval hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHookDecision {
    Allow,
    Block,
    RequireApproval,
    Noop,
}

/// Structured hook output after schema and policy validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHookOutcome {
    pub plugin_id: PluginId,
    pub hook_name: PluginHookName,
    pub kind: PluginHookKind,
    pub status: String,
    pub decision: PluginHookDecision,
    pub mutation: Option<serde_json::Value>,
    pub approval: Option<serde_json::Value>,
    pub error_code: Option<String>,
    pub duration_ms: u64,
    pub metadata: BTreeMap<String, String>,
}

/// Aggregated result returned to the owning service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHookInvokeResult {
    pub hook_name: PluginHookName,
    pub decision: PluginHookDecision,
    pub outcomes: Vec<PluginHookOutcome>,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

/// Deterministic registry snapshot used for diagnostics and tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHookRegistrySnapshot {
    pub generated_at: DateTime<Utc>,
    pub hooks: Vec<PluginHookRegistration>,
}

/// Audit event emitted by hook registration and invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginHookEvent {
    pub plugin_id: Option<PluginId>,
    pub hook_name: Option<PluginHookName>,
    pub kind: Option<PluginHookKind>,
    pub operation: String,
    pub decision: String,
    pub trace: TraceContext,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub error_code: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl PluginHookEvent {
    /// Build an audit-safe event without embedding hook payloads, secrets, or
    /// provider configuration values.
    pub fn new(
        plugin_id: Option<PluginId>,
        hook_name: Option<PluginHookName>,
        kind: Option<PluginHookKind>,
        operation: impl Into<String>,
        decision: impl Into<String>,
        trace: TraceContext,
    ) -> Self {
        Self {
            plugin_id,
            hook_name,
            kind,
            operation: operation.into(),
            decision: decision.into(),
            trace,
            timestamp: Utc::now(),
            duration_ms: None,
            error_code: None,
            metadata: BTreeMap::new(),
        }
    }
}
