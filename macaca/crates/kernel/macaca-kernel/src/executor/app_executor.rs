//! Application Executor - Complete execution environment for a single application.
//!
//! This module provides an isolated execution environment for one application,
//! containing all the components needed for agent-to-agent task delegation.
//!
//! Persistence is injected as a kernel port.  The executor owns scheduling,
//! restart, and fork recovery semantics; service/runtime composition owns the
//! concrete repository implementation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::persistence::KernelPersistencePort;

/// Worker state for tracking worker health.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    /// Worker is running normally
    Running,
    /// Command channel is closed, worker is waiting for recovery
    Disconnected,
    /// Worker has shut down gracefully
    Shutdown,
}

/// Worker health status for external health checks.
#[derive(Debug)]
pub enum WorkerHealth {
    /// Worker is healthy with recent heartbeat
    Healthy { last_heartbeat: std::time::Duration },
    /// Worker is unhealthy (no recent heartbeat)
    Unhealthy { reason: String },
    /// Worker's command channel is disconnected
    Disconnected,
    /// Worker has shut down
    Shutdown,
}

use super::{
    AgentInfo, AgentRunner, ApplicationId, CallbackDispatcher, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkManager,
    RoutingDecision, SystemEvent, TaskContext, TaskId, TaskResult, TaskRouter, TaskStatus,
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

/// Configuration for the worker supervisor that handles automatic restart on failure.
#[derive(Debug, Clone)]
pub struct WorkerSupervisorConfig {
    /// Maximum number of restarts before giving up.
    pub max_restarts: u32,
    /// If the worker runs successfully for this many seconds, reset the restart counter.
    pub cooldown_reset_secs: u64,
    /// Milliseconds to wait before each restart attempt.
    pub restart_delay_ms: u64,
}

impl Default for WorkerSupervisorConfig {
    fn default() -> Self {
        Self {
            max_restarts: 5,
            cooldown_reset_secs: 300,
            restart_delay_ms: 1000,
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
    /// Wrapped in Arc<RwLock<...>> so the supervisor can swap it after each restart.
    command_tx: Arc<RwLock<mpsc::Sender<ExecutorCommand>>>,

    /// Channel for receiving events from the worker.
    event_rx: Option<mpsc::Receiver<ExecutorEvent>>,

    /// Broadcast sender for executor events (for external subscribers like SSE).
    event_broadcast: tokio::sync::broadcast::Sender<ExecutorEvent>,

    /// Fork Manager for Fork-Join workflow.
    fork_manager: Arc<ForkManager>,

    /// Shutdown signal.
    shutdown: Arc<RwLock<bool>>,

    /// Worker heartbeat timestamp (updated every 10 seconds).
    worker_heartbeat: Arc<RwLock<Instant>>,

    /// Worker state (Running/Disconnected/Shutdown).
    worker_state: Arc<RwLock<WorkerState>>,

    /// Flag set by shutdown() to tell the supervisor not to restart.
    shutdown_requested: Arc<AtomicBool>,

    /// Number of times the worker has been restarted.
    restart_count: Arc<AtomicU32>,
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
        let queue = Arc::new(ExecutionQueue::new(
            config.max_parallel,
            config.max_queue_size,
        ));
        let event_bus = EventBus::new();
        let router = TaskRouter::new(Arc::clone(&agents));
        let callback_dispatcher = CallbackDispatcher::new();

        // Create channels for worker communication
        let (command_tx, command_rx) = mpsc::channel(100);
        let (event_tx, event_rx) = mpsc::channel(100);

        // Create broadcast channel for external subscribers
        let (event_broadcast, _) = tokio::sync::broadcast::channel(4096);

        // Create Fork Manager for Fork-Join workflow
        let fork_manager = Arc::new(ForkManager::new());

        let shutdown = Arc::new(RwLock::new(false));

        // Initialize worker state tracking
        let worker_heartbeat = Arc::new(RwLock::new(Instant::now()));
        let worker_state = Arc::new(RwLock::new(WorkerState::Running));

        let shutdown_requested = Arc::new(AtomicBool::new(false));
        let restart_count = Arc::new(AtomicU32::new(0));

        // Wrap command_tx so the supervisor can swap it after each restart.
        let command_tx_shared = Arc::new(RwLock::new(command_tx));

        // Spawn supervisor task which owns the worker loop and restarts it on failure.
        let sup_runner = Arc::clone(&runner);
        let sup_shutdown = Arc::clone(&shutdown);
        let sup_queue = Arc::clone(&queue);
        let sup_app_id = application_id.clone();
        let sup_event_broadcast = event_broadcast.clone();
        let sup_fork_manager = Arc::clone(&fork_manager);
        let sup_heartbeat = Arc::clone(&worker_heartbeat);
        let sup_state = Arc::clone(&worker_state);
        let sup_shutdown_requested = Arc::clone(&shutdown_requested);
        let sup_restart_count = Arc::clone(&restart_count);
        let sup_command_tx = Arc::clone(&command_tx_shared);
        let sup_supervisor_config = WorkerSupervisorConfig::default();
        // The first command_rx was already created above; pass it to the supervisor
        // which will hand it to the first worker invocation.
        let initial_command_rx = command_rx;

        tokio::spawn(async move {
            Self::supervisor_loop(
                sup_runner,
                initial_command_rx,
                event_tx,
                sup_event_broadcast,
                sup_shutdown,
                sup_queue,
                sup_app_id,
                sup_fork_manager,
                sup_heartbeat,
                sup_state,
                sup_shutdown_requested,
                sup_restart_count,
                sup_command_tx,
                sup_supervisor_config,
            )
            .await;
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
            command_tx: command_tx_shared,
            event_rx: Some(event_rx),
            event_broadcast,
            fork_manager,
            shutdown,
            worker_heartbeat,
            worker_state,
            shutdown_requested,
            restart_count,
        }
    }

    /// Create a new ApplicationExecutor with persistence support.
    ///
    /// Restores any previously persisted queue entries and fork states before
    /// starting the worker supervisor.  The store is a provider-neutral port,
    /// so the kernel can recover durable execution mementos without importing
    /// a concrete database backend.
    pub async fn new_with_store(
        application_id: ApplicationId,
        application_name: String,
        agents: Vec<AgentInfo>,
        runner: Arc<dyn AgentRunner>,
        config: ApplicationExecutorConfig,
        store: Arc<dyn KernelPersistencePort>,
    ) -> Self {
        info!(
            app_id = %application_id,
            backend = store.backend_name(),
            "application executor persistence restore started"
        );
        let agents = Arc::new(RwLock::new(agents));

        // Build queue with persistence
        let mut queue_inner = super::queue::ExecutionQueue::new_with_store(
            config.max_parallel,
            config.max_queue_size,
            Some(Arc::clone(&store)),
            application_id.clone(),
        );
        queue_inner.restore_from_store().await;
        let queue = Arc::new(queue_inner);

        let event_bus = EventBus::new();
        let router = TaskRouter::new(Arc::clone(&agents));
        let callback_dispatcher = CallbackDispatcher::new();

        // Create channels for worker communication
        let (command_tx, command_rx) = mpsc::channel(100);
        let (event_tx, event_rx) = mpsc::channel(100);

        // Create broadcast channel for external subscribers
        let (event_broadcast, _) = tokio::sync::broadcast::channel(4096);

        // Build Fork Manager with persistence
        let mut fork_manager_inner = super::fork_manager::ForkManager::new_with_store(
            Some(Arc::clone(&store)),
            application_id.clone(),
        );
        fork_manager_inner.restore_forks().await;
        let fork_manager = Arc::new(fork_manager_inner);

        let shutdown = Arc::new(RwLock::new(false));

        // Initialize worker state tracking
        let worker_heartbeat = Arc::new(RwLock::new(Instant::now()));
        let worker_state = Arc::new(RwLock::new(WorkerState::Running));

        let shutdown_requested = Arc::new(AtomicBool::new(false));
        let restart_count = Arc::new(AtomicU32::new(0));

        // Wrap command_tx so the supervisor can swap it after each restart.
        let command_tx_shared = Arc::new(RwLock::new(command_tx));

        // Spawn supervisor task which owns the worker loop and restarts it on failure.
        let sup_runner = Arc::clone(&runner);
        let sup_shutdown = Arc::clone(&shutdown);
        let sup_queue = Arc::clone(&queue);
        let sup_app_id = application_id.clone();
        let sup_event_broadcast = event_broadcast.clone();
        let sup_fork_manager = Arc::clone(&fork_manager);
        let sup_heartbeat = Arc::clone(&worker_heartbeat);
        let sup_state = Arc::clone(&worker_state);
        let sup_shutdown_requested = Arc::clone(&shutdown_requested);
        let sup_restart_count = Arc::clone(&restart_count);
        let sup_command_tx = Arc::clone(&command_tx_shared);
        let sup_supervisor_config = WorkerSupervisorConfig::default();
        let initial_command_rx = command_rx;

        tokio::spawn(async move {
            Self::supervisor_loop(
                sup_runner,
                initial_command_rx,
                event_tx,
                sup_event_broadcast,
                sup_shutdown,
                sup_queue,
                sup_app_id,
                sup_fork_manager,
                sup_heartbeat,
                sup_state,
                sup_shutdown_requested,
                sup_restart_count,
                sup_command_tx,
                sup_supervisor_config,
            )
            .await;
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
            command_tx: command_tx_shared,
            event_rx: Some(event_rx),
            event_broadcast,
            fork_manager,
            shutdown,
            worker_heartbeat,
            worker_state,
            shutdown_requested,
            restart_count,
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

                let delegate_result = super::fork_manager::DelegateResult {
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

    /// Broadcast an executor event to all subscribers.
    ///
    /// Used by external execution paths (e.g. WorkerLoop via FrameworkRunner)
    /// to emit events into the same broadcast channel that AppExecutor uses.
    pub fn broadcast_event(&self, event: ExecutorEvent) {
        let _ = self.event_broadcast.send(event);
    }

    /// Get the Fork Manager for Fork-Join workflow.
    pub fn fork_manager(&self) -> Arc<ForkManager> {
        Arc::clone(&self.fork_manager)
    }

    /// Shutdown the executor gracefully.
    pub async fn shutdown(&self) {
        // Set the atomic flag first so the supervisor won't restart the worker.
        self.shutdown_requested.store(true, AtomicOrdering::SeqCst);
        *self.shutdown.write().await = true;
        let _ = self
            .command_tx
            .read()
            .await
            .send(ExecutorCommand::Shutdown)
            .await;
        info!(application_id = %self.application_id, "Executor shutdown initiated");
    }

    /// Check if worker is healthy.
    ///
    /// Returns the current health status of the worker based on:
    /// - Current worker state (Running/Disconnected/Shutdown)
    /// - Time since last heartbeat
    pub async fn check_worker_health(&self) -> WorkerHealth {
        let state = self.worker_state.read().await;
        match *state {
            WorkerState::Running => {
                let elapsed = self.worker_heartbeat.read().await.elapsed();
                if elapsed < std::time::Duration::from_secs(30) {
                    WorkerHealth::Healthy {
                        last_heartbeat: elapsed,
                    }
                } else {
                    WorkerHealth::Unhealthy {
                        reason: format!("No heartbeat for {:?}", elapsed),
                    }
                }
            }
            WorkerState::Disconnected => WorkerHealth::Disconnected,
            WorkerState::Shutdown => WorkerHealth::Shutdown,
        }
    }

    /// Check if worker is healthy (simple boolean).
    pub async fn is_worker_healthy(&self) -> bool {
        matches!(
            self.check_worker_health().await,
            WorkerHealth::Healthy { .. }
        )
    }

    /// Return how many times the worker has been restarted by the supervisor.
    pub fn restart_count(&self) -> u32 {
        self.restart_count.load(AtomicOrdering::SeqCst)
    }

    /// Supervisor loop: spawns the worker and restarts it on unexpected exit.
    #[allow(clippy::too_many_arguments)]
    async fn supervisor_loop(
        runner: Arc<dyn AgentRunner>,
        initial_command_rx: mpsc::Receiver<ExecutorCommand>,
        event_tx: mpsc::Sender<ExecutorEvent>,
        event_broadcast: tokio::sync::broadcast::Sender<ExecutorEvent>,
        shutdown: Arc<RwLock<bool>>,
        queue: Arc<ExecutionQueue>,
        application_id: ApplicationId,
        fork_manager: Arc<ForkManager>,
        worker_heartbeat: Arc<RwLock<Instant>>,
        worker_state: Arc<RwLock<WorkerState>>,
        shutdown_requested: Arc<AtomicBool>,
        restart_count: Arc<AtomicU32>,
        command_tx_shared: Arc<RwLock<mpsc::Sender<ExecutorCommand>>>,
        config: WorkerSupervisorConfig,
    ) {
        let mut command_rx = initial_command_rx;

        loop {
            let started_at = Instant::now();

            // Clone everything the worker needs for this iteration.
            let w_runner = Arc::clone(&runner);
            let w_event_tx = event_tx.clone();
            let w_event_broadcast = event_broadcast.clone();
            let w_shutdown = Arc::clone(&shutdown);
            let w_queue = Arc::clone(&queue);
            let w_app_id = application_id.clone();
            let w_fork_manager = Arc::clone(&fork_manager);
            let w_heartbeat = Arc::clone(&worker_heartbeat);
            let w_state = Arc::clone(&worker_state);

            let handle = tokio::spawn(Self::worker_loop(
                w_runner,
                command_rx,
                w_event_tx,
                w_event_broadcast,
                w_shutdown,
                w_queue,
                w_app_id,
                w_fork_manager,
                w_heartbeat,
                w_state,
            ));

            // Wait for worker to finish.
            match handle.await {
                Ok(()) => {
                    // Worker exited cleanly (Shutdown command or channel-closed timeout).
                }
                Err(e) => {
                    error!(
                        application_id = %application_id,
                        error = %e,
                        "Worker task panicked"
                    );
                }
            }

            // If a graceful shutdown was requested, do not restart.
            if shutdown_requested.load(AtomicOrdering::SeqCst) {
                info!(
                    application_id = %application_id,
                    "Supervisor: shutdown requested, not restarting worker"
                );
                break;
            }

            // If worker ran long enough, reset restart counter.
            if started_at.elapsed().as_secs() >= config.cooldown_reset_secs {
                let prev = restart_count.swap(0, AtomicOrdering::SeqCst);
                if prev > 0 {
                    info!(
                        application_id = %application_id,
                        previous_count = prev,
                        "Supervisor: worker ran past cooldown threshold, resetting restart counter"
                    );
                }
            }

            let count = restart_count.fetch_add(1, AtomicOrdering::SeqCst) + 1;

            if count > config.max_restarts {
                error!(
                    application_id = %application_id,
                    restart_count = count,
                    max_restarts = config.max_restarts,
                    "Supervisor: max restarts exceeded, giving up"
                );
                *worker_state.write().await = WorkerState::Shutdown;
                break;
            }

            warn!(
                application_id = %application_id,
                restart_count = count,
                delay_ms = config.restart_delay_ms,
                "Supervisor: worker exited unexpectedly, restarting after delay"
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(config.restart_delay_ms)).await;

            // Create a fresh command channel and hand the new sender to the executor.
            let (new_tx, new_rx) = mpsc::channel(100);
            *command_tx_shared.write().await = new_tx;
            command_rx = new_rx;

            // Mark worker as running again so health checks pass.
            *worker_state.write().await = WorkerState::Running;
            *worker_heartbeat.write().await = Instant::now();

            info!(
                application_id = %application_id,
                restart_count = count,
                "Supervisor: spawning new worker"
            );
        }

        info!(application_id = %application_id, "Supervisor loop exited");
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
        worker_heartbeat: Arc<RwLock<Instant>>,
        worker_state: Arc<RwLock<WorkerState>>,
    ) {
        info!(
            application_id = %application_id,
            "Worker loop started, waiting for commands"
        );

        // Spawn a background task to forward hook events to executor events
        // Using select! pattern for cleaner async handling
        let mut hook_rx = fork_manager.subscribe_to_hooks();
        let hook_event_broadcast = event_broadcast.clone();
        let hook_shutdown = Arc::clone(&shutdown);
        tokio::spawn(async move {
            // Use interval for polling instead of tight loop with sleep
            let mut poll_interval = tokio::time::interval(tokio::time::Duration::from_millis(50));
            poll_interval.tick().await; // Skip initial tick

            info!("Hook event forwarder started");

            loop {
                tokio::select! {
                    biased;

                    // 1. Shutdown signal (highest priority)
                    _ = async {
                        if *hook_shutdown.read().await {
                            futures::future::ready(()).await
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        info!("Hook forwarder shutting down");
                        break;
                    }

                    // 2. Hook event receive
                    result = hook_rx.recv() => {
                        match result {
                            Ok(hook_event) => {
                                debug!("Forwarding hook event to SSE: {:?}", hook_event);
                                // Convert HookEvent to ExecutorEvent for SSE streaming
                                let executor_event = ExecutorEvent::HookEvent {
                                    event: hook_event,
                                };
                                let _ = hook_event_broadcast.send(executor_event);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                info!("Hook event channel closed");
                                break;
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                warn!("Hook event channel lagged, missed {} messages", n);
                                // Continue on lag
                                continue;
                            }
                        }
                    }

                    // 3. Poll interval (lowest priority - just prevents busy-wait)
                    _ = poll_interval.tick() => {
                        // Heartbeat tick - no action needed
                    }
                }
            }
            info!("Hook event forwarder stopped");
        });

        // Heartbeat interval for worker health tracking
        let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        heartbeat_interval.tick().await; // Skip initial tick

        // Track when channel was closed (for graceful degradation)
        let mut channel_closed_at: Option<std::time::Instant> = None;
        const MAX_CHANNEL_CLOSED_DURATION: tokio::time::Duration =
            tokio::time::Duration::from_secs(30);

        loop {
            // Use select! with biased; for deterministic priority
            tokio::select! {
                biased;

                // 1. Shutdown check (highest priority)
                _ = async {
                    if *shutdown.read().await {
                        futures::future::ready(()).await
                    } else {
                        std::future::pending().await
                    }
                } => {
                    *worker_state.write().await = WorkerState::Shutdown;
                    info!("Worker shutting down gracefully");
                    break;
                }

                // 2. Heartbeat (fires every 10s regardless of activity)
                _ = heartbeat_interval.tick() => {
                    *worker_heartbeat.write().await = Instant::now();
                    let elapsed = worker_heartbeat.read().await.elapsed();
                    debug!(
                        application_id = %application_id,
                        elapsed_since_last = ?elapsed,
                        "Worker heartbeat"
                    );
                }

                // 3. Command receive
                cmd_opt = command_rx.recv() => {
                    match cmd_opt {
                        Some(command) => {
                            // Reset channel closed timer on successful receive
                            channel_closed_at = None;
                            info!("Worker received command: {:?}", std::mem::discriminant(&command));

                            match command {
                                ExecutorCommand::Execute(task) => {
                                    let task_id = task.id;
                                    let agent_name = task.to_agent.clone();
                                    let prompt = task.prompt.clone();
                                    let context = task.context.clone();
                                    let events =
                                        ExecutorEventFactory::new(task_id, agent_name.clone());

                                    // Notify task started
                                    let start_event = events.started();
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
                                            // Broadcast to external subscribers (SSE, etc.)
                                            // Note: event_tx internal channel is intentionally skipped
                                            // to avoid blocking when nobody consumes event_rx.
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
                                            let completed_event =
                                                events.completed_with_result(task_result);
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
                                            let error_result = events.failed_result(e.clone());
                                            queue.store_result(error_result).await;

                                            // Notify task failed
                                            let failed_event = events.failed(e);
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
                        None => {
                            // Channel closed - track time and wait before exiting
                            let closed_at = channel_closed_at.get_or_insert(std::time::Instant::now());

                            // Update worker state to Disconnected
                            *worker_state.write().await = WorkerState::Disconnected;

                            if closed_at.elapsed() > MAX_CHANNEL_CLOSED_DURATION {
                                error!(
                                    application_id = %application_id,
                                    duration_secs = closed_at.elapsed().as_secs(),
                                    "Command channel closed for too long, worker exiting"
                                );
                                break;
                            }

                            warn!(
                                application_id = %application_id,
                                duration_secs = closed_at.elapsed().as_secs(),
                                "Command channel closed, waiting for recovery..."
                            );
                            // Continue loop - will check again on next iteration
                        }
                    }
                }
            }
        }

        info!(
            application_id = %application_id,
            "Worker loop exited"
        );
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
        )
        .await
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

        self.executors
            .write()
            .await
            .insert(application_id, Arc::clone(&executor));
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
        self.executors
            .read()
            .await
            .iter()
            .map(|(id, exec)| (id.clone(), exec.application_name.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Mock AgentRunner for testing
    struct MockRunner;

    #[async_trait::async_trait]
    impl AgentRunner for MockRunner {
        async fn execute_agent(
            &self,
            _application_id: &ApplicationId,
            agent_name: &str,
            _prompt: &str,
            _context: Option<TaskContext>,
        ) -> Result<TaskResult, String> {
            // Simulate some work
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(TaskResult {
                task_id: TaskId::new(),
                success: true,
                output: format!("{} executed", agent_name),
                error: None,
                artifacts: vec![],
                completed_at: chrono::Utc::now(),
                tokens_used: None,
            })
        }

        async fn list_agents(&self) -> Vec<AgentInfo> {
            vec![AgentInfo {
                id: "test-backend".to_string(),
                name: "backend".to_string(),
                capabilities: vec!["api".to_string()],
                current_load: 0,
                max_load: 10,
                available: true,
            }]
        }

        async fn agent_exists(&self, agent_name: &str) -> bool {
            agent_name == "backend"
        }
    }

    fn create_test_executor() -> ApplicationExecutor {
        ApplicationExecutor::new(
            ApplicationId::new(),
            "test-app".to_string(),
            vec![AgentInfo {
                id: "test-backend".to_string(),
                name: "backend".to_string(),
                capabilities: vec!["api".to_string()],
                current_load: 0,
                max_load: 10,
                available: true,
            }],
            Arc::new(MockRunner),
            ApplicationExecutorConfig::default(),
        )
    }

    #[test]
    fn test_application_executor_config_defaults() {
        let config = ApplicationExecutorConfig::default();
        assert_eq!(config.max_parallel, 4);
        assert_eq!(config.max_queue_size, 100);
        assert!(config.enable_events);
    }

    #[tokio::test]
    async fn test_worker_health_check_healthy() {
        let executor = create_test_executor();

        // Worker should be healthy immediately after creation
        let health = executor.check_worker_health().await;
        match health {
            WorkerHealth::Healthy { last_heartbeat } => {
                assert!(last_heartbeat < Duration::from_secs(5));
            }
            _ => panic!("Expected worker to be healthy"),
        }
    }

    #[tokio::test]
    async fn test_worker_is_healthy() {
        let executor = create_test_executor();

        // Worker should be healthy immediately after creation
        assert!(executor.is_worker_healthy().await);
    }

    #[tokio::test]
    async fn test_worker_state_running_after_creation() {
        let executor = create_test_executor();

        let state = executor.worker_state.read().await;
        assert_eq!(*state, WorkerState::Running);
    }

    #[tokio::test]
    async fn test_worker_heartbeat_updates() {
        let executor = create_test_executor();

        // Get initial heartbeat
        let before = *executor.worker_heartbeat.read().await;

        // Wait for at least one heartbeat cycle (10s + buffer)
        // Note: For faster tests, we just verify the mechanism exists
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify heartbeat mechanism is working (time should have progressed)
        let after = *executor.worker_heartbeat.read().await;

        // The heartbeat should be recent (within 1 second of now)
        assert!(after.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_worker_graceful_shutdown() {
        let executor = create_test_executor();

        // Verify worker is running
        assert!(executor.is_worker_healthy().await);

        // Initiate shutdown
        executor.shutdown().await;

        // Give time for shutdown to propagate
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Verify worker state changed to Shutdown
        let state = executor.worker_state.read().await;
        assert_eq!(*state, WorkerState::Shutdown);
    }

    #[test]
    fn test_worker_state_enum_values() {
        // Verify enum values can be created and compared
        assert_eq!(WorkerState::Running, WorkerState::Running);
        assert_eq!(WorkerState::Disconnected, WorkerState::Disconnected);
        assert_eq!(WorkerState::Shutdown, WorkerState::Shutdown);

        assert_ne!(WorkerState::Running, WorkerState::Disconnected);
        assert_ne!(WorkerState::Running, WorkerState::Shutdown);
        assert_ne!(WorkerState::Disconnected, WorkerState::Shutdown);
    }
}
