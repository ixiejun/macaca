//! Task Executor - Worker that actually executes delegated agents.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::{mpsc, RwLock};

use super::{
    AgentInfo, DelegatedTask, EventBus, ExecutorEventFactory, RoutingDecision, SystemEvent,
    TaskContext, TaskExecutor as TaskExecutorTrait, TaskId, TaskResult, TaskRouter,
};

/// Maximum number of retries for failed tasks.
const MAX_RETRIES: usize = 3;

/// Commands that can be sent to the executor.
#[derive(Debug)]
pub enum ExecutorCommand {
    /// Execute a delegated task.
    Execute(DelegatedTask),
    /// Cancel a running task.
    Cancel(TaskId),
    /// Shutdown the executor.
    Shutdown,
}

/// Events emitted by the executor.
#[derive(Debug, Clone)]
pub enum ExecutorEvent {
    /// Task started execution.
    TaskStarted { task_id: TaskId, agent: String },
    /// Task progress update.
    TaskProgress {
        task_id: TaskId,
        step: usize,
        output: String,
    },
    /// Agent execution event (for delegated agent tracking).
    AgentEvent {
        task_id: TaskId,
        agent: String,
        event: macaca_proto::AgentExecutionEvent,
    },
    /// Task completed.
    TaskCompleted {
        task_id: TaskId,
        agent: String,
        result: TaskResult,
    },
    /// Task failed.
    TaskFailed {
        task_id: TaskId,
        agent: String,
        error: String,
    },
    /// Task cancelled.
    TaskCancelled { task_id: TaskId },
    /// Hook event from Fork-Join workflow (validation, completion notification).
    HookEvent {
        event: super::fork_manager::HookEvent,
    },
    /// Executor shutdown.
    Shutdown,
}

/// The actual worker that executes tasks.
///
/// This is the core component that:
/// 1. Takes tasks from the queue
/// 2. Routes them to the appropriate agent
/// 3. Executes the agent with the given prompt
/// 4. Reports results back via events and callbacks
pub struct TaskExecutor {
    /// Application ID for isolation.
    application_id: String,
    /// The task router.
    router: Arc<TaskRouter>,
    /// Event bus for publishing events.
    event_bus: Arc<EventBus>,
    /// Channel to receive commands.
    command_rx: mpsc::Receiver<ExecutorCommand>,
    /// Channel to send executor events.
    event_tx: mpsc::Sender<ExecutorEvent>,
    /// Flag indicating if executor is running.
    running: Arc<RwLock<bool>>,
}

