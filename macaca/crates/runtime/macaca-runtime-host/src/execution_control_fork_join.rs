//! Fork-Join orchestration facade backed by `service.execution_control`.
//!
//! Fork-Join coordinates two generic execution facts:
//! 1. A child fork runs delegated work (still owned by kernel `ForkManager` until P3
//!    fully migrates fork state).
//! 2. The parent execution must pause until the fork lifecycle reaches a terminal hook.
//!
//! # Architecture (Facade + Command)
//!
//! - **Command**: [`ForkJoinChildForkRequest`], [`ForkJoinParentWaitRequest`],
//!   [`ForkJoinParentResumeRequest`] capture provider-neutral inputs.
//! - **Facade**: [`ExecutionControlForkJoinCoordinator`] owns the service-call
//!   choreography for pause/resume registration and delivery.
//! - **Collaborator**: kernel [`ForkManager`] still owns fork create/start/suspend until
//!   executor eviction (task 3.4); this module never embeds application workflow names.
//!
//! Shell adapters (web `orchestration_tools`, `hook_consumer`) call this facade so
//! pause/resume evidence is emitted through `service.execution_control` instead of
//! writing directly to in-memory pause channels without audit records.

use std::sync::Arc;

use macaca_kernel::executor::ForkManager;
use macaca_proto::{
    AcceptanceCriteria, ApplicationId, ExecutionControlAwaitResumeCommand,
    ExecutionControlCheckpointMode, ExecutionControlCommandResult, ExecutionControlPauseCommand,
    ExecutionControlPolicy, ExecutionControlRegisterExecutionCommand,
    ExecutionControlResolutionStatus, ExecutionControlResolvedPolicy,
    ExecutionControlResumeCommand, ExecutionControlResumeSource, ExecutionControlScope,
    ExecutionControlTrigger, ForkId, KernelServiceId, LlmMessage, MacacaResult, ServiceBusSource,
    TaskId, TraceContext, EXECUTION_CONTROL_SERVICE_ID,
};
use tracing::{info, warn};

use crate::execution_control::ExecutionControlPolicyResolver;
use crate::ServiceRuntime;

/// Stable bus source label for fork-join execution-control commands.
pub const FORK_JOIN_EXECUTION_CONTROL_SOURCE: &str = "macaca.runtime_host.fork_join";

/// Provider-neutral request to create and start a child fork for delegated work.
#[derive(Debug, Clone)]
pub struct ForkJoinChildForkRequest {
    /// Application scope for the fork.
    pub application_id: ApplicationId,
    /// Agent that will execute the delegated prompt inside the fork.
    pub target_agent: String,
    /// Delegated work prompt.
    pub task_prompt: String,
    /// Optional inherited conversation tail for the fork context.
    pub inherited_messages: Vec<LlmMessage>,
    /// Inherited system prompt for the fork context.
    pub system_prompt: String,
    /// Validation contract applied when the fork completes.
    pub acceptance_criteria: AcceptanceCriteria,
}

/// Provider-neutral request to register a parent pause boundary while a fork runs.
#[derive(Debug, Clone)]
pub struct ForkJoinParentWaitRequest {
    pub application_id: ApplicationId,
    pub session_id: String,
    /// Agent label for the parent execution that is waiting (generic, not a role name).
    pub parent_agent: String,
    pub fork_id: ForkId,
    pub delegate_task_id: TaskId,
    pub trace: TraceContext,
}

/// Provider-neutral request to deliver a fork-lifecycle resume to a paused parent.
#[derive(Debug, Clone)]
pub struct ForkJoinParentResumeRequest {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub parent_agent: String,
    pub fork_id: ForkId,
    pub delegate_task_id: Option<TaskId>,
    pub trace: TraceContext,
    /// Provider-neutral reason code for audit replay (for example success vs failure).
    pub reason_code: String,
}

/// Facade that routes fork-join pause/resume through `service.execution_control`.
pub struct ExecutionControlForkJoinCoordinator {
    service_runtime: Arc<ServiceRuntime>,
    bus_source: ServiceBusSource,
}

impl ExecutionControlForkJoinCoordinator {
    /// Bind the coordinator to the shared host service runtime.
    pub fn new(service_runtime: Arc<ServiceRuntime>) -> Self {
        Self {
            service_runtime,
            bus_source: ServiceBusSource::new(FORK_JOIN_EXECUTION_CONTROL_SOURCE),
        }
    }

    /// Build the execution-control registration id for one parent fork wait.
    ///
    /// The id is deterministic from session + fork so replay and diagnostics can
    /// correlate parent pause boundaries with kernel hook events.
    pub fn parent_execution_id(session_id: &str, fork_id: ForkId) -> String {
        format!("fork-join:{session_id}:{}", fork_id.0)
    }

