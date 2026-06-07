//! Execution Queue - Priority-based task queue for agent execution.
//!
//! Optional durability is expressed through `KernelPersistencePort` so the
//! queue can checkpoint pending work without depending on a concrete database
//! crate.  This keeps execution scheduling in the kernel and backend selection
//! in service/runtime composition.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

use super::{DelegatedTask, TaskId, TaskResult, TaskStatus};
use macaca_proto::ApplicationId;

use crate::persistence::KernelPersistencePort;

/// Maximum number of tasks that can be queued.
const DEFAULT_MAX_QUEUE_SIZE: usize = 100;
/// Maximum number of tasks that can run in parallel.
const DEFAULT_MAX_PARALLEL: usize = 4;

/// A task with priority for the priority queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedTask {
    /// The task to execute.
    pub task: DelegatedTask,
    /// Priority score (higher = more urgent).
    pub priority_score: i32,
}

impl PrioritizedTask {
    /// Create a new prioritized task.
    pub fn new(task: DelegatedTask) -> Self {
        // Calculate priority score: base priority + bonus for parallel tasks
        let bonus = if task.parallel { 1 } else { 0 };
        let priority_score = (task.priority as i32) * 10 + bonus;

        Self {
            task,
            priority_score,
        }
    }
}

impl Eq for PrioritizedTask {}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority_score == other.priority_score
    }
}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first (reverse order for min-heap behavior)
        other.priority_score.cmp(&self.priority_score)
    }
}

/// Execution queue with priority support and concurrency control.
pub struct ExecutionQueue {
    /// Pending tasks waiting to be executed.
    pending: Arc<RwLock<Vec<PrioritizedTask>>>,
    /// Currently running tasks.
    running: Arc<RwLock<HashMap<TaskId, DelegatedTask>>>,
    /// Completed task results.
    results: Arc<RwLock<HashMap<TaskId, TaskResult>>>,
    /// Maximum concurrent tasks.
    max_parallel: usize,
    /// Maximum queue size.
    max_queue_size: usize,
    /// Sender for task completion events.
    completion_tx: Option<mpsc::Sender<TaskResult>>,
    /// Optional persistence port for durability across restarts.
    store: Option<Arc<dyn KernelPersistencePort>>,
    /// Application ID used for key namespacing in the store.
    app_id: ApplicationId,
}

impl ExecutionQueue {
    /// Create a new execution queue.
    pub fn new(max_parallel: usize, max_queue_size: usize) -> Self {
        Self {
            pending: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            max_parallel,
            max_queue_size,
            completion_tx: None,
            store: None,
            app_id: ApplicationId::new(),
        }
    }

    /// Create a new execution queue with optional persistence.
    pub fn new_with_store(
        max_parallel: usize,
        max_queue_size: usize,
        store: Option<Arc<dyn KernelPersistencePort>>,
        app_id: ApplicationId,
    ) -> Self {
        if let Some(store) = &store {
            tracing::info!(
                app_id = %app_id,
                backend = store.backend_name(),
                "execution queue persistence enabled"
            );
        }
        Self {
            pending: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            max_parallel,
            max_queue_size,
            completion_tx: None,
            store,
            app_id,
        }
    }

    /// Create with default settings.
    pub fn default() -> Self {
        Self::new(DEFAULT_MAX_PARALLEL, DEFAULT_MAX_QUEUE_SIZE)
    }

    /// Set the completion channel.
    pub fn with_completion_channel(mut self, tx: mpsc::Sender<TaskResult>) -> Self {
        self.completion_tx = Some(tx);
        self
    }

    /// Enqueue a task for execution.
    ///
    /// Returns `Err` if the queue is full.
    pub async fn enqueue(&self, task: DelegatedTask) -> Result<TaskId, QueueError> {
        let pending = self.pending.read().await;

        if pending.len() >= self.max_queue_size {
            return Err(QueueError::QueueFull {
                current_size: pending.len(),
                max_size: self.max_queue_size,
            });
        }

        let task_id = task.id;
        drop(pending);

        let prioritized = PrioritizedTask::new(task);

        // Persist before inserting into memory.  This ordering gives restart
        // recovery a chance to replay the task if the process exits between
        // the durable write and the in-memory queue update.
        if let Some(ref store) = self.store {
            let key = format!("exec_queue/{}/{}", self.app_id.0, task_id.0);
            match serde_json::to_vec(&prioritized) {
                Ok(bytes) => {
                    if let Err(e) = store.set(&key, &bytes).await {
                        tracing::warn!(task_id = %task_id, error = %e, "Failed to persist queued task");
                    }
                }
                Err(e) => {
                    tracing::warn!(task_id = %task_id, error = %e, "Failed to serialize queued task");
                }
            }
        }

        let mut pending = self.pending.write().await;
        pending.push(prioritized);
        // Re-sort by priority (highest first)
        pending.sort_by(|a, b| b.cmp(a)); // Reverse for max-heap behavior
        drop(pending);

        tracing::info!(
            "Task enqueued: task_id={}, queue_size={}",
            task_id,
            self.pending.read().await.len()
        );

        Ok(task_id)
    }

