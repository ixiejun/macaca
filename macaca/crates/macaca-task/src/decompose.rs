use async_trait::async_trait;

use macaca_proto::{MacacaResult, Task, TaskRequest};

/// Trait for breaking a high-level task into sub-task requests.
/// Implementations may call an LLM or apply heuristic rules.
#[async_trait]
pub trait TaskDecomposer: Send + Sync {
    /// Decompose `task` into zero or more child `TaskRequest`s.
    /// Returning an empty vec means the task should be executed as-is.
    async fn decompose(&self, task: &Task) -> MacacaResult<Vec<TaskRequest>>;
}

/// A no-op decomposer that treats every task as atomic.
pub struct SimpleDecomposer;

#[async_trait]
impl TaskDecomposer for SimpleDecomposer {
    async fn decompose(&self, _task: &Task) -> MacacaResult<Vec<TaskRequest>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{Task, TaskId, TaskPriority, TaskStatus};
    use chrono::Utc;

    fn sample_task() -> Task {
        let now = Utc::now();
        Task {
            id: TaskId::new(),
            description: "sample".into(),
            status: TaskStatus::Pending,
            priority: TaskPriority::Normal,
            assigned_agent: None,
            subtasks: Vec::new(),
            parent: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn simple_decomposer_returns_empty() {
        let d = SimpleDecomposer;
        let task = sample_task();
        let subtasks = d.decompose(&task).await.unwrap();
        assert!(subtasks.is_empty());
    }

    #[tokio::test]
    async fn decomposer_is_object_safe() {
        let d: Box<dyn TaskDecomposer> = Box::new(SimpleDecomposer);
        let task = sample_task();
        let subtasks = d.decompose(&task).await.unwrap();
        assert!(subtasks.is_empty());
    }
}
