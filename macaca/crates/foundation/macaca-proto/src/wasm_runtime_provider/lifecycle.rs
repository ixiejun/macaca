//! Provider-neutral WASM lifecycle, checkpoint, restore, upgrade, and rollback contracts.
//!
//! The types in this module are intentionally data-only.  They apply the State,
//! Command, Memento, Observer, and Specification patterns at the protocol layer:
//! callers describe what lifecycle operation they want, the runtime validates the
//! operation against a centralized state graph, and the result/audit DTOs carry
//! bounded metadata that can be logged or persisted without exposing raw guest
//! memory, raw command payloads, prompts, secrets, environment variables, or
//! provider-specific engine handles.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ApplicationAbiVersion, TraceContext};

use super::{sanitize_diagnostic_text, WasmRuntimeArtifactRef};

/// Lifecycle states visible at the WASM runtime provider boundary.
///
/// The enum models infrastructure execution states, not application business
/// workflows.  It is therefore generic across every WASM application Macaca can
/// host and deliberately avoids app names, driver names, workflow names, or
/// domain-specific branches.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmLifecycleState {
    Declared,
    Validated,
    Compiled,
    Instantiated,
    Initialized,
    Started,
    HandlingEvent,
    Rendered,
    Paused,
    Draining,
    ShuttingDown,
    Stopped,
    Failed,
}

/// Typed lifecycle operation requested by a caller.
///
/// Operations are Command identifiers.  The runtime-host can map supported
/// operations to guest exports, structured unsupported results, checkpoint
/// mementos, or ABI decision reports without changing this public vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmLifecycleOperation {
    Validate,
    Compile,
    Instantiate,
    Init,
    Start,
    HandleEvent,
    Render,
    Pause,
    Resume,
    Drain,
    Shutdown,
    Checkpoint,
    Restore,
    Upgrade,
    Rollback,
}

impl WasmLifecycleOperation {
    /// Return the stable operation code used in logs, audit records, and tests.
    pub fn as_code(&self) -> &'static str {
        match self {
            Self::Validate => "validate",
            Self::Compile => "compile",
            Self::Instantiate => "instantiate",
            Self::Init => "init",
            Self::Start => "start",
            Self::HandleEvent => "handle_event",
            Self::Render => "render",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Drain => "drain",
            Self::Shutdown => "shutdown",
            Self::Checkpoint => "checkpoint",
            Self::Restore => "restore",
            Self::Upgrade => "upgrade",
            Self::Rollback => "rollback",
        }
    }

    /// Return the Application ABI export used by operations with real guest work.
    ///
    /// Checkpoint, restore, upgrade, rollback, and drain are provider lifecycle
    /// operations in the current slice.  They intentionally do not imply raw
    /// guest-memory serialization or engine-level suspension support.
    pub fn abi_export(&self) -> Option<&'static str> {
        match self {
            Self::Init => Some("app:init"),
            Self::Start => Some("app:start"),
            Self::HandleEvent => Some("app:handle_event"),
            Self::Render => Some("app:render"),
            Self::Shutdown => Some("app:shutdown"),
            Self::Pause => Some("app:pause"),
            Self::Resume => Some("app:resume"),
            Self::Upgrade => Some("app:upgrade"),
            _ => None,
        }
    }
}

/// Fail-closed reason taxonomy for lifecycle operations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmLifecycleReasonCode {
    Completed,
    MissingTrace,
    InvalidTransition,
    Unsupported,
    Unavailable,
    PolicyDenied,
    ResourceExhausted,
    AbiMismatch,
    RuntimeFailed,
}

impl WasmLifecycleReasonCode {
    /// Return the stable reason code used by structured results and audit logs.
    pub fn as_code(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::MissingTrace => "missing_trace",
            Self::InvalidTransition => "invalid_transition",
            Self::Unsupported => "unsupported",
            Self::Unavailable => "unavailable",
            Self::PolicyDenied => "policy_denied",
            Self::ResourceExhausted => "resource_exhausted",
            Self::AbiMismatch => "abi_mismatch",
            Self::RuntimeFailed => "runtime_failed",
        }
    }
}

/// Structured lifecycle operation status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmLifecycleOperationStatus {
    Completed,
    Rejected,
    Unsupported,
    Unavailable,
    Failed,
}

/// Command-style request for one lifecycle operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmLifecycleCommand {
    pub operation: WasmLifecycleOperation,
    pub trace: Option<TraceContext>,
    pub payload: Value,
    pub metadata: BTreeMap<String, String>,
}

