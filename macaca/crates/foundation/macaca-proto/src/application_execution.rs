//! Provider-neutral application execution protocol contracts.
//!
//! This module is the stable data contract for Macaca's application execution
//! protocol platform.  It intentionally contains only serializable DTOs,
//! constants, and tiny validation helpers: no runtime providers, no web-shell
//! state, no application package code, and no business workflow logic.  The
//! contracts let Macaca-hosted runtimes, application-owned backends, and remote
//! agents participate in one durable session/event/control protocol while the
//! operating system keeps trace, audit, policy, replay, and provider
//! replacement semantics generic.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ApplicationId, CapabilityId, KernelServiceId, MacacaError, MacacaResult, TraceContext,
};

pub mod gateway;
pub mod provider;
pub mod state;

pub use gateway::*;
pub use provider::*;
pub use state::*;

/// Stable service id for the application execution protocol platform.
pub const APPLICATION_EXECUTION_SERVICE_ID: &str = "service.application_execution";

/// Start or resume one durable application execution session.
pub const APPLICATION_EXECUTION_START_COMMAND: &str = "application_execution.start";

/// Deliver a cancel, approval, pause, resume, retry, or injected input command.
pub const APPLICATION_EXECUTION_CONTROL_COMMAND: &str = "application_execution.control";

/// Replay persisted execution events from a cursor or from the beginning.
pub const APPLICATION_EXECUTION_REPLAY_COMMAND: &str = "application_execution.replay";

/// Build the current execution state by reducing persisted events.
pub const APPLICATION_EXECUTION_CURRENT_STATE_COMMAND: &str = "application_execution.current_state";

/// Inspect provider health without exposing concrete provider internals.
pub const APPLICATION_EXECUTION_PROVIDER_HEALTH_COMMAND: &str =
    "application_execution.provider_health";

/// Return a bounded diagnostic snapshot for service and provider state.
pub const APPLICATION_EXECUTION_SNAPSHOT_COMMAND: &str = "application_execution.snapshot";

/// Append one provider-reported event through gateway ingress.
pub const APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND: &str =
    "application_execution.gateway.append_event";

/// Report provider heartbeat through gateway ingress.
pub const APPLICATION_EXECUTION_GATEWAY_HEARTBEAT_COMMAND: &str =
    "application_execution.gateway.heartbeat";

/// Report a provider snapshot through gateway ingress.
pub const APPLICATION_EXECUTION_GATEWAY_SNAPSHOT_COMMAND: &str =
    "application_execution.gateway.snapshot";

/// Request an approval decision through gateway ingress.
pub const APPLICATION_EXECUTION_GATEWAY_APPROVAL_COMMAND: &str =
    "application_execution.gateway.request_approval";

/// Report terminal success through gateway ingress.
pub const APPLICATION_EXECUTION_GATEWAY_COMPLETION_COMMAND: &str =
    "application_execution.gateway.completion";

/// Report terminal failure through gateway ingress.
pub const APPLICATION_EXECUTION_GATEWAY_FAILURE_COMMAND: &str =
    "application_execution.gateway.failure";

/// Provider strategy selected for one application execution.
///
/// These variants are deliberately protocol-level strategies rather than
/// concrete implementation names.  Routing code may select among them through
/// manifest metadata, policy, health, and capability matching, but generic OS
/// code must not branch on application names or business domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ApplicationExecutionProviderKind {
    MacacaHosted,
    ExternalAppBackend,
    RemoteAgent,
    Unavailable,
}

/// Durable lifecycle state for one execution run.
///
/// The state model is explicit so replay, diagnostics, and control delivery can
/// be audited without reconstructing state from web-shell rendering behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ApplicationExecutionLifecycleState {
    Accepted,
    AssigningProvider,
    Running,
    WaitingForApproval,
    Paused,
    Resuming,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl ApplicationExecutionLifecycleState {
    /// Return whether the state is terminal and no provider work should remain.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// Canonical event families appended to the durable execution EventLog.
///
/// The variants name protocol facts, not UI widgets or application workflows.
/// Providers may attach sanitized payload data in the event envelope, while
/// event consumers can project a current state without knowing provider code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ApplicationExecutionEventType {
    SessionStarted,
    ExecutionAccepted,
    ProviderAssigned,
    ProviderHeartbeat,
    ProviderSnapshot,
    LlmRequested,
    LlmCompleted,
    ToolCallRequested,
    ToolCallDispatched,
    ToolCallCompleted,
    ApprovalRequested,
    ApprovalResolved,
    CheckpointCreated,
    ControlRequested,
    ControlDelivered,
    ControlCompleted,
    ExecutionCompleted,
    ExecutionFailed,
    ExecutionCancelled,
}

