//! ApplicationExecutor struct and factory constructors.
//!
//! Owns the per-application execution sandbox: queue, router, event bus, fork manager,
//! and the supervisor-spawned worker task. Persistence restore flows through the
//! injected `KernelPersistencePort` without binding to a concrete store backend.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, RwLock};
use tracing::info;

use macaca_kernel::KernelPersistencePort;

use super::types::{ApplicationExecutorConfig, WorkerState, WorkerSupervisorConfig};
use crate::executor::{
    AgentInfo, AgentRunner, ApplicationId, CallbackDispatcher, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkManager,
    RoutingDecision, SystemEvent, TaskContext, TaskId, TaskResult, TaskRouter, TaskStatus,
};

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
///
/// Fields are `pub(crate)` so sibling modules (`delegation`, `supervisor`, etc.)
/// can implement behavior via extension `impl` blocks without exposing internals
/// outside the `app_executor` module tree.
pub struct ApplicationExecutor {
    /// Application ID for isolation.
    pub application_id: ApplicationId,
    /// Application name (human-readable).
    pub application_name: String,

    /// Agents registered in this application.
    pub(crate) agents: Arc<RwLock<Vec<AgentInfo>>>,

    /// Task queue for this application (shared with worker).
    pub(crate) queue: Arc<ExecutionQueue>,

    /// Event bus for publishing system events.
    pub(crate) event_bus: EventBus,

    /// Router for matching tasks to agents.
    pub(crate) router: TaskRouter,

    /// Callback dispatcher for pushing results to coordinators.
    pub(crate) callback_dispatcher: CallbackDispatcher,

    /// The agent runner that actually executes agents.
    pub(crate) runner: Arc<dyn AgentRunner>,

    /// Channel for sending commands to the worker.
    /// Wrapped in Arc<RwLock<...>> so the supervisor can swap it after each restart.
    pub(crate) command_tx: Arc<RwLock<mpsc::Sender<ExecutorCommand>>>,

    /// Channel for receiving events from the worker.
    pub(crate) event_rx: Option<mpsc::Receiver<ExecutorEvent>>,

    /// Broadcast sender for executor events (for external subscribers like SSE).
    pub(crate) event_broadcast: tokio::sync::broadcast::Sender<ExecutorEvent>,

    /// Fork Manager for Fork-Join workflow.
    pub(crate) fork_manager: Arc<ForkManager>,

    /// Shutdown signal.
    pub(crate) shutdown: Arc<RwLock<bool>>,

    /// Worker heartbeat timestamp (updated every 10 seconds).
    pub(crate) worker_heartbeat: Arc<RwLock<Instant>>,

    /// Worker state (Running/Disconnected/Shutdown).
    pub(crate) worker_state: Arc<RwLock<WorkerState>>,

    /// Flag set by shutdown() to tell the supervisor not to restart.
    pub(crate) shutdown_requested: Arc<AtomicBool>,

    /// Number of times the worker has been restarted.
    pub(crate) restart_count: Arc<AtomicU32>,
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
        let mut queue_inner = crate::executor::queue::ExecutionQueue::new_with_store(
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
        let mut fork_manager_inner = crate::executor::fork_manager::ForkManager::new_with_store(
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
}
