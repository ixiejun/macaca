//! Session-loop orchestration facade backed by `service.execution_control`.
//!
//! PlanLoop and WorkerLoop controllers inside `macaca-task` still run as local
//! async tasks, but every register/wake/shutdown decision must leave durable
//! service-call evidence correlated with the generic task service (`service.task`).
//! This module routes those lifecycle signals through typed execution-control
//! commands so audit replay can reconstruct loop wake chains without reading
//! shell-private waker maps.
//!
//! # Architecture (Facade + Command)
//!
//! - **Command**: [`SessionLoopRegisterRequest`], [`SessionLoopWakeRequest`],
//!   [`SessionLoopShutdownRequest`] capture provider-neutral inputs (no application
//!   role names or workflow labels).
//! - **Facade**: [`ExecutionControlSessionLoopCoordinator`] owns the service-call
//!   choreography for register/checkpoint/shutdown recording.
//! - **Observer**: web `session_loop_shell_adapter` invokes the facade before
//!   touching local in-process `PlanLoopWaker` / `WorkerLoopWaker` handles.
//!
//! Shell adapters retain local wakers until loop_manager fully subscribes to
//! execution-control event streams (task 4.2 / 4.3 file split).

use std::sync::Arc;

use macaca_proto::{
    ApplicationId, ExecutionControlCheckpointCommand, ExecutionControlCheckpointMode,
    ExecutionControlCommandResult, ExecutionControlPolicy,
    ExecutionControlRegisterExecutionCommand, ExecutionControlResolutionStatus,
    ExecutionControlResumeSource, ExecutionControlScope, ExecutionControlTrigger, KernelServiceId,
    MacacaResult, ServiceBusSource, TraceContext, EXECUTION_CONTROL_SERVICE_ID,
};
use tracing::{info, warn};

use crate::execution_control::ExecutionControlPolicyResolver;
use crate::ServiceRuntime;

/// Stable bus source label for session-loop execution-control commands.
pub const SESSION_LOOP_EXECUTION_CONTROL_SOURCE: &str = "macaca.runtime_host.session_loop";

/// Generic task-service capability id referenced by session-loop service events.
///
/// The id is intentionally short (`task`) because execution-control policies
/// correlate service events by capability id + event kind, not by concrete crate
/// names.  The full service id remains `service.task` at the bus layer.
pub const SESSION_LOOP_TASK_CAPABILITY_ID: &str = "task";

/// Provider-neutral task-service event kinds for plan/worker loop lifecycle.
pub const SESSION_LOOP_PLAN_WAKE_EVENT: &str = "plan_loop_wake";
pub const SESSION_LOOP_WORKER_WAKE_EVENT: &str = "worker_loop_wake";
pub const SESSION_LOOP_PLAN_REGISTER_EVENT: &str = "plan_loop_registered";
pub const SESSION_LOOP_WORKER_REGISTER_EVENT: &str = "worker_loop_registered";
pub const SESSION_LOOP_SHUTDOWN_EVENT: &str = "session_loop_shutdown";

/// Identifies which session loop controller emitted a lifecycle signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLoopKind {
    /// Goal-planning controller (single instance per application scope).
    Plan,
    /// Task-pull worker controller fleet (one or more agents per application).
    Worker,
}

impl SessionLoopKind {
    /// Return the register event kind for this loop controller.
    pub fn register_event_kind(self) -> &'static str {
        match self {
            Self::Plan => SESSION_LOOP_PLAN_REGISTER_EVENT,
            Self::Worker => SESSION_LOOP_WORKER_REGISTER_EVENT,
        }
    }

    /// Return the wake event kind for this loop controller.
    pub fn wake_event_kind(self) -> &'static str {
        match self {
            Self::Plan => SESSION_LOOP_PLAN_WAKE_EVENT,
            Self::Worker => SESSION_LOOP_WORKER_WAKE_EVENT,
        }
    }

    /// Stable execution-control registration suffix for one loop kind.
    ///
    /// Suffixes are provider-neutral controller labels (not application agent
    /// role names).  `task_pull` identifies pull-based task controllers without
    /// hardcoding manifest role strings such as worker/planner/coordinator.
    pub fn execution_suffix(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Worker => "task_pull",
        }
    }
}