/// Control command that can be routed to any application execution provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ApplicationExecutionControlKind {
    Cancel,
    Approve,
    Reject,
    Pause,
    Resume,
    Retry,
    InjectInput,
}

/// Stable result status used by start/control/gateway commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ApplicationExecutionCommandStatus {
    Accepted,
    Delivered,
    Completed,
    Unavailable,
    Disabled,
    Denied,
    Unsupported,
    InvalidSchema,
    InvalidState,
    Duplicate,
    StaleLease,
    Timeout,
    ProviderFailed,
    PolicyDenied,
    ResourceDenied,
    EntitlementDenied,
    SanitizationFailed,
    Failed,
}

/// Stable error payload for application execution protocol failures.
///
/// The error is data, not an exception type.  Service/runtime adapters can
/// return it across process, HTTP, SDK, WASM, or remote-agent boundaries while
/// preserving audit-safe context and retryability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationExecutionError {
    pub code: ApplicationExecutionCommandStatus,
    pub layer: String,
    pub operation: String,
    pub application_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub run_id: Option<String>,
    pub provider_id: Option<String>,
    pub provider_kind: Option<ApplicationExecutionProviderKind>,
    pub trace_id: Option<String>,
    pub reason: String,
    pub retryable: bool,
}

impl ApplicationExecutionError {
    /// Build the Null Object unavailable error used when the service/provider is absent.
    pub fn unavailable(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            code: ApplicationExecutionCommandStatus::Unavailable,
            layer: "service.application_execution".into(),
            operation: operation.into(),
            application_id: None,
            session_id: None,
            run_id: None,
            provider_id: None,
            provider_kind: Some(ApplicationExecutionProviderKind::Unavailable),
            trace_id: None,
            reason: reason.into(),
            retryable: false,
        }
    }
}

/// Reference to payload material stored outside inline protocol surfaces.
///
/// EventLog, trace, audit, diagnostics, and realtime payloads should carry this
/// safe reference when raw material is too large or too sensitive to inline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationExecutionPayloadRef {
    pub id: String,
    pub media_type: String,
    pub sha256: Option<String>,
    pub byte_len: u64,
    pub redaction_profile: String,
}

/// Bounded provider-neutral payload used by commands and events.
///
/// `summary` is safe to log after sanitization.  `data` is for small structured
/// values that have already passed a sanitizer.  Large or sensitive content
/// must move to `payload_ref`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationExecutionPayload {
    pub summary: String,
    pub data: Option<serde_json::Value>,
    pub payload_ref: Option<ApplicationExecutionPayloadRef>,
    pub truncated: bool,
}

impl ApplicationExecutionPayload {
    /// Build a small sanitized summary payload with no raw provider material.
    pub fn summary(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            data: None,
            payload_ref: None,
            truncated: false,
        }
    }
}

/// Scope shared by all application execution protocol commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationExecutionScope {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub run_id: String,
    pub tenant_id: Option<String>,
    pub actor: String,
}

impl ApplicationExecutionScope {
    /// Create a scope and reject blank session/run/actor identifiers early.
    pub fn new(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        run_id: impl Into<String>,
        actor: impl Into<String>,
    ) -> MacacaResult<Self> {
        let scope = Self {
            application_id,
            session_id: normalize_required("session_id", session_id)?,
            run_id: normalize_required("run_id", run_id)?,
            tenant_id: None,
            actor: normalize_required("actor", actor)?,
        };
        Ok(scope)
    }
}

/// Provider preference carried by start commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationExecutionProviderPreference {
    pub provider_kind: ApplicationExecutionProviderKind,
    pub provider_id: Option<String>,
}

