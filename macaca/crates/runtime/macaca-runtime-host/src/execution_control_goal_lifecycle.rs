//! Goal-lifecycle orchestration facade backed by `service.execution_control`.
//!
//! When an entry agent creates a long-running goal (`create_goal` tool), the parent
//! session execution must pause until the PlanLoop reports `GoalCompleted`.  This
//! module routes that pause/resume contract through typed execution-control service
//! commands so audit replay can correlate goal lifecycle events with session wakeups.
//!
//! # Architecture (Facade + Command)
//!
//! - **Command**: [`GoalLifecycleParentWaitRequest`], [`GoalLifecycleParentResumeRequest`]
//!   capture provider-neutral inputs (no application role names).
//! - **Facade**: [`ExecutionControlGoalLifecycleCoordinator`] owns the service-call
//!   choreography for register/pause/await/resume.
//! - **Observer**: PlanLoop `GoalCompleted` consumers and shell adapters call
//!   `deliver_parent_goal_resume` when task service signals goal completion.
//!
//! Shell adapters retain local in-memory resume channels until loop_manager fully
//! subscribes to execution-control events (tasks 2.3.4 / 4.2).

use std::sync::Arc;

use macaca_proto::{
    ApplicationId, ExecutionControlAwaitResumeCommand, ExecutionControlCheckpointMode,
    ExecutionControlCommandResult, ExecutionControlPauseCommand, ExecutionControlPolicy,
    ExecutionControlRegisterExecutionCommand, ExecutionControlResolutionStatus,
    ExecutionControlResumeCommand, ExecutionControlResumeSource, ExecutionControlScope,
    ExecutionControlTrigger, KernelServiceId, MacacaResult, ServiceBusSource, TaskId, TraceContext,
    EXECUTION_CONTROL_SERVICE_ID,
};
use tracing::{info, warn};

use crate::execution_control::ExecutionControlPolicyResolver;
use crate::ServiceRuntime;

/// Stable bus source label for goal-lifecycle execution-control commands.
pub const GOAL_LIFECYCLE_EXECUTION_CONTROL_SOURCE: &str = "macaca.runtime_host.goal_lifecycle";

/// Provider-neutral request to register a parent pause while a goal executes.
#[derive(Debug, Clone)]
pub struct GoalLifecycleParentWaitRequest {
    pub application_id: ApplicationId,
    pub session_id: String,
    /// Agent label for the parent execution that is waiting (generic, not a role name).
    pub parent_agent: String,
    pub goal_id: TaskId,
    pub trace: TraceContext,
}

/// Provider-neutral request to deliver a goal-lifecycle resume to a paused parent.
#[derive(Debug, Clone)]
pub struct GoalLifecycleParentResumeRequest {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub parent_agent: String,
    pub goal_id: TaskId,
    pub trace: TraceContext,
    /// Provider-neutral reason code for audit replay (for example success vs failure).
    pub reason_code: String,
    /// Optional human-readable completion summary for audit metadata (bounded by callers).
    pub completion_summary: Option<String>,
}

/// Facade that routes goal-lifecycle pause/resume through `service.execution_control`.
pub struct ExecutionControlGoalLifecycleCoordinator {
    service_runtime: Arc<ServiceRuntime>,
    bus_source: ServiceBusSource,
}

impl ExecutionControlGoalLifecycleCoordinator {
    /// Bind the coordinator to the shared host service runtime.
    pub fn new(service_runtime: Arc<ServiceRuntime>) -> Self {
        Self {
            service_runtime,
            bus_source: ServiceBusSource::new(GOAL_LIFECYCLE_EXECUTION_CONTROL_SOURCE),
        }
    }

    /// Build the execution-control registration id for one parent goal wait.
    ///
    /// The id is deterministic from session + goal so replay and diagnostics can
    /// correlate parent pause boundaries with PlanLoop completion events.
    pub fn parent_execution_id(session_id: &str, goal_id: TaskId) -> String {
        format!("goal-lifecycle:{session_id}:{}", goal_id.0)
    }