    /// Build the default fork-lifecycle policy used when applications have not
    /// declared execution-control defaults in their manifest projection.
    pub fn fork_lifecycle_policy() -> ExecutionControlPolicy {
        ExecutionControlPolicy::enabled(
            vec![ExecutionControlTrigger::ForkLifecycle],
            vec![ExecutionControlResumeSource::ForkLifecycle],
            ExecutionControlCheckpointMode::ReferenceOnly,
        )
    }

    /// Build the provider-neutral execution-control scope for a parent fork wait.
    pub fn parent_wait_scope(
        request: &ForkJoinParentWaitRequest,
    ) -> MacacaResult<ExecutionControlScope> {
        let execution_id = Self::parent_execution_id(&request.session_id, request.fork_id);
        let mut scope = ExecutionControlScope::new(
            request.application_id,
            request.session_id.clone(),
            execution_id,
            request.trace.clone(),
            FORK_JOIN_EXECUTION_CONTROL_SOURCE,
        )?;
        scope.task_id = Some(request.delegate_task_id);
        scope
            .metadata
            .insert("parent_agent".into(), request.parent_agent.clone());
        scope
            .metadata
            .insert("fork_id".into(), request.fork_id.0.to_string());
        scope.metadata.insert(
            "graph_owner".into(),
            "execution_control.fork_join.parent".into(),
        );
        Ok(scope)
    }

    /// Create and start a child fork through the kernel fork manager.
    ///
    /// Fork state remains in `ForkManager` for this migration stage; only pause/resume
    /// ownership moves behind execution-control service commands.
    pub async fn create_and_start_child_fork(
        fork_manager: &ForkManager,
        request: ForkJoinChildForkRequest,
    ) -> Result<ForkId, String> {
        let fork_id = fork_manager
            .create_fork(
                None,
                request.application_id,
                request.target_agent,
                request.task_prompt,
                request.inherited_messages,
                request.system_prompt,
                request.acceptance_criteria,
            )
            .await?;
        fork_manager.start_fork(fork_id).await?;
        info!(
            application_id = %request.application_id.0,
            fork_id = %fork_id.0,
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            "Fork-join child fork created and started"
        );
        Ok(fork_id)
    }

    /// Suspend a child fork while the delegated task executes through the service path.
    pub async fn suspend_child_fork(
        fork_manager: &ForkManager,
        fork_id: ForkId,
        delegate_task_id: TaskId,
    ) -> Result<(), String> {
        fork_manager
            .suspend_fork(fork_id, delegate_task_id)
            .await?;
        info!(
            fork_id = %fork_id.0,
            task_id = %delegate_task_id.0,
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            "Fork-join child fork suspended on delegated task"
        );
        Ok(())
    }

