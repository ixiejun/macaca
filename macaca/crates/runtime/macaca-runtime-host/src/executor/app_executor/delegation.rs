//! Task delegation entry points — worker command path and service-backed path.
//!
//! Implements the Template Method split between kernel admission/lifecycle events
//! and runtime-host agent execution via the unified service boundary.

use tracing::{error, info, warn};

use macaca_proto::TaskId as ProtoTaskId;

use super::executor::ApplicationExecutor;
use crate::executor::{
    AgentInfo, AgentRunner, ApplicationId, CallbackDispatcher, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkManager,
    RoutingDecision, SystemEvent, TaskContext, TaskId, TaskResult, TaskRouter, TaskStatus,
};

impl ApplicationExecutor {
    /// Delegate a task to an agent.
    ///
    /// This is the main entry point for task delegation. The task will be
    /// queued and executed asynchronously.
    pub async fn delegate_task(
        &self,
        from_agent: &str,
        to_agent: &str,
        prompt: String,
        priority: u8,
        parallel: bool,
        context: Option<TaskContext>,
    ) -> Result<TaskId, String> {
        // Verify the target agent exists
        let agents = self.agents.read().await;
        let target_exists = agents.iter().any(|a| a.name == to_agent);
        if !target_exists {
            return Err(format!(
                "Agent '{}' not found in application '{}'",
                to_agent, self.application_id
            ));
        }
        drop(agents);

        let task = DelegatedTask {
            id: TaskId::new(),
            application_id: self.application_id.clone(),
            from_agent: from_agent.to_string(),
            to_agent: to_agent.to_string(),
            prompt,
            priority,
            parallel,
            created_at: chrono::Utc::now(),
            deadline: None,
            parent_task: None,
            context,
        };

        let task_id = task.id;

        // Enqueue the task
        self.queue
            .enqueue(task.clone())
            .await
            .map_err(|e| format!("Failed to enqueue task: {}", e))?;

        // Publish task delegated event
        self.event_bus
            .emit(SystemEvent::TaskDelegated {
                task_id: task.id,
                application_id: self.application_id.clone(),
                from_agent: from_agent.to_string(),
                to_agent: to_agent.to_string(),
                prompt: task.prompt.clone(),
            })
            .await;

        // Send execute command to worker
        let command_tx = self.command_tx.read().await;
        info!(
            application_id = %self.application_id,
            task_id = %task_id,
            from_agent = %from_agent,
            to_agent = %to_agent,
            channel_capacity = command_tx.capacity(),
            "Sending execute command to worker"
        );

        // Check if sender is still valid before sending
        if command_tx.is_closed() {
            error!(
                application_id = %self.application_id,
                task_id = %task_id,
                "Command channel is closed before sending!"
            );
        }

        match command_tx.send(ExecutorCommand::Execute(task)).await {
            Ok(_) => {
                info!(
                    application_id = %self.application_id,
                    task_id = %task_id,
                    "Execute command sent successfully"
                );
            }
            Err(e) => {
                error!(
                    application_id = %self.application_id,
                    task_id = %task_id,
                    error = %e,
                    "Failed to send execute command"
                );
                return Err(format!("Failed to send execute command: {}", e));
            }
        }

        Ok(task_id)
    }

    /// Begin tracking a delegated task that will execute through `service.agent_execution`.
    ///
    /// This is the transitional registration path used while orchestration migrates off the
    /// kernel worker command channel.  The caller owns the service invocation and must
    /// finalize the task through [`Self::complete_service_backed_delegation`].
    ///
    /// # Design note (Template Method split)
    ///
    /// - **Kernel executor**: task admission, lifecycle events, fork resume hooks.
    /// - **Runtime-host / shell**: actual agent execution via the unified service boundary.
    pub async fn begin_service_backed_delegation(
        &self,
        from_agent: &str,
        to_agent: &str,
        prompt: String,
        priority: u8,
        parallel: bool,
        context: Option<TaskContext>,
    ) -> Result<TaskId, String> {
        let agents = self.agents.read().await;
        let target_exists = agents.iter().any(|a| a.name == to_agent);
        if !target_exists {
            return Err(format!(
                "Agent '{}' not found in application '{}'",
                to_agent, self.application_id
            ));
        }
        drop(agents);

        let task = DelegatedTask {
            id: TaskId::new(),
            application_id: self.application_id.clone(),
            from_agent: from_agent.to_string(),
            to_agent: to_agent.to_string(),
            prompt: prompt.clone(),
            priority,
            parallel,
            created_at: chrono::Utc::now(),
            deadline: None,
            parent_task: None,
            context,
        };
        let task_id = task.id;

        self.queue
            .admit_running_task(task)
            .await
            .map_err(|error| format!("Failed to admit service-backed task: {error}"))?;

        self.event_bus
            .emit(SystemEvent::TaskDelegated {
                task_id,
                application_id: self.application_id.clone(),
                from_agent: from_agent.to_string(),
                to_agent: to_agent.to_string(),
                prompt,
            })
            .await;

        let started = ExecutorEventFactory::new(task_id, to_agent).started();
        self.broadcast_event(started);

        info!(
            application_id = %self.application_id,
            task_id = %task_id,
            from_agent = %from_agent,
            to_agent = %to_agent,
            "Service-backed delegation admitted without worker command dispatch"
        );

        Ok(task_id)
    }

