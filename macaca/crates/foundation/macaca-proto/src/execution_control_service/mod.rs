//! Provider-neutral Execution Control service contracts.
//!
//! Execution control is the OS-level contract for optional pause/resume,
//! checkpoint identity, and replayable state transitions.  The DTOs in this
//! module are plain serializable data so application manifests, agent execution
//! commands, runtime providers, SDK clients, and future remote services can
//! share one contract without depending on web-shell channels or concrete
//! workflow implementations.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ApplicationId, MacacaError, MacacaResult, TaskId, TraceContext};

/// Stable service id for the Execution Control system service.
pub const EXECUTION_CONTROL_SERVICE_ID: &str = "service.execution_control";

/// Resolve manifest defaults plus command overrides into an effective policy.
pub const EXECUTION_CONTROL_RESOLVE_POLICY_COMMAND: &str = "execution_control.resolve_policy";

/// Register one execution instance before pause/resume state transitions occur.
pub const EXECUTION_CONTROL_REGISTER_EXECUTION_COMMAND: &str =
    "execution_control.register_execution";

/// Request that an execution enters a pause boundary.
pub const EXECUTION_CONTROL_REQUEST_PAUSE_COMMAND: &str = "execution_control.request_pause";

/// Record a bounded checkpoint reference for replay and diagnostics.
pub const EXECUTION_CONTROL_RECORD_CHECKPOINT_COMMAND: &str = "execution_control.record_checkpoint";

/// Await a resume signal from an allowed resume source.
pub const EXECUTION_CONTROL_AWAIT_RESUME_COMMAND: &str = "execution_control.await_resume";

/// Request delivery of a resume signal to a paused execution.
pub const EXECUTION_CONTROL_REQUEST_RESUME_COMMAND: &str = "execution_control.request_resume";

/// Cancel an outstanding execution-control wait.
pub const EXECUTION_CONTROL_CANCEL_WAIT_COMMAND: &str = "execution_control.cancel_wait";

/// Query the current execution-control state for diagnostics.
pub const EXECUTION_CONTROL_QUERY_STATE_COMMAND: &str = "execution_control.query_state";

/// Build a sanitized execution-control snapshot.
pub const EXECUTION_CONTROL_SNAPSHOT_COMMAND: &str = "execution_control.snapshot";

/// Whether execution control is active for a policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionControlMode {
    Disabled,
    Enabled,
}

/// Bounded checkpoint behavior selected by application policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionControlCheckpointMode {
    Disabled,
    ReferenceOnly,
    SnapshotReference,
}

/// Generic pause trigger strategies.  These variants name execution facts, not
/// concrete applications, agent names, workflow names, or provider names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionControlTrigger {
    ToolCallBarrier {
        tool_name: String,
    },
    GoalLifecycle,
    ForkLifecycle,
    ApprovalBarrier,
    WorkflowBarrier {
        barrier_id: String,
    },
    ApplicationLifecycle,
    ServiceEvent {
        capability_id: String,
        event_kind: String,
    },
    Extension {
        trigger_id: String,
    },
}

impl ExecutionControlTrigger {
    /// Pause after a declared tool reaches a policy-owned barrier.
    pub fn tool_call_barrier(tool_name: impl Into<String>) -> Self {
        Self::ToolCallBarrier {
            tool_name: sanitize_label(tool_name),
        }
    }
}

/// Generic resume source strategies.  Sources are intentionally typed so
/// policy can reject unknown or unsupported extension points deterministically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionControlResumeSource {
    GoalLifecycle,
    ForkLifecycle,
    ApprovalDecision,
    WorkflowSignal {
        signal_id: String,
    },
    ApplicationLifecycle,
    ServiceEvent {
        capability_id: String,
        event_kind: String,
    },
    Extension {
        source_id: String,
    },
}

impl ExecutionControlResumeSource {
    /// Resume when a goal lifecycle reaches a policy-approved terminal state.
    pub fn goal_lifecycle() -> Self {
        Self::GoalLifecycle
    }
}