    /// Register a parent pause boundary and record await intent via execution control.
    ///
    /// This emits durable service-call evidence before the shell adapter blocks on
    /// its local resume channel.  The local channel remains a host concern until loop
    /// manager fully consumes execution-control events (task 2.3.4 / 4.2).
    pub async fn register_parent_fork_wait(
        &self,
        request: ForkJoinParentWaitRequest,
    ) -> Result<ExecutionControlCommandResult, String> {
        let scope = Self::parent_wait_scope(&request).map_err(|error| error.to_string())?;
        let resolved = ExecutionControlPolicyResolver::resolve(
            Some(&Self::fork_lifecycle_policy()),
            None,
        );
        if resolved.status != ExecutionControlResolutionStatus::Enabled {
            return Err(format!(
                "fork-join execution control denied: {}",
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
                    trigger: ExecutionControlTrigger::ForkLifecycle,
                    reason_code: "execution_control.fork_join.parent_pause".into(),
                }
                .into_service_command()
                .map_err(|error| error.to_string())?,
            )
            .await?;

        let _await_resume = self
            .call_execution_control(
                ExecutionControlAwaitResumeCommand {
                    scope,
                    resume_sources: vec![ExecutionControlResumeSource::ForkLifecycle],
                    timeout_ms: None,
                }
                .into_service_command()
                .map_err(|error| error.to_string())?,
            )
            .await?;

        info!(
            session_id = %request.session_id,
            fork_id = %request.fork_id.0,
            task_id = %request.delegate_task_id.0,
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            reason_code = %pause.reason_code,
            "Registered parent fork-join pause boundary via execution control"
        );

        Ok(register)
    }

    /// Deliver a fork-lifecycle resume signal to a paused parent execution.
    pub async fn deliver_parent_fork_resume(
        &self,
        request: ForkJoinParentResumeRequest,
    ) -> Result<ExecutionControlCommandResult, String> {
        let execution_id = Self::parent_execution_id(&request.session_id, request.fork_id);
        let mut scope = ExecutionControlScope::new(
            request.application_id,
            request.session_id.clone(),
            execution_id,
            request.trace.clone(),
            FORK_JOIN_EXECUTION_CONTROL_SOURCE,
        )
        .map_err(|error| error.to_string())?;
        scope.task_id = request.delegate_task_id;
        scope
            .metadata
            .insert("parent_agent".into(), request.parent_agent);
        scope
            .metadata
            .insert("fork_id".into(), request.fork_id.0.to_string());

        let result = self
            .call_execution_control(
                ExecutionControlResumeCommand {
                    scope,
                    resume_source: ExecutionControlResumeSource::ForkLifecycle,
                    reason_code: request.reason_code,
                }
                .into_service_command()
                .map_err(|error| error.to_string())?,
            )
            .await?;

        info!(
            session_id = %request.session_id,
            fork_id = %request.fork_id.0,
            service_id = EXECUTION_CONTROL_SERVICE_ID,
            state = ?result.state,
            "Delivered parent fork-join resume via execution control"
        );
        Ok(result)
    }

    /// Lookup a fork snapshot for result polling tools.
    pub async fn get_fork(
        fork_manager: &ForkManager,
        fork_id: ForkId,
    ) -> Option<macaca_kernel::executor::fork_manager::ForkContext> {
        fork_manager.get_fork(fork_id).await
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
                "execution control service returned non-success for fork-join command"
            );
            return Err(format!(
                "execution control service rejected fork-join command: {}",
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

    fn sample_parent_wait() -> ForkJoinParentWaitRequest {
        ForkJoinParentWaitRequest {
            application_id: ApplicationId::new(),
            session_id: "session-fork-join".into(),
            parent_agent: "parent-agent".into(),
            fork_id: ForkId::new(),
            delegate_task_id: TaskId::new(),
            trace: TraceContext::new("trace-fork-join-parent"),
        }
    }

    #[test]
    fn parent_execution_id_is_deterministic() {
        let fork_id = ForkId::new();
        let id = ExecutionControlForkJoinCoordinator::parent_execution_id("session-a", fork_id);
        assert_eq!(id, format!("fork-join:session-a:{}", fork_id.0));
    }

    #[test]
    fn fork_lifecycle_policy_uses_generic_trigger_and_resume_source() {
        let policy = ExecutionControlForkJoinCoordinator::fork_lifecycle_policy();
        assert!(policy
            .triggers
            .contains(&ExecutionControlTrigger::ForkLifecycle));
        assert!(policy
            .resume_sources
            .contains(&ExecutionControlResumeSource::ForkLifecycle));
    }

    #[test]
    fn parent_wait_scope_carries_fork_and_task_metadata() {
        let request = sample_parent_wait();
        let scope = ExecutionControlForkJoinCoordinator::parent_wait_scope(&request).unwrap();
        assert_eq!(scope.session_id, "session-fork-join");
        assert_eq!(scope.task_id, Some(request.delegate_task_id));
        assert_eq!(
            scope.metadata.get("fork_id").map(String::as_str),
            Some(request.fork_id.0.to_string().as_str())
        );
    }

    #[test]
    fn policy_resolver_enables_fork_lifecycle_defaults() {
        let resolved = ExecutionControlPolicyResolver::resolve(
            Some(&ExecutionControlForkJoinCoordinator::fork_lifecycle_policy()),
            None,
        );
        assert_eq!(resolved.status, ExecutionControlResolutionStatus::Enabled);
    }

    #[test]
    fn runtime_capability_replays_fork_lifecycle_chain() {
        use crate::execution_control_runtime::ExecutionControlRuntimeCapability;

        let runtime = ExecutionControlRuntimeCapability::new();
        let request = sample_parent_wait();
        let scope = ExecutionControlForkJoinCoordinator::parent_wait_scope(&request).unwrap();
        let policy = ExecutionControlResolvedPolicy {
            status: ExecutionControlResolutionStatus::Enabled,
            policy: Some(ExecutionControlForkJoinCoordinator::fork_lifecycle_policy()),
            reason_code: "execution_control.enabled.fork_join.test".into(),
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
                trigger: ExecutionControlTrigger::ForkLifecycle,
                reason_code: "execution_control.fork_join.parent_pause".into(),
            })
            .unwrap();
        let resume = runtime
            .request_resume(ExecutionControlResumeCommand {
                scope,
                resume_source: ExecutionControlResumeSource::ForkLifecycle,
                reason_code: "execution_control.fork_join.parent_resume".into(),
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