impl WasmLifecycleCommand {
    /// Build a traced lifecycle command with empty payload and metadata.
    pub fn traced(operation: WasmLifecycleOperation, trace: TraceContext) -> Self {
        Self {
            operation,
            trace: Some(trace),
            payload: Value::Null,
            metadata: BTreeMap::new(),
        }
    }

    /// Return a non-empty trace or a typed fail-closed reason.
    pub fn require_trace(&self) -> Result<TraceContext, WasmLifecycleReasonCode> {
        match &self.trace {
            Some(trace) if !trace.trace_id.trim().is_empty() => Ok(trace.clone()),
            _ => Err(WasmLifecycleReasonCode::MissingTrace),
        }
    }
}

/// Result returned for every lifecycle transition command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmLifecycleTransitionResult {
    pub operation: WasmLifecycleOperation,
    pub status: WasmLifecycleOperationStatus,
    pub from: WasmLifecycleState,
    pub to: WasmLifecycleState,
    pub reason_code: String,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmLifecycleTransitionResult {
    /// Build a completed transition result with sanitized metadata.
    pub fn completed(
        operation: WasmLifecycleOperation,
        from: WasmLifecycleState,
        to: WasmLifecycleState,
        trace: Option<TraceContext>,
        metadata: BTreeMap<String, String>,
    ) -> Self {
        Self {
            operation,
            status: WasmLifecycleOperationStatus::Completed,
            from,
            to,
            reason_code: WasmLifecycleReasonCode::Completed.as_code().into(),
            trace,
            metadata: sanitize_wasm_lifecycle_metadata(metadata),
        }
    }

    /// Build a fail-closed result without changing the runtime state.
    pub fn rejected(
        operation: WasmLifecycleOperation,
        state: WasmLifecycleState,
        reason: WasmLifecycleReasonCode,
        trace: Option<TraceContext>,
        metadata: BTreeMap<String, String>,
    ) -> Self {
        let status = match reason {
            WasmLifecycleReasonCode::Unsupported => WasmLifecycleOperationStatus::Unsupported,
            WasmLifecycleReasonCode::Unavailable => WasmLifecycleOperationStatus::Unavailable,
            WasmLifecycleReasonCode::RuntimeFailed => WasmLifecycleOperationStatus::Failed,
            _ => WasmLifecycleOperationStatus::Rejected,
        };
        Self {
            operation,
            status,
            from: state.clone(),
            to: state,
            reason_code: reason.as_code().into(),
            trace,
            metadata: sanitize_wasm_lifecycle_metadata(metadata),
        }
    }
}

/// Sanitized Observer record emitted for lifecycle requests and outcomes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmLifecycleAuditEvent {
    pub session_id: String,
    pub application_id: String,
    pub ability_id: String,
    pub operation: WasmLifecycleOperation,
    pub status: WasmLifecycleOperationStatus,
    pub from: WasmLifecycleState,
    pub to: WasmLifecycleState,
    pub reason_code: String,
    pub trace_id: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmLifecycleAuditEvent {
    /// Build an audit event from a transition result while redacting metadata.
    pub fn from_transition(
        session_id: impl Into<String>,
        application_id: impl Into<String>,
        ability_id: impl Into<String>,
        result: &WasmLifecycleTransitionResult,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            application_id: application_id.into(),
            ability_id: ability_id.into(),
            operation: result.operation.clone(),
            status: result.status.clone(),
            from: result.from.clone(),
            to: result.to.clone(),
            reason_code: result.reason_code.clone(),
            trace_id: result.trace.as_ref().map(|trace| trace.trace_id.clone()),
            occurred_at: Utc::now(),
            metadata: sanitize_wasm_lifecycle_metadata(result.metadata.clone()),
        }
    }
}

