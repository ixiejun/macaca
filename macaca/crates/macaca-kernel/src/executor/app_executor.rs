//! Application Executor - Complete execution environment for a single application.
//!
//! This module provides an isolated execution environment for one application,
//! containing all the components needed for agent-to-agent task delegation.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{info, error, warn};

use super::{
    AgentInfo, AgentRunner, ApplicationId, DelegatedTask, EventBus, ExecutionQueue,
    RoutingDecision, TaskContext, TaskId, TaskResult, TaskRouter, TaskStatus,
    CallbackDispatcher, ExecutorCommand, ExecutorEvent, SystemEvent, ForkManager,
};
use macaca_proto::TaskId as ProtoTaskId;

/// Configuration for an ApplicationExecutor.
#[derive(Debug, Clone)]
pub struct ApplicationExecutorConfig {
    /// Maximum number of parallel task executions.
    pub max_parallel: usize,
    /// Maximum queue size for pending tasks.
    pub max_queue_size: usize,
    /// Enable event bus for system-wide events.
    pub enable_events: bool,
}

impl Default for ApplicationExecutorConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            max_queue_size: 100,
            enable_events: true,
        }
    }
}

/// Complete execution environment for a single application.
///
/// This struct owns all the components needed for task delegation:
/// - Task queue for pending work
/// - Event bus for system events
/// - Router for matching tasks to agents
/// - Callback dispatcher for pushing results
/// - Reference to the agent runner
///
/// # Application Isolation
///
/// Each ApplicationExecutor is isolated from others. Tasks delegated within
/// application A will only be executed by agents in application A.
pub struct ApplicationExecutor {
    /// Application ID for isolation.
    pub application_id: ApplicationId,
    /// Application name (human-readable).
    pub application_name: String,

    /// Agents registered in this application.
    agents: Arc<RwLock<Vec<AgentInfo>>>,

    /// Task queue for this application (shared with worker).
    queue: Arc<ExecutionQueue>,

    /// Event bus for publishing system events.
    event_bus: EventBus,

    /// Router for matching tasks to agents.
    router: TaskRouter,

    /// Callback dispatcher for pushing results to coordinators.
    callback_dispatcher: CallbackDispatcher,

    /// The agent runner that actually executes agents.
    runner: Arc<dyn AgentRunner>,

    /// Channel for sending commands to the worker.
    command_tx: mpsc::Sender<ExecutorCommand>,

    /// Channel for receiving events from the worker.
    event_rx: Option<mpsc::Receiver<ExecutorEvent>>,

    /// Broadcast sender for executor events (for external subscribers like SSE).
    event_broadcast: tokio::sync::broadcast::Sender<ExecutorEvent>,

    /// Fork Manager for Fork-Join workflow.
    fork_manager: Arc<ForkManager>,

    /// Shutdown signal.
    shutdown: Arc<RwLock<bool>>,
}

impl ApplicationExecutor {
    /// Create a new ApplicationExecutor with the given configuration.
    pub fn new(
        application_id: ApplicationId,
        application_name: String,
        agents: Vec<AgentInfo>,
        runner: Arc<dyn AgentRunner>,
        config: ApplicationExecutorConfig,
    ) -> Self {
        let agents = Arc::new(RwLock::new(agents));
        let queue = Arc::new(ExecutionQueue::new(config.max_parallel, config.max_queue_size));
        let event_bus = EventBus::new();
        let router = TaskRouter::new(Arc::clone(&agents));
        let callback_dispatcher = CallbackDispatcher::new();

        // Create channels for worker communication
        let (command_tx, command_rx) = mpsc::channel(100);
        let (event_tx, event_rx) = mpsc::channel(100);

        // Create broadcast channel for external subscribers
        let (event_broadcast, _) = tokio::sync::broadcast::channel(256);

        // Create Fork Manager for Fork-Join workflow
        let fork_manager = Arc::new(ForkManager::new());

        let shutdown = Arc::new(RwLock::new(false));

        // Spawn the worker task
        let worker_runner = Arc::clone(&runner);
        let worker_shutdown = Arc::clone(&shutdown);
        let worker_queue = Arc::clone(&queue);
        let worker_app_id = application_id.clone();
        let worker_event_broadcast = event_broadcast.clone();
        let worker_fork_manager = Arc::clone(&fork_manager);

        tokio::spawn(async move {
            Self::worker_loop(
                worker_runner,
                command_rx,
                event_tx,
                worker_event_broadcast,
                worker_shutdown,
                worker_queue,
                worker_app_id,
                worker_fork_manager,
            ).await;
        });

        Self {
            application_id,
            application_name,
            agents,
            queue,
            event_bus,
            router,
            callback_dispatcher,
            runner,
            command_tx,
            event_rx: Some(event_rx),
            event_broadcast,
            fork_manager,
            shutdown,
        }
    }

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
            return Err(format!("Agent '{}' not found in application '{}'", to_agent, self.application_id));
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
        self.queue.enqueue(task.clone()).await
            .map_err(|e| format!("Failed to enqueue task: {}", e))?;

