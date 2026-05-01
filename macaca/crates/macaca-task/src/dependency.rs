use macaca_proto::{TaskId, TodoGoal, TodoGoalStatus, TodoItem, TodoStatus};

pub trait TaskDependencyResolver: Send + Sync {
    fn initial_status_for_new_task(&self, depends_on: &[TaskId], todos: &[TodoItem]) -> TodoStatus;
    fn can_claim_task(&self, task: &TodoItem, goals: &[TodoGoal]) -> bool;
    fn blocked_tasks_ready_after_completion(
        &self,
        todos: &[TodoItem],
        completed_id: &TaskId,
    ) -> Vec<TaskId>;
}

#[derive(Debug, Default)]
pub struct DefaultTaskDependencyResolver;

impl TaskDependencyResolver for DefaultTaskDependencyResolver {
    fn initial_status_for_new_task(&self, depends_on: &[TaskId], todos: &[TodoItem]) -> TodoStatus {
        if depends_on.is_empty() {
            return TodoStatus::Pending;
        }

        let completed_ids: Vec<TaskId> = todos
            .iter()
            .filter(|t| t.status == TodoStatus::Completed)
            .map(|t| t.id)
            .collect();

        if depends_on.iter().all(|dep| completed_ids.contains(dep)) {
            TodoStatus::Pending
        } else {
            TodoStatus::Blocked
        }
    }

    fn can_claim_task(&self, task: &TodoItem, goals: &[TodoGoal]) -> bool {
        let Some(goal_id) = task.parent_task else {
            return true;
        };

        goals
            .iter()
            .any(|goal| goal.id == goal_id && goal.status == TodoGoalStatus::InProgress)
    }

    fn blocked_tasks_ready_after_completion(
        &self,
        todos: &[TodoItem],
        completed_id: &TaskId,
    ) -> Vec<TaskId> {
        let completed_ids: Vec<TaskId> = todos
            .iter()
            .filter(|t| t.status == TodoStatus::Completed)
            .map(|t| t.id)
            .collect();

        todos
            .iter()
            .filter(|item| {
                item.status == TodoStatus::Blocked && item.depends_on.contains(completed_id)
            })
            .filter(|item| {
                item.depends_on
                    .iter()
                    .all(|dep| completed_ids.contains(dep))
            })
            .map(|item| item.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::ApplicationId;

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
    fn resolver_blocks_new_tasks_with_unmet_dependencies() {
        let resolver = DefaultTaskDependencyResolver;
        let dep = make_task(TodoStatus::Pending);
        let status = resolver.initial_status_for_new_task(&[dep.id], &[dep]);
        assert_eq!(status, TodoStatus::Blocked);
    }

    #[test]
    fn resolver_unblocks_only_after_all_dependencies_complete() {
        let resolver = DefaultTaskDependencyResolver;
        let dep1 = make_task(TodoStatus::Completed);
        let dep2 = make_task(TodoStatus::Completed);
        let mut blocked = make_task(TodoStatus::Blocked);
        blocked.depends_on = vec![dep1.id, dep2.id];
        let ready = resolver.blocked_tasks_ready_after_completion(
            &[dep1.clone(), dep2.clone(), blocked.clone()],
            &dep2.id,
        );
        assert_eq!(ready, vec![blocked.id]);

        let cancelled = make_task(TodoStatus::Cancelled);
        let mut still_blocked = make_task(TodoStatus::Blocked);
        still_blocked.depends_on = vec![cancelled.id];
        let ready = resolver.blocked_tasks_ready_after_completion(
            &[cancelled, still_blocked.clone()],
            &still_blocked.depends_on[0],
        );
        assert!(ready.is_empty());
    }
}