/// Portable checkpoint memento for a WASM session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmCheckpointMemento {
    pub checkpoint_id: String,
    pub session_id: String,
    pub application_id: String,
    pub ability_id: String,
    pub lifecycle_state: WasmLifecycleState,
    pub artifact: WasmRuntimeArtifactRef,
    pub artifact_digest_prefix: String,
    pub abi_version: ApplicationAbiVersion,
    pub created_at: DateTime<Utc>,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// Request to restore from a portable checkpoint memento.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmRestoreRequest {
    pub checkpoint: WasmCheckpointMemento,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// Report returned after a restore ABI decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmRestoreReport {
    pub status: WasmLifecycleOperationStatus,
    pub reason_code: String,
    pub checkpoint_id: String,
    pub lifecycle_state: WasmLifecycleState,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// Request to upgrade a session to an ABI-matching artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmUpgradeRequest {
    pub target_artifact: WasmRuntimeArtifactRef,
    pub target_artifact_digest: String,
    pub target_abi_version: ApplicationAbiVersion,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// ABI decision report for an upgrade operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmUpgradeReport {
    pub status: WasmLifecycleOperationStatus,
    pub reason_code: String,
    pub source_artifact: WasmRuntimeArtifactRef,
    pub source_artifact_digest_prefix: String,
    pub target_artifact: WasmRuntimeArtifactRef,
    pub target_artifact_digest_prefix: String,
    pub abi_matches: bool,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// Request to roll a session back to a checkpoint memento.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmRollbackRequest {
    pub checkpoint: WasmCheckpointMemento,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// Report returned after a rollback ABI decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmRollbackReport {
    pub status: WasmLifecycleOperationStatus,
    pub reason_code: String,
    pub checkpoint_id: String,
    pub restored_lifecycle_state: WasmLifecycleState,
    pub trace: Option<TraceContext>,
    pub metadata: BTreeMap<String, String>,
}

/// Stateful lifecycle validator for one WASM session.
#[derive(Debug, Clone)]
pub struct WasmLifecycleStateMachine {
    state: WasmLifecycleState,
}

impl WasmLifecycleStateMachine {
    /// Create a state machine from an already validated state.
    pub fn from_state(state: WasmLifecycleState) -> Self {
        Self { state }
    }

    /// Create a state machine for a session that has already been instantiated.
    pub fn instantiated() -> Self {
        Self {
            state: WasmLifecycleState::Instantiated,
        }
    }

    /// Return the current lifecycle state.
    pub fn state(&self) -> WasmLifecycleState {
        self.state.clone()
    }

    /// Validate and commit a transition for the requested operation.
    pub fn transition(&mut self, command: &WasmLifecycleCommand) -> WasmLifecycleTransitionResult {
        let trace = match command.require_trace() {
            Ok(trace) => Some(trace),
            Err(reason) => {
                return WasmLifecycleTransitionResult::rejected(
                    command.operation.clone(),
                    self.state.clone(),
                    reason,
                    command.trace.clone(),
                    command.metadata.clone(),
                );
            }
        };
        let from = self.state.clone();
        let Some(to) = next_state_for_operation(&from, &command.operation) else {
            return WasmLifecycleTransitionResult::rejected(
                command.operation.clone(),
                from,
                WasmLifecycleReasonCode::InvalidTransition,
                trace,
                command.metadata.clone(),
            );
        };
        self.state = to.clone();
        WasmLifecycleTransitionResult::completed(
            command.operation.clone(),
            from,
            to,
            trace,
            command.metadata.clone(),
        )
    }
}

/// Return the next state for an allowed lifecycle operation.
pub fn next_state_for_operation(
    from: &WasmLifecycleState,
    operation: &WasmLifecycleOperation,
) -> Option<WasmLifecycleState> {
    use WasmLifecycleOperation::*;
    use WasmLifecycleState::*;
    match (from, operation) {
        (Declared, Validate) => Some(Validated),
        (Validated, Compile) => Some(Compiled),
        (Compiled, Instantiate) => Some(Instantiated),
        (Instantiated, Init) => Some(Initialized),
        (Initialized, Start) | (Rendered, Start) | (HandlingEvent, Start) => Some(Started),
        (Started, HandleEvent) => Some(HandlingEvent),
        (Started, Render) => Some(Rendered),
        (Started, Pause) => Some(Paused),
        (Paused, Resume) => Some(Started),
        (Started, Drain) | (Paused, Drain) => Some(Draining),
        (Draining, Shutdown)
        | (Started, Shutdown)
        | (Initialized, Shutdown)
        | (Paused, Shutdown) => Some(ShuttingDown),
        (ShuttingDown, Shutdown) => Some(Stopped),
        _ => None,
    }
}

/// Return metadata with unsafe keys omitted and unsafe values redacted.
pub fn sanitize_wasm_lifecycle_metadata(
    metadata: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    metadata
        .into_iter()
        .filter_map(|(key, value)| {
            let lower = key.to_ascii_lowercase();
            if [
                "raw", "memory", "payload", "prompt", "secret", "api_key", "apikey", "env",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
            {
                return None;
            }
            Some((key, sanitize_diagnostic_text(value)))
        })
        .collect()
}