/// Provider-neutral request to register a session loop controller with execution control.
#[derive(Debug, Clone)]
pub struct SessionLoopRegisterRequest {
    pub application_id: ApplicationId,
    /// Optional session scope when loops are started for one chat session.
    pub session_id: Option<String>,
    pub loop_kind: SessionLoopKind,
    /// Number of worker loops when `loop_kind == Worker` (plan loops use `None`).
    pub worker_count: Option<usize>,
    pub trace: TraceContext,
}

/// Provider-neutral request to record a loop wake through execution control.
#[derive(Debug, Clone)]
pub struct SessionLoopWakeRequest {
    pub application_id: ApplicationId,
    pub session_id: Option<String>,
    pub loop_kind: SessionLoopKind,
    pub trace: TraceContext,
    /// Provider-neutral reason code for audit replay (for example decomposition_ready).
    pub reason_code: String,
    /// Optional bounded detail string stored in execution-control metadata.
    pub detail: Option<String>,
}

/// Provider-neutral request to record session-loop shutdown through execution control.
#[derive(Debug, Clone)]
pub struct SessionLoopShutdownRequest {
    pub application_id: ApplicationId,
    pub session_id: Option<String>,
    pub trace: TraceContext,
    pub reason_code: String,
}

/// Facade that routes session-loop lifecycle signals through `service.execution_control`.
pub struct ExecutionControlSessionLoopCoordinator {
    service_runtime: Arc<ServiceRuntime>,
    bus_source: ServiceBusSource,
}

impl ExecutionControlSessionLoopCoordinator {
    /// Bind the coordinator to the shared host service runtime.
    pub fn new(service_runtime: Arc<ServiceRuntime>) -> Self {
        Self {
            service_runtime,
            bus_source: ServiceBusSource::new(SESSION_LOOP_EXECUTION_CONTROL_SOURCE),
        }
    }

    /// Build the execution-control registration id for one application loop controller.
    ///
    /// The id is deterministic from application + loop kind (+ optional session)
    /// so replay and diagnostics can correlate wake checkpoints with loop startup.
    pub fn loop_execution_id(
        application_id: &ApplicationId,
        session_id: Option<&str>,
        loop_kind: SessionLoopKind,
    ) -> String {
        let session_suffix = session_id.unwrap_or("application");
        format!(
            "session-loop:{}:{}:{}",
            loop_kind.execution_suffix(),
            session_suffix,
            application_id.0
        )
    }

    /// Build the default session-loop policy used when applications have not
    /// declared execution-control defaults in their manifest projection.
    pub fn session_loop_policy() -> ExecutionControlPolicy {
        ExecutionControlPolicy::enabled(
            vec![
                ExecutionControlTrigger::ServiceEvent {
                    capability_id: SESSION_LOOP_TASK_CAPABILITY_ID.into(),
                    event_kind: SESSION_LOOP_PLAN_WAKE_EVENT.into(),
                },
                ExecutionControlTrigger::ServiceEvent {
                    capability_id: SESSION_LOOP_TASK_CAPABILITY_ID.into(),
                    event_kind: SESSION_LOOP_WORKER_WAKE_EVENT.into(),
                },
            ],
            vec![
                ExecutionControlResumeSource::ServiceEvent {
                    capability_id: SESSION_LOOP_TASK_CAPABILITY_ID.into(),
                    event_kind: SESSION_LOOP_PLAN_WAKE_EVENT.into(),
                },
                ExecutionControlResumeSource::ServiceEvent {
                    capability_id: SESSION_LOOP_TASK_CAPABILITY_ID.into(),
                    event_kind: SESSION_LOOP_WORKER_WAKE_EVENT.into(),
                },
            ],
            ExecutionControlCheckpointMode::ReferenceOnly,
        )
    }

    /// Build the provider-neutral execution-control scope for one loop controller.
    pub fn loop_scope(request: &SessionLoopRegisterRequest) -> MacacaResult<ExecutionControlScope> {
        let execution_id = Self::loop_execution_id(
            &request.application_id,
            request.session_id.as_deref(),
            request.loop_kind,
        );
        let mut scope = ExecutionControlScope::new(
            request.application_id,
            request
                .session_id
                .clone()
                .unwrap_or_else(|| format!("_macaca_app_{}", request.application_id.0)),
            execution_id,
            request.trace.clone(),
            SESSION_LOOP_EXECUTION_CONTROL_SOURCE,
        )?;
        scope.metadata.insert(
            "loop_kind".into(),
            request.loop_kind.execution_suffix().into(),
        );
        scope.metadata.insert(
            "register_event_kind".into(),
            request.loop_kind.register_event_kind().into(),
        );
        if let Some(worker_count) = request.worker_count {
            scope
                .metadata
                .insert("worker_count".into(), worker_count.to_string());
        }
        scope.metadata.insert(
            "graph_owner".into(),
            "execution_control.session_loop.controller".into(),
        );
        Ok(scope)
    }

