//! Application Executor - Complete execution environment for a single application.
//!
//! This module provides an isolated execution environment for one application,
//! containing all the components needed for agent-to-agent task delegation.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{info, error, warn};

use super::{
    AgentInfo, AgentRunner, DelegatedTask, EventBus, ExecutionQueue,
    RoutingDecision, TaskContext, TaskId, TaskResult, TaskRouter, TaskStatus,
    CallbackDispatcher, ExecutorCommand, ExecutorEvent, SystemEvent,
};

/// Unique identifier for an application.
pub type ApplicationId = String;

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

        let shutdown = Arc::new(RwLock::new(false));

        // Spawn the worker task
        let worker_runner = Arc::clone(&runner);
        let worker_shutdown = Arc::clone(&shutdown);
        let worker_queue = Arc::clone(&queue);

        tokio::spawn(async move {
            Self::worker_loop(
                worker_runner,
                command_rx,
                event_tx,
                worker_shutdown,
                worker_queue,
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
        self.command_tx.send(ExecutorCommand::Execute(task)).await
            .map_err(|e| format!("Failed to send execute command: {}", e))?;

        info!(
            application_id = %self.application_id,
            task_id = %task_id,
            from = %from_agent,
            to = %to_agent,
            "Task delegated"
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
        shutdown: Arc<RwLock<bool>>,
        queue: Arc<ExecutionQueue>,
    ) {
        info!("Application executor worker started");

        loop {
            // Check for shutdown
            if *shutdown.read().await {
                info!("Worker shutting down");
                break;
            }

            // Wait for commands
            let command = match command_rx.recv().await {
                Some(cmd) => cmd,
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
                    let _ = event_tx.send(ExecutorEvent::TaskStarted {
                        task_id,
                        agent: agent_name.clone(),
                    }).await;

                    info!(
                        task_id = %task_id,
                        agent = %agent_name,
                        "Executing task"
                    );

                    // Execute the agent
                    let result = runner.execute_agent(
                        &agent_name,
                        &prompt,
                        context,
                    ).await;

                    match result {
                        Ok(task_result) => {
                            // Store result in queue
                            queue.store_result(task_result.clone()).await;

                            // Notify task completed
                            let _ = event_tx.send(ExecutorEvent::TaskCompleted {
                                task_id,
                                result: task_result,
                            }).await;
                        }
                        Err(e) => {
                            error!(
                                task_id = %task_id,
                                error = %e,
                                "Task execution failed"
                            );

                            // Store error result
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
                            let _ = event_tx.send(ExecutorEvent::TaskFailed {
                                task_id,
                                error: e,
                            }).await;
                        }
                    }
                }
                ExecutorCommand::Cancel(task_id) => {
                    if queue.cancel(task_id).await {
                        let _ = event_tx.send(ExecutorEvent::TaskCancelled { task_id }).await;
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
    pub async fn get(&self, application_id: &str) -> Option<Arc<ApplicationExecutor>> {
        self.executors.read().await.get(application_id).cloned()
    }

    /// Unregister an application.
    pub async fn unregister(&self, application_id: &str) -> bool {
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