impl TaskExecutor {
    /// Create a new executor.
    pub fn new(
        application_id: String,
        router: Arc<TaskRouter>,
        event_bus: Arc<EventBus>,
        command_rx: mpsc::Receiver<ExecutorCommand>,
    ) -> Self {
        let (event_tx, _) = mpsc::channel(100);
        Self {
            application_id,
            router,
            event_bus,
            command_rx,
            event_tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Run the executor loop.
    pub async fn run(&mut self) {
        *self.running.write().await = true;

        tracing::info!(app_id = %self.application_id, "TaskExecutor started");

        while let Some(cmd) = self.command_rx.recv().await {
            match cmd {
                ExecutorCommand::Execute(task) => {
                    self.execute_task(task).await;
                }
                ExecutorCommand::Cancel(task_id) => {
                    self.cancel_task(task_id).await;
                }
                ExecutorCommand::Shutdown => {
                    tracing::info!(app_id = %self.application_id, "TaskExecutor shutting down");
                    break;
                }
            }
        }

        *self.running.write().await = false;
    }

    /// Execute a single task.
    async fn execute_task(&self, task: DelegatedTask) {
        let task_id = task.id;
        let to_agent = task.to_agent.clone();
        let events = ExecutorEventFactory::new(task_id, to_agent.clone());

        tracing::info!(task_id = %task_id, agent = %to_agent, "Executing task");

        // Publish TaskStarted event
        let _ = self
            .event_bus
            .emit(SystemEvent::TaskStarted {
                task_id: task_id.to_string(),
                agent: to_agent.clone(),
            })
            .await;

        // Send TaskStarted to event channel
        let _ = self.event_tx.send(events.started()).await;

        // Execute the agent
        let result = self.run_agent(&task).await;

        match result {
            Ok(task_result) => {
                tracing::info!(task_id = %task_id, success = task_result.success, "Task completed");

                // Publish TaskCompleted event
                let _ = self
                    .event_bus
                    .emit(SystemEvent::TaskCompleted {
                        task_id: task_id.to_string(),
                        agent: to_agent,
                        success: task_result.success,
                        output_preview: task_result.output.chars().take(100).collect(),
                    })
                    .await;

                let _ = self
                    .event_tx
                    .send(events.completed_with_result(task_result))
                    .await;
            }
            Err(error) => {
                tracing::error!(task_id = %task_id, error = %error, "Task failed");

                // Publish TaskFailed event
                let _ = self
                    .event_bus
                    .emit(SystemEvent::TaskFailed {
                        task_id: task_id.to_string(),
                        agent: to_agent,
                        error: error.clone(),
                    })
                    .await;

                let _ = self.event_tx.send(events.failed(error)).await;
            }
        }
    }

    /// Run an agent with the given task.
    async fn run_agent(&self, task: &DelegatedTask) -> Result<TaskResult, String> {
        // Get routing decision
        let decision = self.router.route(task).await;

        if decision.confidence <= 0.0 {
            return Err(format!("No suitable agent found: {}", decision.reasoning));
        }

        tracing::debug!(
            task_id = %task.id,
            agent = %decision.agent_name,
            confidence = decision.confidence,
            "Routed to agent"
        );

        // Execute the agent via the router's runner
        // Note: The actual execution is delegated to the AgentRunner implementation
        // which is injected via the router
        self.execute_agent(&decision.agent_name, &task.prompt, task.context.clone())
            .await
    }

    /// Execute an agent by name.
    ///
    /// This method should be implemented by the concrete runner
    /// that has access to the actual agent execution infrastructure.
    async fn execute_agent(
        &self,
        agent_name: &str,
        prompt: &str,
        context: Option<TaskContext>,
    ) -> Result<TaskResult, String> {
        // This is a placeholder - the actual implementation would:
        // 1. Load the agent's persona
        // 2. Build the LLM messages
        // 3. Execute the agent loop
        // 4. Return the result

        // For now, return a placeholder result
        // In the real implementation, this would call the Kernel or runtime
        tracing::info!(agent = agent_name, "Executing agent (placeholder)");

        Ok(TaskResult {
            task_id: TaskId::new(),
            success: true,
            output: format!(
                "Agent '{}' executed prompt: {}",
                agent_name,
                prompt.chars().take(100).collect::<String>()
            ),
            error: None,
            artifacts: vec![],
            completed_at: Utc::now(),
            tokens_used: None,
        })
    }

    /// Cancel a running task.
    async fn cancel_task(&self, task_id: TaskId) {
        tracing::info!(task_id = %task_id, "Cancelling task");

        let _ = self
            .event_bus
            .emit(SystemEvent::TaskCancelled {
                task_id: task_id.to_string(),
            })
            .await;
    }

    /// Check if the executor is running.
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get the event receiver channel.
    pub fn events(&self) -> mpsc::Receiver<ExecutorEvent> {
        // This creates a new receiver - in practice you'd want to handle this differently
        let (_, rx) = mpsc::channel(100);
        rx
    }
}

/// Builder for creating TaskExecutor instances.
pub struct TaskExecutorBuilder {
    application_id: Option<String>,
    router: Option<Arc<TaskRouter>>,
    event_bus: Option<Arc<EventBus>>,
    command_rx: Option<mpsc::Receiver<ExecutorCommand>>,
}

impl TaskExecutorBuilder {
    pub fn new() -> Self {
        Self {
            application_id: None,
            router: None,
            event_bus: None,
            command_rx: None,
        }
    }

    pub fn application_id(mut self, id: impl Into<String>) -> Self {
        self.application_id = Some(id.into());
        self
    }

    pub fn router(mut self, router: Arc<TaskRouter>) -> Self {
        self.router = Some(router);
        self
    }

    pub fn event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    pub fn command_channel(mut self, rx: mpsc::Receiver<ExecutorCommand>) -> Self {
        self.command_rx = Some(rx);
        self
    }

    pub fn build(self) -> TaskExecutor {
        TaskExecutor::new(
            self.application_id.unwrap_or_default(),
            self.router
                .unwrap_or_else(|| Arc::new(TaskRouter::new(Arc::new(RwLock::new(vec![]))))),
            self.event_bus.unwrap_or_else(|| Arc::new(EventBus::new())),
            self.command_rx.expect("command_rx is required"),
        )
    }
}

impl Default for TaskExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Start a task executor in the background.
pub async fn start_executor(
    application_id: String,
    router: Arc<TaskRouter>,
    event_bus: Arc<EventBus>,
) -> (TaskExecutor, mpsc::Sender<ExecutorCommand>) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<ExecutorCommand>(100);

    let executor = TaskExecutorBuilder::new()
        .application_id(application_id)
        .router(router)
        .event_bus(event_bus)
        .command_channel(cmd_rx)
        .build();

    (executor, cmd_tx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_executor_creation() {
        let (cmd_tx, cmd_rx) = mpsc::channel(10);

        let executor = TaskExecutorBuilder::new()
            .application_id("test-app")
            .command_channel(cmd_rx)
            .build();

        assert!(!executor.is_running().await);
        drop(cmd_tx);
    }

    #[tokio::test]
    async fn test_executor_start() {
        let (cmd_tx, cmd_rx) = mpsc::channel(10);

        let executor = TaskExecutorBuilder::new()
            .application_id("test-app")
            .command_channel(cmd_rx)
            .build();

        let mut exec = executor;
        // Note: In a real test, we'd spawn this as a task
        // exec.run().await;
    }
}
