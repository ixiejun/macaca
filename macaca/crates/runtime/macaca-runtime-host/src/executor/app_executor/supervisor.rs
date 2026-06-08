//! Worker supervisor — spawns and restarts the command-processing worker.
//!
//! Implements the Supervisor pattern: on unexpected worker exit, recreates the
//! command channel, resets health markers, and retries up to `max_restarts`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

use super::executor::ApplicationExecutor;
use super::types::{WorkerState, WorkerSupervisorConfig};
use crate::executor::{
    AgentInfo, AgentRunner, ApplicationId, CallbackDispatcher, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkManager,
    RoutingDecision, SystemEvent, TaskContext, TaskId, TaskResult, TaskRouter, TaskStatus,
};

impl ApplicationExecutor {
    /// Supervisor loop: spawns the worker and restarts it on unexpected exit.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn supervisor_loop(
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
}
