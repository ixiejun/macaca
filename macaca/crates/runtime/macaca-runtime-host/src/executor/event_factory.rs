//! Canonical executor lifecycle event construction.

use chrono::Utc;

use super::{ExecutorEvent, TaskResult};
use super::TaskId;

/// Factory for task-scoped executor lifecycle events.
#[derive(Debug, Clone)]
pub struct ExecutorEventFactory {
    task_id: TaskId,
    agent: String,
}

impl ExecutorEventFactory {
    /// Create a factory for one task/agent pair.
    pub fn new(task_id: TaskId, agent: impl Into<String>) -> Self {
        Self {
            task_id,
            agent: agent.into(),
        }
    }

    /// Create a task-started event.
    pub fn started(&self) -> ExecutorEvent {
        ExecutorEvent::TaskStarted {
            task_id: self.task_id,
            agent: self.agent.clone(),
        }
    }

    /// Create a successful task result.
    pub fn success_result(&self, output: impl Into<String>) -> TaskResult {
        TaskResult {
            task_id: self.task_id,
            success: true,
            output: output.into(),
            error: None,
            artifacts: vec![],
            completed_at: Utc::now(),
            tokens_used: None,
        }
    }

    /// Create a failed task result.
    pub fn failed_result(&self, error: impl Into<String>) -> TaskResult {
        TaskResult {
            task_id: self.task_id,
            success: false,
            output: String::new(),
            error: Some(error.into()),
            artifacts: vec![],
            completed_at: Utc::now(),
            tokens_used: None,
        }
    }

    /// Create a completed event from output text.
    pub fn completed(&self, output: impl Into<String>) -> ExecutorEvent {
        self.completed_with_result(self.success_result(output))
    }

    /// Create a completed event from an existing result.
    ///
    /// The result task id is normalized to the factory task id so delegated
    /// runners cannot accidentally emit a mismatched id.
    pub fn completed_with_result(&self, mut result: TaskResult) -> ExecutorEvent {
        result.task_id = self.task_id;
        ExecutorEvent::TaskCompleted {
            task_id: self.task_id,
            agent: self.agent.clone(),
            result,
        }
    }

    /// Create a task-failed event.
    pub fn failed(&self, error: impl Into<String>) -> ExecutorEvent {
        ExecutorEvent::TaskFailed {
            task_id: self.task_id,
            agent: self.agent.clone(),
            error: error.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::TokenUsage;
    use super::*;

    #[test]
    fn started_preserves_task_and_agent() {
        let task_id = TaskId::new();
        let event = ExecutorEventFactory::new(task_id, "planner").started();

        match event {
            ExecutorEvent::TaskStarted {
                task_id: got,
                agent,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "planner");
            }
            other => panic!("expected TaskStarted, got {other:?}"),
        }
    }

    #[test]
    fn completed_preserves_result_fields() {
        let task_id = TaskId::new();
        let event = ExecutorEventFactory::new(task_id, "backend").completed("done");

        match event {
            ExecutorEvent::TaskCompleted {
                task_id: got,
                agent,
                result,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "backend");
                assert_eq!(result.task_id, task_id);
                assert!(result.success);
                assert_eq!(result.output, "done");
                assert_eq!(result.error, None);
                assert!(result.artifacts.is_empty());
                assert!(result.tokens_used.is_none());
            }
            other => panic!("expected TaskCompleted, got {other:?}"),
        }
    }

    #[test]
    fn completed_with_result_overwrites_task_id() {
        let task_id = TaskId::new();
        let wrong_task_id = TaskId::new();
        let result = TaskResult {
            task_id: wrong_task_id,
            success: true,
            output: "done".into(),
            error: None,
            artifacts: vec!["artifact.txt".into()],
            completed_at: Utc::now(),
            tokens_used: Some(TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 2,
                total_tokens: 3,
            }),
        };

        let event = ExecutorEventFactory::new(task_id, "frontend").completed_with_result(result);

        match event {
            ExecutorEvent::TaskCompleted { result, .. } => {
                assert_eq!(result.task_id, task_id);
                assert_eq!(result.artifacts, vec!["artifact.txt"]);
                assert_eq!(result.tokens_used.unwrap().total_tokens, 3);
            }
            other => panic!("expected TaskCompleted, got {other:?}"),
        }
    }

    #[test]
    fn failed_result_preserves_error() {
        let task_id = TaskId::new();
        let result = ExecutorEventFactory::new(task_id, "frontend").failed_result("boom");

        assert_eq!(result.task_id, task_id);
        assert!(!result.success);
        assert_eq!(result.output, "");
        assert_eq!(result.error, Some("boom".into()));
        assert!(result.artifacts.is_empty());
        assert!(result.tokens_used.is_none());
    }

    #[test]
    fn failed_preserves_error() {
        let task_id = TaskId::new();
        let event = ExecutorEventFactory::new(task_id, "frontend").failed("boom");

        match event {
            ExecutorEvent::TaskFailed {
                task_id: got,
                agent,
                error,
            } => {
                assert_eq!(got, task_id);
                assert_eq!(agent, "frontend");
                assert_eq!(error, "boom");
            }
            other => panic!("expected TaskFailed, got {other:?}"),
        }
    }
}