    /// Register a session loop controller and record an initial checkpoint.
    ///
    /// Registration emits durable service-call evidence when PlanLoop/WorkerLoop
    /// tasks are spawned.  Local waker maps remain shell notification state.
    pub async fn register_session_loop(
        &self,
        request: SessionLoopRegisterRequest,
    ) -> Result<ExecutionControlCommandResult, String> {
        let scope = Self::loop_scope(&request).map_err(|error| error.to_string())?;
        let resolved =
            ExecutionControlPolicyResolver::resolve(Some(&Self::session_loop_policy()), None);
        if resolved.status != ExecutionControlResolutionStatus::Enabled {
            return Err(format!(
                "session-loop execution control denied: {}",
                resolved.reason_code
            ));
        }

        let register = self
            .call_execution_control(
                ExecutionControlRegisterExecutionCommand {
                    scope: scope.clone(),
                    policy: resolved,
                }
                .into_service_command()
                .map_err(|error| error.to_string())?,
            )
            .await?;

        let checkpoint_ref = format!(
            "{}:{}",
            request.loop_kind.register_event_kind(),
            request.application_id.0
        );
        let _checkpoint = self
            .call_execution_control(
                ExecutionControlCheckpointCommand {
                    scope,
                    checkpoint_ref,
                    checkpoint_mode: ExecutionControlCheckpointMode::ReferenceOnly,
                }
                .into_service_command()
                .map_err(|error| error.to_string())?,
            )
            .await?;

        info!(
            application_id = %request.application_id.0,
            loop_kind = request.loop_kind.execution_suffix(),
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            event_kind = request.loop_kind.register_event_kind(),
            "Registered session loop controller via execution control"
        );

        Ok(register)
    }

    /// Record a loop wake checkpoint before the shell adapter wakes local controllers.
    pub async fn record_loop_wake(
        &self,
        request: SessionLoopWakeRequest,
    ) -> Result<ExecutionControlCommandResult, String> {
        let trace = request.trace.clone();
        let mut scope = ExecutionControlScope::new(
            request.application_id,
            request
                .session_id
                .clone()
                .unwrap_or_else(|| format!("_macaca_app_{}", request.application_id.0)),
            Self::loop_execution_id(
                &request.application_id,
                request.session_id.as_deref(),
                request.loop_kind,
            ),
            trace,
            SESSION_LOOP_EXECUTION_CONTROL_SOURCE,
        )
        .map_err(|error| error.to_string())?;
        scope.metadata.insert(
            "loop_kind".into(),
            request.loop_kind.execution_suffix().into(),
        );
        scope.metadata.insert(
            "wake_event_kind".into(),
            request.loop_kind.wake_event_kind().into(),
        );
        scope
            .metadata
            .insert("reason_code".into(), request.reason_code.clone());
        if let Some(detail) = request.detail {
            scope
                .metadata
                .insert("detail".into(), detail.chars().take(200).collect());
        }

        let checkpoint_ref = format!(
            "{}:{}:{}",
            request.loop_kind.wake_event_kind(),
            request.application_id.0,
            request.reason_code
        );
        let result = self
            .call_execution_control(
                ExecutionControlCheckpointCommand {
                    scope,
                    checkpoint_ref,
                    checkpoint_mode: ExecutionControlCheckpointMode::ReferenceOnly,
                }
                .into_service_command()
                .map_err(|error| error.to_string())?,
            )
            .await?;

        info!(
            application_id = %request.application_id.0,
            loop_kind = request.loop_kind.execution_suffix(),
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            reason_code = %request.reason_code,
            event_kind = request.loop_kind.wake_event_kind(),
            "Recorded session loop wake via execution control"
        );

        Ok(result)
    }