        // Publish task delegated event
        self.event_bus.emit(SystemEvent::TaskDelegated {
            task_id: task.id,
            application_id: self.application_id.clone(),
            from_agent: from_agent.to_string(),
            to_agent: to_agent.to_string(),
            prompt: task.prompt.clone(),
        }).await;

        // Send execute command to worker
        info!(
            "Sending execute command to worker: app={}, task_id={}, from={}, to={}",
            self.application_id, task_id, from_agent, to_agent
        );
        self.command_tx.send(ExecutorCommand::Execute(task)).await
            .map_err(|e| format!("Failed to send execute command: {}", e))?;

        info!(
            "Execute command sent successfully: app={}, task_id={}",
            self.application_id, task_id
        );

        Ok(task_id)
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
            return Err(format!("No suitable agent found for prompt: {}", decision.reasoning));
        }

        // Delegate to the selected agent
        let task_id = self.delegate_task(
            from_agent,
            &decision.agent_name,
            prompt,
            priority,
            parallel,
            context,
        ).await?;

        Ok((task_id, decision))
    }

    /// Get the result of a delegated task.
    pub async fn get_task_result(&self, task_id: TaskId) -> Option<TaskResult> {
        self.queue.get_result(&task_id).await
    }

    /// Get the status of a task.
    pub async fn get_task_status(&self, task_id: &TaskId) -> Option<TaskStatus> {
        self.queue.get_status(task_id).await
    }

    /// List all agents in this application.
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        self.agents.read().await.clone()
    }

    /// Update agent availability.
    pub async fn update_agent_availability(&self, agent_name: &str, available: bool) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.iter_mut().find(|a| a.name == agent_name) {
            agent.available = available;
        }
    }

    /// Subscribe to system events.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<super::bus::Event> {
        self.event_bus.subscribe_to_broadcast()
    }

    /// Subscribe to executor events (for external subscribers like SSE).
    ///
    /// This returns a broadcast receiver that will receive all executor events
    /// including task lifecycle events and agent execution events.
    pub fn subscribe_to_events(&self) -> tokio::sync::broadcast::Receiver<ExecutorEvent> {
        self.event_broadcast.subscribe()
    }

    /// Get the Fork Manager for Fork-Join workflow.
    pub fn fork_manager(&self) -> Arc<ForkManager> {
        Arc::clone(&self.fork_manager)
    }

    /// Shutdown the executor gracefully.
    pub async fn shutdown(&self) {
        *self.shutdown.write().await = true;
        let _ = self.command_tx.send(ExecutorCommand::Shutdown).await;
        info!(application_id = %self.application_id, "Executor shutdown initiated");
    }

    /// Worker loop that processes tasks.
    async fn worker_loop(
        runner: Arc<dyn AgentRunner>,
        mut command_rx: mpsc::Receiver<ExecutorCommand>,
        event_tx: mpsc::Sender<ExecutorEvent>,
        event_broadcast: tokio::sync::broadcast::Sender<ExecutorEvent>,
        shutdown: Arc<RwLock<bool>>,
        queue: Arc<ExecutionQueue>,
        application_id: ApplicationId,
        fork_manager: Arc<ForkManager>,
    ) {
        info!("Application executor worker started");

        // Spawn a background task to forward hook events to executor events
        let mut hook_rx = fork_manager.subscribe_to_hooks();
        let hook_event_broadcast = event_broadcast.clone();
        let hook_shutdown = Arc::clone(&shutdown);
        tokio::spawn(async move {
            loop {
                if *hook_shutdown.read().await {
                    break;
                }
                // Non-blocking check for hook events
                match hook_rx.try_recv() {
                    Ok(hook_event) => {
                        info!("Forwarding hook event to SSE: {:?}", hook_event);
                        // Convert HookEvent to ExecutorEvent for SSE streaming
                        let executor_event = ExecutorEvent::HookEvent {
                            event: hook_event,
                        };
                        let _ = hook_event_broadcast.send(executor_event);
                    }
                    Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                        // No events available, sleep briefly
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    }
                    Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                        info!("Hook event channel closed");
                        break;
                    }
                    Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                        // Continue on lag
                        continue;
                    }
                }
            }
            info!("Hook event forwarder stopped");
        });

        loop {
            // Check for shutdown
            if *shutdown.read().await {
                info!("Worker shutting down");
                break;
            }

            // Wait for commands
            info!("Worker waiting for command...");
            let command = match command_rx.recv().await {
                Some(cmd) => {
                    info!("Worker received command: {:?}", std::mem::discriminant(&cmd));
                    cmd
                },
                None => {
                    warn!("Command channel closed, worker exiting");
                    break;
                }
            };

            match command {
                ExecutorCommand::Execute(task) => {
                    let task_id = task.id;
                    let agent_name = task.to_agent.clone();
                    let prompt = task.prompt.clone();
                    let context = task.context.clone();

                    // Notify task started
                    let start_event = ExecutorEvent::TaskStarted {
                        task_id,
                        agent: agent_name.clone(),
                    };
                    let _ = event_tx.send(start_event.clone()).await;
                    let _ = event_broadcast.send(start_event);

                    info!(
                        task_id = %task_id,
                        agent = %agent_name,
                        prompt_preview = %prompt.chars().take(100).collect::<String>(),
                        "Worker: executing task"
                    );

                    // Create a channel for agent execution events
                    let (agent_event_tx, mut agent_event_rx) =
                        mpsc::channel::<macaca_proto::AgentExecutionEvent>(64);

                    // Clone event_tx and event_broadcast for the forwarding task
                    let event_tx_clone = event_tx.clone();
                    let event_broadcast_clone = event_broadcast.clone();
                    let agent_name_clone = agent_name.clone();

                    // Spawn a task to forward agent events to executor events
                    let forward_handle = tokio::spawn(async move {
                        while let Some(event) = agent_event_rx.recv().await {
                            let executor_event = ExecutorEvent::AgentEvent {
                                task_id,
                                agent: agent_name_clone.clone(),
                                event,
                            };
                            // Send to internal channel
                            let _ = event_tx_clone.send(executor_event.clone()).await;
                            // Broadcast to external subscribers (SSE, etc.)
                            let _ = event_broadcast_clone.send(executor_event);
                        }
                    });

                    // Execute the agent with events
                    let result = runner.execute_agent_with_events(
                        &application_id,
                        &agent_name,
                        &prompt,
                        context,
                        Some(agent_event_tx),
                    ).await;

                    // Wait for event forwarding to complete
                    let _ = forward_handle.await;

                    match result {
                        Ok(mut task_result) => {
                            info!(
                                task_id = %task_id,
                                agent = %agent_name,
                                success = task_result.success,
                                output_len = task_result.output.len(),
                                error = ?task_result.error,
                                "Worker: task executed, storing result"
                            );

                            // Use the original task_id, not the one from the runner
                            task_result.task_id = task_id;

                            // Store result in queue
                            queue.store_result(task_result.clone()).await;

                            // Resume any fork waiting on this task
                            let delegate_result = super::fork_manager::DelegateResult {
                                task_id: macaca_proto::TaskId(task_id.0),
                                success: task_result.success,
                                output: task_result.output.clone(),
                                error: task_result.error.clone(),
                                artifacts: task_result.artifacts.clone(),
                            };
                            if let Err(e) = fork_manager.resume_fork_by_task(macaca_proto::TaskId(task_id.0), delegate_result).await {
                                warn!(task_id = %task_id, error = %e, "Failed to resume fork waiting on task");
                            }

                            // Notify task completed
                            let completed_event = ExecutorEvent::TaskCompleted {
                                task_id,
                                result: task_result,
                            };
                            let _ = event_tx.send(completed_event.clone()).await;
                            let _ = event_broadcast.send(completed_event);
                        }
                        Err(e) => {
                            error!(
                                task_id = %task_id,
                                agent = %agent_name,
                                error = %e,
                                "Worker: task execution failed"
                            );

                            // Store error result with original task_id
                            let error_result = TaskResult {
                                task_id,
                                success: false,
                                output: String::new(),
                                error: Some(e.clone()),
                                artifacts: vec![],
                                completed_at: chrono::Utc::now(),
                                tokens_used: None,
                            };
                            queue.store_result(error_result).await;

                            // Notify task failed
                            let failed_event = ExecutorEvent::TaskFailed {
                                task_id,
                                error: e,
                            };
                            let _ = event_tx.send(failed_event.clone()).await;
                            let _ = event_broadcast.send(failed_event);
                        }
                    }
                }
                ExecutorCommand::Cancel(task_id) => {
                    if queue.cancel(task_id).await {
                        let cancel_event = ExecutorEvent::TaskCancelled { task_id };
                        let _ = event_tx.send(cancel_event.clone()).await;
                        let _ = event_broadcast.send(cancel_event);
                    }
                }
                ExecutorCommand::Shutdown => {
                    info!("Shutdown command received");
                    break;
                }
            }
        }

        info!("Worker loop exited");
    }
}