    /// Admit a task directly into the running set for service-backed execution.
    ///
    /// Service-backed delegations bypass the worker dequeue loop but still need
    /// queue visibility (`get_task_status`) and the same concurrency accounting
    /// as worker-executed tasks.  Callers outside the kernel worker channel own
    /// the actual agent execution and must publish completion through the
    /// executor completion helpers.
    pub async fn admit_running_task(&self, task: DelegatedTask) -> Result<TaskId, QueueError> {
        let running_count = self.running.read().await.len();
        if running_count >= self.max_parallel {
            return Err(QueueError::ConcurrencyLimitReached {
                running: running_count,
                max_parallel: self.max_parallel,
            });
        }

        let task_id = task.id;
        self.running.write().await.insert(task_id, task);
        tracing::info!(
            task_id = %task_id,
            running = running_count + 1,
            "Task admitted for service-backed execution"
        );
        Ok(task_id)
    }

    /// Try to dequeue the next highest priority task.
    ///
    /// Returns `None` if:
    /// - The queue is empty
    /// - Too many tasks are already running (concurrency limit)
    pub async fn dequeue(&self) -> Option<DelegatedTask> {
        // Check concurrency limit
        let running_count = self.running.read().await.len();
        if running_count >= self.max_parallel {
            return None;
        }

        let mut pending = self.pending.write().await;
        if pending.is_empty() {
            return None;
        }

        let task = pending.pop().map(|p| p.task).take()?;
        let task_id = task.id;

        // Remove from persistent store.  The queued memento represents pending
        // work only; once the task is running the worker owns completion and
        // failure observation through executor events.
        if let Some(ref store) = self.store {
            let key = format!("exec_queue/{}/{}", self.app_id.0, task_id.0);
            if let Err(e) = store.delete(&key).await {
                tracing::warn!(task_id = %task_id, error = %e, "Failed to delete dequeued task from store");
            }
        }

        // Add to running
        self.running.write().await.insert(task_id, task.clone());
        drop(pending);

        tracing::info!(task_id = %task_id, running = running_count + 1, "Task dequeued for execution");

        Some(task)
    }

    /// Restore pending tasks from the persistence store after a restart.
    ///
    /// Tasks that were previously in `Running` state are rolled back to `Queued`
    /// (since the executor that was running them is gone).
    pub async fn restore_from_store(&mut self) {
        let store = match &self.store {
            Some(s) => Arc::clone(s),
            None => return,
        };

        let prefix = format!("exec_queue/{}/", self.app_id.0);
        let keys = match store.list_keys(&prefix).await {
            Ok(k) => k,
            Err(e) => {
                tracing::error!(error = %e, "Failed to list exec_queue keys during restore");
                return;
            }
        };

        let mut pending = self.pending.write().await;
        let mut restored = 0usize;

        for key in &keys {
            let bytes = match store.get(key).await {
                Ok(Some(b)) => b,
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!(key = %key, error = %e, "Failed to read queued task during restore");
                    continue;
                }
            };

            match serde_json::from_slice::<PrioritizedTask>(&bytes) {
                Ok(prioritized) => {
                    pending.push(prioritized);
                    restored += 1;
                }
                Err(e) => {
                    tracing::warn!(key = %key, error = %e, "Failed to deserialize queued task during restore");
                }
            }
        }

        // Re-sort by priority after bulk insert
        pending.sort_by(|a, b| b.cmp(a));

