use macaca_proto::{TodoItem, TodoReviewResult, TodoStatus};

pub trait TodoLifecyclePolicy: Send + Sync {
    fn on_claim(&self, task: &TodoItem) -> Option<TodoStatus>;
    fn on_start(&self, task: &TodoItem) -> Option<TodoStatus>;
    fn on_submit_for_review(&self, task: &TodoItem) -> Option<TodoStatus>;
    fn on_review(&self, task: &TodoItem, result: &TodoReviewResult) -> TodoStatus;
    fn on_skip(&self, task: &TodoItem) -> Option<TodoStatus>;
    fn on_mark_failed(&self, _task: &TodoItem) -> TodoStatus;
}

#[derive(Debug, Default)]
pub struct DefaultTodoLifecyclePolicy;

impl TodoLifecyclePolicy for DefaultTodoLifecyclePolicy {
    fn on_claim(&self, task: &TodoItem) -> Option<TodoStatus> {
        (task.status == TodoStatus::Pending).then_some(TodoStatus::Assigned)
    }

    fn on_start(&self, task: &TodoItem) -> Option<TodoStatus> {
        matches!(
            task.status,
            TodoStatus::Assigned | TodoStatus::NeedsOptimization
        )
        .then_some(TodoStatus::InProgress)
    }

    fn on_submit_for_review(&self, task: &TodoItem) -> Option<TodoStatus> {
        (task.status == TodoStatus::InProgress).then_some(TodoStatus::PendingReview)
    }

    fn on_review(&self, task: &TodoItem, result: &TodoReviewResult) -> TodoStatus {
        if result.passed {
            TodoStatus::Completed
        } else if task.attempt_count >= task.max_attempts {
            TodoStatus::Failed
        } else {
            TodoStatus::NeedsOptimization
        }
    }

    fn on_skip(&self, task: &TodoItem) -> Option<TodoStatus> {
        matches!(task.status, TodoStatus::Pending | TodoStatus::Blocked)
            .then_some(TodoStatus::Cancelled)
    }

    fn on_mark_failed(&self, _task: &TodoItem) -> TodoStatus {
        TodoStatus::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{ApplicationId, TodoItem};

    fn make_task(status: TodoStatus) -> TodoItem {
        let mut task = TodoItem::new(
            ApplicationId::new(),
            Some("session".into()),
            "backend",
            "planner",
            "title",
            "description",
            5,
        );
        task.status = status;
        task
    }

    #[test]
    fn lifecycle_policy_keeps_existing_transition_rules() {
        let policy = DefaultTodoLifecyclePolicy;
        let cases = vec![
            (
                TodoStatus::Pending,
                policy.on_claim(&make_task(TodoStatus::Pending)),
                Some(TodoStatus::Assigned),
            ),
            (
                TodoStatus::Assigned,
                policy.on_start(&make_task(TodoStatus::Assigned)),
                Some(TodoStatus::InProgress),
            ),
            (
                TodoStatus::NeedsOptimization,
                policy.on_start(&make_task(TodoStatus::NeedsOptimization)),
                Some(TodoStatus::InProgress),
            ),
            (
                TodoStatus::InProgress,
                policy.on_submit_for_review(&make_task(TodoStatus::InProgress)),
                Some(TodoStatus::PendingReview),
            ),
            (
                TodoStatus::Blocked,
                policy.on_skip(&make_task(TodoStatus::Blocked)),
                Some(TodoStatus::Cancelled),
            ),
        ];

        for (_from, actual, expected) in cases {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn lifecycle_policy_review_preserves_retry_and_fail_contract() {
        let policy = DefaultTodoLifecyclePolicy;
        let mut task = make_task(TodoStatus::PendingReview);
        task.attempt_count = 1;
        task.max_attempts = 3;

        let retry = policy.on_review(
            &task,
            &macaca_proto::TodoReviewResult {
                passed: false,
                feedback: "needs work".into(),
                verified_criteria: vec![],
            },
        );
        assert_eq!(retry, TodoStatus::NeedsOptimization);

        task.attempt_count = 3;
        let failed = policy.on_review(
            &task,
            &macaca_proto::TodoReviewResult {
                passed: false,
                feedback: "still broken".into(),
                verified_criteria: vec![],
            },
        );
        assert_eq!(failed, TodoStatus::Failed);
    }
}