/// Registry of all application executors.
///
/// This is the top-level container that manages all isolated application
/// executors. Each application gets its own executor instance.
pub struct ApplicationExecutorRegistry {
    executors: RwLock<HashMap<ApplicationId, Arc<ApplicationExecutor>>>,
    default_runner: Arc<dyn AgentRunner>,
}

impl ApplicationExecutorRegistry {
    /// Create a new registry with a default agent runner.
    pub fn new(default_runner: Arc<dyn AgentRunner>) -> Self {
        Self {
            executors: RwLock::new(HashMap::new()),
            default_runner,
        }
    }

    /// Register a new application with its agents.
    pub async fn register_application(
        &self,
        application_id: ApplicationId,
        application_name: String,
        agents: Vec<AgentInfo>,
    ) -> Arc<ApplicationExecutor> {
        self.register_application_with_config(
            application_id,
            application_name,
            agents,
            ApplicationExecutorConfig::default(),
        ).await
    }

    /// Register a new application with custom configuration.
    pub async fn register_application_with_config(
        &self,
        application_id: ApplicationId,
        application_name: String,
        agents: Vec<AgentInfo>,
        config: ApplicationExecutorConfig,
    ) -> Arc<ApplicationExecutor> {
        let executor = Arc::new(ApplicationExecutor::new(
            application_id.clone(),
            application_name,
            agents,
            Arc::clone(&self.default_runner),
            config,
        ));

        self.executors.write().await.insert(application_id, Arc::clone(&executor));
        executor
    }

    /// Get an executor by application ID.
    pub async fn get(&self, application_id: &ApplicationId) -> Option<Arc<ApplicationExecutor>> {
        self.executors.read().await.get(application_id).cloned()
    }

    /// Unregister an application.
    pub async fn unregister(&self, application_id: &ApplicationId) -> bool {
        if let Some(executor) = self.executors.write().await.remove(application_id) {
            executor.shutdown().await;
            true
        } else {
            false
        }
    }

    /// List all registered applications.
    pub async fn list_applications(&self) -> Vec<(ApplicationId, String)> {
        self.executors.read().await
            .iter()
            .map(|(id, exec)| (id.clone(), exec.application_name.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_executor_config_defaults() {
        let config = ApplicationExecutorConfig::default();
        assert_eq!(config.max_parallel, 4);
        assert_eq!(config.max_queue_size, 100);
        assert!(config.enable_events);
    }
}
