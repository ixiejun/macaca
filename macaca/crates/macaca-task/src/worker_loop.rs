//! Worker Agent autonomous execution loop.
//!
//! Continuously: claim task → execute → submit for review → repeat.
//! When idle, polls the task board on a heartbeat interval.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tracing::info;

use crate::todo_board::TaskBoard;

/// Configuration for the Worker Agent loop.
pub struct WorkerLoopConfig {
    /// How often to poll the board when idle (default: 10s).
    pub heartbeat_interval: Duration,
    /// Maximum continuous execution time before yielding (default: 30min).
    pub max_execution_time: Duration,
}

impl Default for WorkerLoopConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(3),
            max_execution_time: Duration::from_secs(30 * 60),
        }
    }
}

/// Events emitted by the Worker loop for the agent runtime to act on.
#[derive(Debug, Clone)]
pub enum WorkerEvent {
    /// A task was claimed and needs execution.
    TaskClaimed {
        task_id: macaca_proto::TaskId,
        title: String,
        description: String,
        acceptance_criteria: Vec<String>,
        context: Option<String>,
        optimization_suggestions: Option<String>,
        attempt: u32,
        /// Session scope for tracing / UI (mirrors `TodoItem.session_id`).
        session_id: Option<String>,
    },
    /// A task needing optimization was found.
    RetryTask {
        task_id: macaca_proto::TaskId,
        title: String,
        description: String,
        optimization_suggestions: String,
        attempt: u32,
        session_id: Option<String>,
    },
    /// No tasks available — agent is idle.
    Idle,
}

/// The Worker Agent's autonomous execution loop.
///
/// This loop handles the pull-based work model:
/// 1. Claim highest-priority pending task
/// 2. Emit a WorkerEvent for the agent runtime to execute
/// 3. After execution, check for optimization tasks
/// 4. If no work, sleep for heartbeat interval and check again
///
/// The actual task execution (LLM calls, tool use) is done by the caller
/// who receives WorkerEvents and drives the agent's agentic loop.
pub struct WorkerLoop {
    board: Arc<TaskBoard>,
    config: WorkerLoopConfig,
    notify: Arc<tokio::sync::Notify>,
}

/// Handle to wake a WorkerLoop immediately (e.g., when tasks are assigned to this agent).
#[derive(Clone)]
pub struct WorkerLoopWaker {
    notify: Arc<tokio::sync::Notify>,
}

impl WorkerLoopWaker {
    /// Wake the worker loop immediately to check for new tasks.
    pub fn wake(&self) {
        self.notify.notify_one();
    }
}

impl WorkerLoop {
    #[deprecated(note = "Use WorkerLoop::with_components instead")]
    pub fn new(board: Arc<TaskBoard>, config: WorkerLoopConfig) -> Self {
        Self::with_components(board, config)
    }

    pub fn with_components(board: Arc<TaskBoard>, config: WorkerLoopConfig) -> Self {
        Self {
            board,
            config,
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Get a waker handle for this worker loop.
    pub fn waker(&self) -> WorkerLoopWaker {
        WorkerLoopWaker {
            notify: Arc::clone(&self.notify),
        }
    }

    /// Run the worker loop until shutdown is signaled.
    ///
    /// Emits `WorkerEvent`s for the agent runtime to process.
    #[deprecated(note = "Use WorkerLoop::run_with_default_template instead")]
    pub async fn run(
        &self,
        shutdown: Arc<AtomicBool>,
        event_tx: tokio::sync::mpsc::Sender<WorkerEvent>,
    ) {
        self.run_with_default_template(shutdown, event_tx).await;
    }

    pub async fn run_with_default_template(
        &self,
        shutdown: Arc<AtomicBool>,
        event_tx: tokio::sync::mpsc::Sender<WorkerEvent>,
    ) {
        info!(agent = self.board.agent_name(), "Worker loop started");

        loop {
            if shutdown.load(Ordering::SeqCst) {
                info!(agent = self.board.agent_name(), "Worker loop shutting down");
                break;
            }

            if self.try_claim_task(&shutdown, &event_tx).await {
                continue;
            }

            if self.try_retry_task(&shutdown, &event_tx).await {
                continue;
            }

            self.emit_idle_if_needed(&event_tx).await;
            if self.wait_for_next_tick(&shutdown).await {
                break;
            }
        }
    }

    async fn try_claim_task(
        &self,
        shutdown: &Arc<AtomicBool>,
        event_tx: &tokio::sync::mpsc::Sender<WorkerEvent>,
    ) -> bool {
        let Some(task) = self.board.claim_next_task().await else {
            return false;
        };

        info!(
            agent = self.board.agent_name(),
            task_id = %task.id,
            title = %task.title,
            priority = task.priority,
            "Task claimed"
        );

        self.board.mark_task_in_progress(&task.id).await;

        let claimed_id = task.id;
        let _ = event_tx
            .send(WorkerEvent::TaskClaimed {
                task_id: task.id,
                title: task.title,
                description: task.description,
                acceptance_criteria: task.acceptance_criteria,
                context: task.context,
                optimization_suggestions: task.optimization_suggestions,
                attempt: task.attempt_count,
                session_id: task.session_id.clone(),
            })
            .await;

        self.wait_for_task_exit(shutdown, claimed_id).await;
        true
    }

    async fn try_retry_task(
        &self,
        shutdown: &Arc<AtomicBool>,
        event_tx: &tokio::sync::mpsc::Sender<WorkerEvent>,
    ) -> bool {
        let opt_tasks = self.board.needs_optimization_tasks().await;
        let Some(task) = opt_tasks.into_iter().next() else {
            return false;
        };

        info!(
            agent = self.board.agent_name(),
            task_id = %task.id,
            attempt = task.attempt_count,
            "Retrying task with optimization suggestions"
        );

        self.board.mark_task_in_progress(&task.id).await;

        let retry_id = task.id;
        let _ = event_tx
            .send(WorkerEvent::RetryTask {
                task_id: task.id,
                title: task.title,
                description: task.description,
                optimization_suggestions: task.optimization_suggestions.unwrap_or_default(),
                attempt: task.attempt_count,
                session_id: task.session_id.clone(),
            })
            .await;

        self.wait_for_task_exit(shutdown, retry_id).await;
        true
    }

    async fn emit_idle_if_needed(&self, event_tx: &tokio::sync::mpsc::Sender<WorkerEvent>) {
        let has_any = !self.board.list_all().await.is_empty();
        if has_any {
            let _ = event_tx.send(WorkerEvent::Idle).await;
        }
    }

    async fn wait_for_task_exit(&self, shutdown: &Arc<AtomicBool>, task_id: macaca_proto::TaskId) {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            match self.board.current_task().await {
                Some(t) if t.id == task_id => continue,
                _ => break,
            }
        }
    }

    async fn wait_for_next_tick(&self, shutdown: &Arc<AtomicBool>) -> bool {
        tokio::select! {
            _ = tokio::time::sleep(self.config.heartbeat_interval) => false,
            _ = self.notify.notified() => {
                tracing::info!(agent = self.board.agent_name(), "Worker loop woken up by event");
                false
            }
            _ = async {
                while !shutdown.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            } => true
        }
    }
}

/// Convenience: check if a worker has pending work without running the full loop.
pub async fn has_work(board: &TaskBoard) -> bool {
    board.has_pending_tasks().await || !board.needs_optimization_tasks().await.is_empty()
}