    /// Build the default goal-lifecycle policy used when applications have not
    /// declared execution-control defaults in their manifest projection.
    pub fn goal_lifecycle_policy() -> ExecutionControlPolicy {
        ExecutionControlPolicy::enabled(
            vec![ExecutionControlTrigger::GoalLifecycle],
            vec![ExecutionControlResumeSource::goal_lifecycle()],
            ExecutionControlCheckpointMode::ReferenceOnly,
        )
    }

    /// Build the provider-neutral execution-control scope for a parent goal wait.
    pub fn parent_wait_scope(
        request: &GoalLifecycleParentWaitRequest,
    ) -> MacacaResult<ExecutionControlScope> {
        let execution_id = Self::parent_execution_id(&request.session_id, request.goal_id);
        let mut scope = ExecutionControlScope::new(
            request.application_id,
            request.session_id.clone(),
            execution_id,
            request.trace.clone(),
            GOAL_LIFECYCLE_EXECUTION_CONTROL_SOURCE,
        )?;
        scope.task_id = Some(request.goal_id);
        scope
            .metadata
            .insert("parent_agent".into(), request.parent_agent.clone());
        scope
            .metadata
            .insert("goal_id".into(), request.goal_id.0.to_string());
        scope.metadata.insert(
            "graph_owner".into(),
            "execution_control.goal_lifecycle.parent".into(),
        );
        Ok(scope)
    }

