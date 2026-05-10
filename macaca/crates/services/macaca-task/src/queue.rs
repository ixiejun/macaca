use std::cmp::Ordering;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::{Mutex, Notify};

use macaca_proto::{TaskId, TaskPriority};

/// A task entry held in the binary heap.
struct PrioritizedTask {
    task_id: TaskId,
    priority: TaskPriority,
    created_at: DateTime<Utc>,
}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for PrioritizedTask {}

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first; on tie, earlier created_at first.
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => other.created_at.cmp(&self.created_at),
            ord => ord,
        }
    }
}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Priority queue for pending tasks. Higher priority tasks are dequeued first.
/// On equal priority the earliest-created task is dequeued first.
pub struct TaskQueue {
    queue: Arc<Mutex<std::collections::BinaryHeap<PrioritizedTask>>>,
    notify: Arc<Notify>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(std::collections::BinaryHeap::new())),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Push a task onto the queue and wake any waiting consumer.
    pub async fn push(&self, task_id: TaskId, priority: TaskPriority, created_at: DateTime<Utc>) {
        self.queue.lock().await.push(PrioritizedTask {
            task_id,
            priority,
            created_at,
        });
        self.notify.notify_one();
    }

    /// Block until a task is available, then return its id.
    pub async fn pop(&self) -> TaskId {
        loop {
            {
                let mut queue = self.queue.lock().await;
                if let Some(entry) = queue.pop() {
                    return entry.task_id;
                }
            }
            self.notify.notified().await;
        }
    }

    /// Non-blocking pop. Returns `None` if the queue is empty.
    pub fn try_pop(&self) -> Option<TaskId> {
        self.queue.try_lock().ok()?.pop().map(|e| e.task_id)
    }

    /// Number of tasks currently in the queue.
    pub fn len(&self) -> usize {
        self.queue.try_lock().map(|q| q.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn push_and_try_pop() {
        let q = TaskQueue::new();
        let id = TaskId::new();
        q.push(id, TaskPriority::Normal, Utc::now()).await;
        assert_eq!(q.try_pop(), Some(id));
        assert!(q.try_pop().is_none());
    }

    #[tokio::test]
    async fn priority_ordering() {
        let q = TaskQueue::new();
        let low_id = TaskId::new();
        let high_id = TaskId::new();
        let crit_id = TaskId::new();

        q.push(low_id, TaskPriority::Low, Utc::now()).await;
        q.push(high_id, TaskPriority::High, Utc::now()).await;
        q.push(crit_id, TaskPriority::Critical, Utc::now()).await;

        assert_eq!(q.pop().await, crit_id);
        assert_eq!(q.pop().await, high_id);
        assert_eq!(q.pop().await, low_id);
    }

    #[tokio::test]
    async fn same_priority_fifo_by_created_at() {
        let q = TaskQueue::new();
        let t0 = Utc::now();
        let t1 = t0 + chrono::Duration::milliseconds(1);
        let t2 = t0 + chrono::Duration::milliseconds(2);

        let id0 = TaskId::new();
        let id1 = TaskId::new();
        let id2 = TaskId::new();

        // Push out of chronological order to verify sorting by created_at.
        q.push(id2, TaskPriority::Normal, t2).await;
        q.push(id0, TaskPriority::Normal, t0).await;
        q.push(id1, TaskPriority::Normal, t1).await;

        assert_eq!(q.pop().await, id0);
        assert_eq!(q.pop().await, id1);
        assert_eq!(q.pop().await, id2);
    }

    #[tokio::test]
    async fn blocking_pop_wakes_on_push() {
        let q = Arc::new(TaskQueue::new());
        let q_clone = Arc::clone(&q);

        let handle = tokio::spawn(async move { q_clone.pop().await });

        // Give the spawned task time to block.
        tokio::time::sleep(Duration::from_millis(10)).await;

        let id = TaskId::new();
        q.push(id, TaskPriority::Normal, Utc::now()).await;

        let received = tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("timed out")
            .unwrap();

        assert_eq!(received, id);
    }

    #[tokio::test]
    async fn len_and_is_empty() {
        let q = TaskQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);

        q.push(TaskId::new(), TaskPriority::Low, Utc::now()).await;
        assert!(!q.is_empty());
        assert_eq!(q.len(), 1);

        q.try_pop();
        assert!(q.is_empty());
    }
}