/// Provider-neutral local resume payload for in-process execution-control adapters.
///
/// This DTO intentionally contains only bounded execution facts. It is safe for
/// runtime-host local channels and future remote execution-control event streams
/// because it does not reference Web, CLI, framework, or application-specific
/// types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeResumeSignal {
    /// Operator or policy requested a normal resume.
    Manual,
    /// Delegated work or a goal lifecycle completed and supplied bounded output.
    DelegateCompleted {
        task_id: String,
        success: bool,
        output: String,
    },
    /// Delegated work failed and supplied a bounded diagnostic.
    DelegateFailed { task_id: String, error: String },
    /// The wait timed out according to policy.
    Timeout,
}

impl RuntimeResumeSignal {
    /// Build a successful delegation/goal lifecycle resume signal.
    pub fn delegate_completed(
        task_id: impl Into<String>,
        success: bool,
        output: impl Into<String>,
    ) -> Self {
        Self::DelegateCompleted {
            task_id: task_id.into(),
            success,
            output: output.into(),
        }
    }

    /// Build a failed delegation/goal lifecycle resume signal.
    pub fn delegate_failed(task_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::DelegateFailed {
            task_id: task_id.into(),
            error: error.into(),
        }
    }
}

/// Application-declared default execution-control policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlPolicy {
    pub mode: ExecutionControlMode,
    pub triggers: Vec<ExecutionControlTrigger>,
    pub resume_sources: Vec<ExecutionControlResumeSource>,
    pub checkpoint_mode: ExecutionControlCheckpointMode,
    /// Defaults to `false` when omitted from YAML/JSON manifests so application
    /// authors only declare overrides when they explicitly want them.
    #[serde(default)]
    pub allow_command_overrides: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Optional provider-neutral extension metadata.  Defaults to empty so YAML
    /// manifests do not need to declare an explicit `metadata: {}` block.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl ExecutionControlPolicy {
    /// Build an explicit disabled policy.  Disabled is data, not absence, so
    /// traces can distinguish "not declared" from "declared off".
    pub fn disabled() -> Self {
        Self {
            mode: ExecutionControlMode::Disabled,
            triggers: Vec::new(),
            resume_sources: Vec::new(),
            checkpoint_mode: ExecutionControlCheckpointMode::Disabled,
            allow_command_overrides: false,
            timeout_ms: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Build an enabled policy from the minimum safe strategy set.
    pub fn enabled(
        triggers: Vec<ExecutionControlTrigger>,
        resume_sources: Vec<ExecutionControlResumeSource>,
        checkpoint_mode: ExecutionControlCheckpointMode,
    ) -> Self {
        Self {
            mode: ExecutionControlMode::Enabled,
            triggers,
            resume_sources,
            checkpoint_mode,
            allow_command_overrides: false,
            timeout_ms: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Permit per-run command overrides within the declared policy boundary.
    pub fn allow_command_overrides(mut self, allow: bool) -> Self {
        self.allow_command_overrides = allow;
        self
    }
}

/// Per-run override carried by `AgentExecutionCommand`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlPolicyOverride {
    pub mode: ExecutionControlMode,
    pub triggers: Vec<ExecutionControlTrigger>,
    pub resume_sources: Vec<ExecutionControlResumeSource>,
    pub checkpoint_mode: ExecutionControlCheckpointMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl ExecutionControlPolicyOverride {
    /// Request execution control for this run.  The runtime resolver still
    /// checks this request against the application-declared policy.
    pub fn enable_for_run(
        triggers: Vec<ExecutionControlTrigger>,
        resume_sources: Vec<ExecutionControlResumeSource>,
        checkpoint_mode: ExecutionControlCheckpointMode,
    ) -> Self {
        Self {
            mode: ExecutionControlMode::Enabled,
            triggers,
            resume_sources,
            checkpoint_mode,
            timeout_ms: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Outcome of policy resolution before runtime side effects occur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionControlResolutionStatus {
    Disabled,
    Enabled,
    Denied,
    Unsupported,
}

/// Effective policy and diagnostic reason returned by policy resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlResolvedPolicy {
    pub status: ExecutionControlResolutionStatus,
    pub policy: Option<ExecutionControlPolicy>,
    pub reason_code: String,
    pub metadata: BTreeMap<String, String>,
    /// Sanitized policy-resolution evidence emitted before any runtime side
    /// effect is allowed to register or pause an execution.
    ///
    /// Keeping the event beside the resolved policy lets adapters persist the
    /// exact admission decision without reconstructing it from logs.  The
    /// event uses the same provider-neutral `ExecutionControlEvent` envelope as
    /// later state transitions, so audit replay can show one continuous chain:
    /// resolve policy, register execution, pause, checkpoint, resume, timeout
    /// or cancellation, and diagnostics queries.
    #[serde(default)]
    pub events: Vec<ExecutionControlEvent>,
}

/// State machine for one execution-control registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionControlState {
    Running,
    PauseRequested,
    Paused,
    ResumeRequested,
    Resuming,
    Completed,
    Cancelled,
    TimedOut,
    Failed,
}

/// Sanitized event emitted by execution-control transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlEvent {
    pub execution_id: String,
    pub state: ExecutionControlState,
    pub reason_code: String,
    pub trace: TraceContext,
    pub metadata: BTreeMap<String, String>,
}

/// Common envelope for service commands that target one execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlScope {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub execution_id: String,
    pub task_id: Option<TaskId>,
    pub trace: TraceContext,
    pub source: String,
    pub metadata: BTreeMap<String, String>,
}

impl ExecutionControlScope {
    /// Build the required command scope with a trace id before side effects.
    pub fn new(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        execution_id: impl Into<String>,
        trace: TraceContext,
        source: impl Into<String>,
    ) -> MacacaResult<Self> {
        if trace.trace_id.trim().is_empty() {
            return Err(MacacaError::Agent(
                "execution control command requires trace_id".into(),
            ));
        }
        Ok(Self {
            application_id,
            session_id: non_empty(session_id.into(), "session_id is required")?,
            execution_id: non_empty(execution_id.into(), "execution_id is required")?,
            task_id: None,
            trace,
            source: non_empty(source.into(), "source is required")?,
            metadata: BTreeMap::new(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlResolvePolicyCommand {
    pub scope: ExecutionControlScope,
    pub app_default: Option<ExecutionControlPolicy>,
    pub command_override: Option<ExecutionControlPolicyOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlRegisterExecutionCommand {
    pub scope: ExecutionControlScope,
    pub policy: ExecutionControlResolvedPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlPauseCommand {
    pub scope: ExecutionControlScope,
    pub trigger: ExecutionControlTrigger,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlCheckpointCommand {
    pub scope: ExecutionControlScope,
    pub checkpoint_ref: String,
    pub checkpoint_mode: ExecutionControlCheckpointMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlAwaitResumeCommand {
    pub scope: ExecutionControlScope,
    pub resume_sources: Vec<ExecutionControlResumeSource>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlCancelWaitCommand {
    pub scope: ExecutionControlScope,
    pub reason_code: String,
    /// Optional wait budget that was exhausted before a resume source arrived.
    ///
    /// `None` means the caller intentionally cancelled the wait.  `Some` means
    /// the same command surface is recording a timeout completion.  The field
    /// is optional and defaulted so older serialized commands continue to
    /// decode as explicit cancellations.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlResumeCommand {
    pub scope: ExecutionControlScope,
    pub resume_source: ExecutionControlResumeSource,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlStateCommand {
    pub scope: ExecutionControlScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlSnapshotCommand {
    pub scope: ExecutionControlScope,
}

/// Generic service result for execution-control commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionControlCommandResult {
    pub scope: ExecutionControlScope,
    pub status: ExecutionControlResolutionStatus,
    pub state: ExecutionControlState,
    pub reason_code: String,
    pub events: Vec<ExecutionControlEvent>,
    pub metadata: BTreeMap<String, String>,
}

/// Trim user-supplied labels before they enter serializable trigger/barrier DTOs.
fn sanitize_label(value: impl Into<String>) -> String {
    value.into().trim().to_string()
}

/// Reject empty scope fields so execution-control commands always carry auditable identity.
fn non_empty(value: String, message: &str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        Err(MacacaError::Agent(message.into()))
    } else {
        Ok(trimmed)
    }
}

/// Adapter layer: typed execution-control commands → provider-neutral `ServiceCommand`.
///
/// **Pattern:** Adapter — keeps serde/command-name wiring out of the pure DTO module so
/// contract types stay stable while transport encoding evolves independently.
mod command_adapters;

#[cfg(test)]
mod tests;