    /// Record shutdown checkpoints for all session loops owned by one application scope.
    pub async fn record_session_loop_shutdown(
        &self,
        request: SessionLoopShutdownRequest,
    ) -> Result<(), String> {
        for loop_kind in [SessionLoopKind::Plan, SessionLoopKind::Worker] {
            let mut scope = ExecutionControlScope::new(
                request.application_id,
                request
                    .session_id
                    .clone()
                    .unwrap_or_else(|| format!("_macaca_app_{}", request.application_id.0)),
                Self::loop_execution_id(
                    &request.application_id,
                    request.session_id.as_deref(),
                    loop_kind,
                ),
                request.trace.clone(),
                SESSION_LOOP_EXECUTION_CONTROL_SOURCE,
            )
            .map_err(|error| error.to_string())?;
            scope
                .metadata
                .insert("loop_kind".into(), loop_kind.execution_suffix().into());
            scope.metadata.insert(
                "shutdown_event_kind".into(),
                SESSION_LOOP_SHUTDOWN_EVENT.into(),
            );
            scope
                .metadata
                .insert("reason_code".into(), request.reason_code.clone());

            let checkpoint_ref = format!(
                "{}:{}:{}",
                SESSION_LOOP_SHUTDOWN_EVENT,
                loop_kind.execution_suffix(),
                request.application_id.0
            );
            if let Err(error) = self
                .call_execution_control(
                    ExecutionControlCheckpointCommand {
                        scope,
                        checkpoint_ref,
                        checkpoint_mode: ExecutionControlCheckpointMode::ReferenceOnly,
                    }
                    .into_service_command()
                    .map_err(|error| error.to_string())?,
                )
                .await
            {
                warn!(
                    application_id = %request.application_id.0,
                    loop_kind = loop_kind.execution_suffix(),
                    error = %error,
                    "Session-loop shutdown checkpoint failed; continuing shell cleanup"
                );
            }
        }

        info!(
            application_id = %request.application_id.0,
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            reason_code = %request.reason_code,
            "Recorded session loop shutdown via execution control"
        );
        Ok(())
    }

    async fn call_execution_control(
        &self,
        service_command: macaca_proto::ServiceCommand,
    ) -> Result<ExecutionControlCommandResult, String> {
        let reply = self
            .service_runtime
            .call(
                &KernelServiceId::new(EXECUTION_CONTROL_SERVICE_ID),
                self.bus_source.clone(),
                service_command,
            )
            .await
            .map_err(|error| error.to_string())?;
        if !reply.success {
            warn!(
                service_id = EXECUTION_CONTROL_SERVICE_ID,
                status = %reply.status,
                "execution control service returned non-success for session-loop command"
            );
            return Err(format!(
                "execution control service rejected session-loop command: {}",
                reply.status
            ));
        }
        let output = reply
            .output
            .ok_or_else(|| "execution control service returned no output".to_string())?;
        serde_json::from_value(output).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_execution_id_is_deterministic() {
        let app_id = ApplicationId::new();
        let id = ExecutionControlSessionLoopCoordinator::loop_execution_id(
            &app_id,
            Some("session-a"),
            SessionLoopKind::Plan,
        );
        assert_eq!(id, format!("session-loop:plan:session-a:{}", app_id.0));
    }

    #[test]
    fn session_loop_policy_uses_task_service_events() {
        let policy = ExecutionControlSessionLoopCoordinator::session_loop_policy();
        assert!(policy
            .triggers
            .contains(&ExecutionControlTrigger::ServiceEvent {
                capability_id: SESSION_LOOP_TASK_CAPABILITY_ID.into(),
                event_kind: SESSION_LOOP_PLAN_WAKE_EVENT.into(),
            }));
        assert!(policy
            .triggers
            .contains(&ExecutionControlTrigger::ServiceEvent {
                capability_id: SESSION_LOOP_TASK_CAPABILITY_ID.into(),
                event_kind: SESSION_LOOP_WORKER_WAKE_EVENT.into(),
            }));
    }

    #[test]
    fn loop_scope_carries_loop_metadata() {
        let request = SessionLoopRegisterRequest {
            application_id: ApplicationId::new(),
            session_id: Some("session-loop".into()),
            loop_kind: SessionLoopKind::Worker,
            worker_count: Some(3),
            trace: TraceContext::new("trace-session-loop"),
        };
        let scope = ExecutionControlSessionLoopCoordinator::loop_scope(&request).unwrap();
        assert_eq!(scope.session_id, "session-loop");
        assert_eq!(
            scope.metadata.get("worker_count").map(String::as_str),
            Some("3")
        );
    }
}