    /// Register a parent pause boundary and record await intent via execution control.
    ///
    /// This emits durable service-call evidence when `create_goal` records a
    /// session-bound goal.  The shell adapter may still block on local resume
    /// channels until loop_manager fully consumes execution-control events.
    pub async fn register_parent_goal_wait(
        &self,
        request: GoalLifecycleParentWaitRequest,
    ) -> Result<ExecutionControlCommandResult, String> {
        let scope = Self::parent_wait_scope(&request).map_err(|error| error.to_string())?;
        let resolved =
            ExecutionControlPolicyResolver::resolve(Some(&Self::goal_lifecycle_policy()), None);
        if resolved.status != ExecutionControlResolutionStatus::Enabled {
            return Err(format!(
                "goal-lifecycle execution control denied: {}",
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

        let pause = self
            .call_execution_control(
                ExecutionControlPauseCommand {
                    scope: scope.clone(),
                    trigger: ExecutionControlTrigger::GoalLifecycle,
                    reason_code: "execution_control.goal_lifecycle.parent_pause".into(),
                }
                .into_service_command()
                .map_err(|error| error.to_string())?,
            )
            .await?;

        let _await_resume = self
            .call_execution_control(
                ExecutionControlAwaitResumeCommand {
                    scope,
                    resume_sources: vec![ExecutionControlResumeSource::goal_lifecycle()],
                    timeout_ms: None,
                }
                .into_service_command()
                .map_err(|error| error.to_string())?,
            )
            .await?;

        info!(
            session_id = %request.session_id,
            goal_id = %request.goal_id.0,
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            reason_code = %pause.reason_code,
            "Registered parent goal-lifecycle pause boundary via execution control"
        );

        Ok(register)
    }

    /// Deliver a goal-lifecycle resume signal to a paused parent execution.
    pub async fn deliver_parent_goal_resume(
        &self,
        request: GoalLifecycleParentResumeRequest,
    ) -> Result<ExecutionControlCommandResult, String> {
        let execution_id = Self::parent_execution_id(&request.session_id, request.goal_id);
        let mut scope = ExecutionControlScope::new(
            request.application_id,
            request.session_id.clone(),
            execution_id,
            request.trace.clone(),
            GOAL_LIFECYCLE_EXECUTION_CONTROL_SOURCE,
        )
        .map_err(|error| error.to_string())?;
        scope.task_id = Some(request.goal_id);
        scope
            .metadata
            .insert("parent_agent".into(), request.parent_agent);
        scope
            .metadata
            .insert("goal_id".into(), request.goal_id.0.to_string());
        if let Some(summary) = request.completion_summary {
            scope.metadata.insert(
                "completion_summary".into(),
                summary.chars().take(200).collect(),
            );
        }

        let result = self
            .call_execution_control(
                ExecutionControlResumeCommand {
                    scope,
                    resume_source: ExecutionControlResumeSource::goal_lifecycle(),
                    reason_code: request.reason_code,
                }
                .into_service_command()
                .map_err(|error| error.to_string())?,
            )
            .await?;

        info!(
            session_id = %request.session_id,
            goal_id = %request.goal_id.0,
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            state = ?result.state,
            "Delivered parent goal-lifecycle resume via execution control"
        );
        Ok(result)
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
                "execution control service returned non-success for goal-lifecycle command"
            );
            return Err(format!(
                "execution control service rejected goal-lifecycle command: {}",
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
    use macaca_proto::ExecutionControlState;

    fn sample_parent_wait() -> GoalLifecycleParentWaitRequest {
        GoalLifecycleParentWaitRequest {
            application_id: ApplicationId::new(),
            session_id: "session-goal-lifecycle".into(),
            parent_agent: "parent-agent".into(),
            goal_id: TaskId::new(),
            trace: TraceContext::new("trace-goal-lifecycle-parent"),
        }
    }

    #[test]
    fn parent_execution_id_is_deterministic() {
        let goal_id = TaskId::new();
        let id =
            ExecutionControlGoalLifecycleCoordinator::parent_execution_id("session-a", goal_id);
        assert_eq!(id, format!("goal-lifecycle:session-a:{}", goal_id.0));
    }

    #[test]
    fn goal_lifecycle_policy_uses_generic_trigger_and_resume_source() {
        let policy = ExecutionControlGoalLifecycleCoordinator::goal_lifecycle_policy();
        assert!(policy
            .triggers
            .contains(&ExecutionControlTrigger::GoalLifecycle));
        assert!(policy
            .resume_sources
            .contains(&ExecutionControlResumeSource::goal_lifecycle()));
    }

    #[test]
    fn parent_wait_scope_carries_goal_metadata() {
        let request = sample_parent_wait();
        let scope = ExecutionControlGoalLifecycleCoordinator::parent_wait_scope(&request).unwrap();
        assert_eq!(scope.session_id, "session-goal-lifecycle");
        assert_eq!(scope.task_id, Some(request.goal_id));
        assert_eq!(
            scope.metadata.get("goal_id").map(String::as_str),
            Some(request.goal_id.0.to_string().as_str())
        );
    }

    #[test]
    fn policy_resolver_enables_goal_lifecycle_defaults() {
        let resolved = ExecutionControlPolicyResolver::resolve(
            Some(&ExecutionControlGoalLifecycleCoordinator::goal_lifecycle_policy()),
            None,
        );
        assert_eq!(resolved.status, ExecutionControlResolutionStatus::Enabled);
    }

    #[test]
    fn runtime_capability_replays_goal_lifecycle_chain() {
        use crate::execution_control_runtime::ExecutionControlRuntimeCapability;

        let runtime = ExecutionControlRuntimeCapability::new();
        let request = sample_parent_wait();
        let scope = ExecutionControlGoalLifecycleCoordinator::parent_wait_scope(&request).unwrap();
        let policy = macaca_proto::ExecutionControlResolvedPolicy {
            status: ExecutionControlResolutionStatus::Enabled,
            policy: Some(ExecutionControlGoalLifecycleCoordinator::goal_lifecycle_policy()),
            reason_code: "execution_control.enabled.goal_lifecycle.test".into(),
            metadata: Default::default(),
            events: Vec::new(),
        };

        runtime
            .register_execution(ExecutionControlRegisterExecutionCommand {
                scope: scope.clone(),
                policy,
            })
            .unwrap();
        runtime
            .request_pause(ExecutionControlPauseCommand {
                scope: scope.clone(),
                trigger: ExecutionControlTrigger::GoalLifecycle,
                reason_code: "execution_control.goal_lifecycle.parent_pause".into(),
            })
            .unwrap();
        let resume = runtime
            .request_resume(ExecutionControlResumeCommand {
                scope,
                resume_source: ExecutionControlResumeSource::goal_lifecycle(),
                reason_code: "execution_control.goal_lifecycle.parent_resume".into(),
            })
            .unwrap();

        assert_eq!(resume.state, ExecutionControlState::Resuming);
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.executions.len(), 1);
        assert!(snapshot.executions[0]
            .events
            .iter()
            .any(|event| event.reason_code == "execution_control.pause_entered"));
    }
}