        if restored > 0 {
            tracing::info!(restored = restored, app_id = %self.app_id.0, "Restored queued tasks from store");
        }
    }

    /// Mark a task as completed and remove it from running.
    pub async fn complete(&self, task_id: TaskId, result: TaskResult) {
        let task = self.running.write().await.remove(&task_id);

        if let Some(ref tx) = self.completion_tx {
            let _ = tx.send(result).await;
        }

        if task.is_some() {
            tracing::info!(task_id = %task_id, "Task completed and removed from running");
        }
    }

    /// Mark a task as failed.
    pub async fn fail(&self, task_id: TaskId, error: String) {
        let task = self.running.write().await.remove(&task_id);

        let result = TaskResult {
            task_id,
            success: false,
            output: String::new(),
            error: Some(error),
            artifacts: vec![],
            completed_at: chrono::Utc::now(),
            tokens_used: None,
        };

        if let Some(ref tx) = self.completion_tx {
            let _ = tx.send(result).await;
        }

        if task.is_some() {
            tracing::error!(task_id = %task_id, "Task failed");
        }
    }

    /// Cancel a task.
    pub async fn cancel(&self, task_id: TaskId) -> bool {
        // Try to remove from pending
        let mut pending = self.pending.write().await;
        let idx = pending.iter().position(|p| p.task.id == task_id);

        if let Some(idx) = idx {
            pending.remove(idx);
            drop(pending);

            // Remove from persistent store
            if let Some(ref store) = self.store {
                let key = format!("exec_queue/{}/{}", self.app_id.0, task_id.0);
                if let Err(e) = store.delete(&key).await {
                    tracing::warn!(task_id = %task_id, error = %e, "Failed to delete cancelled task from store");
                }
            }

            tracing::info!(task_id = %task_id, "Task cancelled (was pending)");
            return true;
        }
        drop(pending);

        // Try to remove from running
        let mut running = self.running.write().await;
        if running.remove(&task_id).is_some() {
            tracing::info!(task_id = %task_id, "Task cancelled (was running)");
            return true;
        }

        false
    }

    /// Get the number of pending tasks.
    pub async fn pending_count(&self) -> usize {
        self.pending.read().await.len()
    }

    /// Get the number of running tasks.
    pub async fn running_count(&self) -> usize {
        self.running.read().await.len()
    }

    /// Check if the queue is empty.
    pub async fn is_empty(&self) -> bool {
        self.pending.read().await.is_empty() && self.running.read().await.is_empty()
    }

    /// Get queue status.
    pub async fn status(&self) -> QueueStatus {
        QueueStatus {
            pending: self.pending.read().await.len(),
            running: self.running.read().await.len(),
            max_parallel: self.max_parallel,
            max_queue_size: self.max_queue_size,
        }
    }

    /// Store a task result.
    pub async fn store_result(&self, result: TaskResult) {
        let task_id = result.task_id;
        self.results.write().await.insert(task_id, result);
        tracing::info!(task_id = %task_id, "Task result stored");
    }

    /// Get a task result by ID.
    pub async fn get_result(&self, task_id: &TaskId) -> Option<TaskResult> {
        self.results.read().await.get(task_id).cloned()
    }

    /// Get task status by ID.
    pub async fn get_status(&self, task_id: &TaskId) -> Option<TaskStatus> {
        // Check results first (completed)
        if self.results.read().await.contains_key(task_id) {
            return Some(TaskStatus::Completed);
        }
        // Check running
        if self.running.read().await.contains_key(task_id) {
            return Some(TaskStatus::Running);
        }
        // Check pending
        if self
            .pending
            .read()
            .await
            .iter()
            .any(|p| p.task.id == *task_id)
        {
            return Some(TaskStatus::Queued);
        }
        None
    }
}

/// Queue status snapshot.
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueueStatus {
    pub pending: usize,
    pub running: usize,
    pub max_parallel: usize,
    pub max_queue_size: usize,
}

/// Queue errors.
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue is full: {current_size}/{max_size}")]
    QueueFull {
        current_size: usize,
        max_size: usize,
    },

    #[error("Task not found: {0}")]
    TaskNotFound(TaskId),

    #[error("Concurrency limit reached: {running}/{max_parallel}")]
    ConcurrencyLimitReached { running: usize, max_parallel: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::ApplicationId;

    fn create_test_task(priority: u8) -> DelegatedTask {
        DelegatedTask {
            id: TaskId::new(),
            application_id: ApplicationId::new(),
            from_agent: "coordinator".into(),
            to_agent: "backend".into(),
            prompt: "Test task".into(),
            priority,
            parallel: false,
            created_at: Utc::now(),
            deadline: None,
            parent_task: None,
            context: None,
        }
    }

    #[tokio::test]
    async fn test_enqueue_dequeue_priority() {
        let queue = ExecutionQueue::default();

        // Enqueue tasks with different priorities
        queue.enqueue(create_test_task(1)).await.unwrap();
        queue.enqueue(create_test_task(10)).await.unwrap();
        queue.enqueue(create_test_task(5)).await.unwrap();

        // First dequeue should get priority 10
        let task = queue.dequeue().await.unwrap();
        assert_eq!(task.priority, 10);

        // Second should get priority 5
        let task = queue.dequeue().await.unwrap();
        assert_eq!(task.priority, 5);

        // Third should get priority 1
        let task = queue.dequeue().await.unwrap();
        assert_eq!(task.priority, 1);
    }

    #[tokio::test]
    async fn test_concurrency_limit() {
        let queue = ExecutionQueue::new(2, 10);

        // Enqueue 3 tasks
        queue.enqueue(create_test_task(5)).await.unwrap();
        queue.enqueue(create_test_task(5)).await.unwrap();
        queue.enqueue(create_test_task(5)).await.unwrap();

        // First two should dequeue
        assert!(queue.dequeue().await.is_some());
        assert!(queue.dequeue().await.is_some());

        // Third should not dequeue due to concurrency limit
        assert!(queue.dequeue().await.is_none());
    }

    #[tokio::test]
    async fn test_admit_running_task_respects_concurrency_limit() {
        let queue = ExecutionQueue::new(1, 10);
        queue.admit_running_task(create_test_task(5)).await.unwrap();
        let second = queue.admit_running_task(create_test_task(3)).await;
        assert!(matches!(
            second,
            Err(QueueError::ConcurrencyLimitReached { .. })
        ));
    }

    #[tokio::test]
    async fn test_queue_full() {
        let queue = ExecutionQueue::new(10, 2);

        queue.enqueue(create_test_task(1)).await.unwrap();
        queue.enqueue(create_test_task(1)).await.unwrap();

        // Third should fail
        let result = queue.enqueue(create_test_task(1)).await;
        assert!(result.is_err());
    }
}