/// Start command for creating a durable execution run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartApplicationExecutionCommand {
    pub application_id: ApplicationId,
    pub session_id: Option<String>,
    pub run_id: Option<String>,
    pub task_input: ApplicationExecutionPayload,
    pub workspace_ref: Option<String>,
    pub requested_capabilities: Vec<CapabilityId>,
    pub provider_preference: Option<ApplicationExecutionProviderPreference>,
    pub trace: TraceContext,
    pub policy_context: BTreeMap<String, String>,
    pub tenant_id: Option<String>,
    pub actor: String,
    pub idempotency_key: String,
}

/// Start result returned before callers subscribe to durable events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartApplicationExecutionResult {
    pub status: ApplicationExecutionCommandStatus,
    pub session_id: Option<String>,
    pub run_id: Option<String>,
    pub provider_id: Option<String>,
    pub provider_kind: ApplicationExecutionProviderKind,
    pub event_cursor: Option<String>,
    pub control_ref: Option<String>,
    pub workspace_ref: Option<String>,
    pub error: Option<ApplicationExecutionError>,
}

impl StartApplicationExecutionResult {
    /// Return a structured unavailable start result for Null Object providers.
    pub fn unavailable(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            status: ApplicationExecutionCommandStatus::Unavailable,
            session_id: None,
            run_id: None,
            provider_id: None,
            provider_kind: ApplicationExecutionProviderKind::Unavailable,
            event_cursor: None,
            control_ref: None,
            workspace_ref: None,
            error: Some(ApplicationExecutionError::unavailable(operation, reason)),
        }
    }
}

/// Durable event envelope appended by providers through Macaca-owned ingress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationExecutionEventEnvelope {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub run_id: String,
    pub seq: Option<u64>,
    pub timestamp: DateTime<Utc>,
    pub event_type: ApplicationExecutionEventType,
    pub trace: TraceContext,
    pub actor: String,
    pub provider_id: String,
    pub provider_kind: ApplicationExecutionProviderKind,
    pub visibility: String,
    pub causality: Vec<String>,
    pub sanitized_payload: ApplicationExecutionPayload,
    pub payload_ref: Option<ApplicationExecutionPayloadRef>,
    pub schema_version: String,
    pub idempotency_key: String,
}

/// Control command routed through service policy and provider adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationExecutionControlCommand {
    pub scope: ApplicationExecutionScope,
    pub command: ApplicationExecutionControlKind,
    pub control_id: String,
    pub reason_code: String,
    pub trace: TraceContext,
    pub policy_context: BTreeMap<String, String>,
    pub payload: Option<ApplicationExecutionPayload>,
    pub idempotency_key: String,
}

/// Control result records delivery and completion without shell-owned state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationExecutionControlResult {
    pub status: ApplicationExecutionCommandStatus,
    pub scope: ApplicationExecutionScope,
    pub provider_id: Option<String>,
    pub provider_kind: ApplicationExecutionProviderKind,
    pub event_cursor: Option<String>,
    pub error: Option<ApplicationExecutionError>,
}

/// Build a provider-neutral service descriptor for `service.application_execution`.
pub fn application_execution_service_descriptor() -> crate::ServiceDescriptor {
    let mut descriptor = crate::ServiceDescriptor::new(
        KernelServiceId::new(APPLICATION_EXECUTION_SERVICE_ID),
        crate::ServiceType::new("application.execution"),
        crate::TraceSchemaRef::new("trace.service.application_execution.v1"),
    );
    descriptor.capabilities.push(crate::ServiceCapability::new(
        CapabilityId::new("capability.application_execution"),
        "Provider-neutral application execution protocol platform",
    ));
    descriptor.lifecycle_state = crate::ServiceLifecycleState::Registered;
    descriptor.health = crate::ServiceHealth::Unavailable {
        reason: "provider not installed".into(),
    };
    descriptor
}

fn normalize_required(name: &str, value: impl Into<String>) -> MacacaResult<String> {
    let value = value.into().trim().to_string();
    if value.is_empty() {
        return Err(MacacaError::Config(format!(
            "application execution {name} must not be empty"
        )));
    }
    Ok(value)
}