    /// Finalize a service-backed delegation and resume any fork waiting on the task.
    ///
    /// Mirrors the worker completion path so `get_task_result`, SSE subscribers, and
    /// fork-join orchestration observe identical semantics regardless of execution transport.
    pub async fn complete_service_backed_delegation(
        &self,
        task_id: TaskId,
        agent_name: &str,
        execution_result: Result<TaskResult, String>,
    ) {
        let events = ExecutorEventFactory::new(task_id, agent_name);
        match execution_result {
            Ok(mut task_result) => {
                task_result.task_id = task_id;
                info!(
                    application_id = %self.application_id,
                    task_id = %task_id,
                    agent = %agent_name,
                    success = task_result.success,
                    "Service-backed delegation completed"
                );

                self.queue.store_result(task_result.clone()).await;
                self.queue.complete(task_id, task_result.clone()).await;

                let delegate_result = crate::executor::fork_manager::DelegateResult {
                    task_id: ProtoTaskId(task_id.0),
                    success: task_result.success,
                    output: task_result.output.clone(),
                    error: task_result.error.clone(),
                    artifacts: task_result.artifacts.clone(),
                };
                if let Err(error) = self
                    .fork_manager
                    .resume_fork_by_task(ProtoTaskId(task_id.0), delegate_result)
                    .await
                {
                    warn!(
                        application_id = %self.application_id,
                        task_id = %task_id,
                        error = %error,
                        "Failed to resume fork waiting on service-backed task"
                    );
                }

                self.broadcast_event(events.completed_with_result(task_result));
            }
            Err(error) => {
                error!(
                    application_id = %self.application_id,
                    task_id = %task_id,
                    agent = %agent_name,
                    error = %error,
                    "Service-backed delegation failed"
                );
                let error_result = events.failed_result(error.clone());
                self.queue.store_result(error_result).await;
                self.queue.fail(task_id, error.clone()).await;
                self.broadcast_event(events.failed(error));
            }
        }
    }

    /// Route a task to the best available agent.
    ///
    /// Instead of specifying the agent, let the router decide based on
    /// the prompt content and agent capabilities.
    pub async fn route_and_delegate(
        &self,
        from_agent: &str,
        prompt: String,
        priority: u8,
        parallel: bool,
        context: Option<TaskContext>,
    ) -> Result<(TaskId, RoutingDecision), String> {
        // Create a temporary task for routing
        let temp_task = DelegatedTask {
            id: TaskId::new(),
            application_id: self.application_id.clone(),
            from_agent: from_agent.to_string(),
            to_agent: String::new(), // Empty - let router decide
            prompt: prompt.clone(),
            priority,
            parallel,
            created_at: chrono::Utc::now(),
            deadline: None,
            parent_task: None,
            context: context.clone(),
        };

        // Get the routing decision
        let decision = self.router.route(&temp_task).await;

        // Check if routing was successful
        if decision.confidence <= 0.0 {
            return Err(format!(
                "No suitable agent found for prompt: {}",
                decision.reasoning
            ));
        }

        // Delegate to the selected agent
        let task_id = self
            .delegate_task(
                from_agent,
                &decision.agent_name,
                prompt,
                priority,
                parallel,
                context,
            )
            .await?;

        Ok((task_id, decision))
    }
}
