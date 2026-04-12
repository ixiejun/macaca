use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use chrono::Utc;
use tracing::{debug, warn};

use macaca_proto::{
    AgentId, MacacaError, MacacaResult, Task, TaskId, TaskRequest, TaskResult, TaskStatus,
};

pub struct TaskTracker {
    tasks: Arc<RwLock<HashMap<TaskId, Task>>>,
    results: Arc<RwLock<HashMap<TaskId, TaskResult>>>,
}

impl TaskTracker {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create(&self, request: TaskRequest) -> MacacaResult<Task> {
        let now = Utc::now();
        let task = Task {
            id: TaskId::new(),
            description: request.description,
            status: TaskStatus::Pending,
            priority: request.priority,
            assigned_agent: None,
            subtasks: Vec::new(),
            parent: None,
            created_at: now,
            updated_at: now,
        };
        debug!("Creating task {:?}", task.id);
        self.tasks.write().await.insert(task.id, task.clone());
        Ok(task)
    }

    pub async fn get(&self, id: &TaskId) -> MacacaResult<Option<Task>> {
        Ok(self.tasks.read().await.get(id).cloned())
    }

    pub async fn assign(&self, task_id: &TaskId, agent_id: AgentId) -> MacacaResult<()> {
        let mut tasks = self.tasks.write().await;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| MacacaError::NotFound(format!("task {:?}", task_id)))?;

        match task.status {
            TaskStatus::Pending => {
                task.status = TaskStatus::Assigned;
                task.assigned_agent = Some(agent_id);
                task.updated_at = Utc::now();
                debug!("Task {:?} assigned to agent {:?}", task_id, agent_id);
                Ok(())
            }
            other => {
                warn!("Invalid transition: {:?} -> Assigned", other);
                Err(MacacaError::Task(format!(
                    "cannot assign task in status {:?}: expected Pending",
                    other
                )))
            }
        }
    }

    pub async fn start(&self, task_id: &TaskId) -> MacacaResult<()> {
        let mut tasks = self.tasks.write().await;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| MacacaError::NotFound(format!("task {:?}", task_id)))?;

        match task.status {
            TaskStatus::Assigned => {
                task.status = TaskStatus::Running;
                task.updated_at = Utc::now();
                debug!("Task {:?} started", task_id);
                Ok(())
            }
            other => {
                warn!("Invalid transition: {:?} -> Running", other);
                Err(MacacaError::Task(format!(
                    "cannot start task in status {:?}: expected Assigned",
                    other
                )))
            }
        }
    }

    pub async fn complete(&self, task_id: &TaskId, result: TaskResult) -> MacacaResult<()> {
        let mut tasks = self.tasks.write().await;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| MacacaError::NotFound(format!("task {:?}", task_id)))?;

        match task.status {
            TaskStatus::Running => {
                task.status = TaskStatus::Completed;
                task.updated_at = Utc::now();
                debug!("Task {:?} completed", task_id);
                drop(tasks);
                self.results.write().await.insert(*task_id, result);
                Ok(())
            }
            other => {
                warn!("Invalid transition: {:?} -> Completed", other);
                Err(MacacaError::Task(format!(
                    "cannot complete task in status {:?}: expected Running",
                    other
                )))
            }
        }
    }

    pub async fn fail(&self, task_id: &TaskId, error: String) -> MacacaResult<()> {
        let mut tasks = self.tasks.write().await;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| MacacaError::NotFound(format!("task {:?}", task_id)))?;

        match task.status {
            TaskStatus::Running | TaskStatus::Pending => {
                task.status = TaskStatus::Failed;
                task.updated_at = Utc::now();
                warn!("Task {:?} failed: {}", task_id, error);
                Ok(())
            }
            other => Err(MacacaError::Task(format!(
                "cannot fail task in status {:?}: expected Running or Pending",
                other
            ))),
        }
    }

    pub async fn status(&self, task_id: &TaskId) -> MacacaResult<TaskStatus> {
        self.tasks
            .read()
            .await
            .get(task_id)
            .map(|t| t.status)
            .ok_or_else(|| MacacaError::NotFound(format!("task {:?}", task_id)))
    }

    pub async fn get_result(&self, task_id: &TaskId) -> MacacaResult<Option<TaskResult>> {
        Ok(self.results.read().await.get(task_id).cloned())
    }

    pub async fn list_by_status(&self, status: TaskStatus) -> MacacaResult<Vec<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks
            .values()
            .filter(|t| t.status == status)
            .cloned()
            .collect())
    }

    pub async fn list_by_agent(&self, agent_id: &AgentId) -> MacacaResult<Vec<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks
            .values()
            .filter(|t| t.assigned_agent.as_ref() == Some(agent_id))
            .cloned()
            .collect())
    }
}

impl Default for TaskTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::TaskPriority;

    fn make_request(desc: &str) -> TaskRequest {
        TaskRequest {
            description: desc.into(),
            priority: TaskPriority::Normal,
            requester: "test".into(),
        }
    }

    fn make_result(task_id: TaskId) -> TaskResult {
        TaskResult {
            task_id,
            success: true,
            output: "done".into(),
            artifacts: vec![],
            completed_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn create_and_get() {
        let tracker = TaskTracker::new();
        let task = tracker.create(make_request("test task")).await.unwrap();
        assert_eq!(task.status, TaskStatus::Pending);

        let fetched = tracker.get(&task.id).await.unwrap().unwrap();
        assert_eq!(fetched.description, "test task");
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let tracker = TaskTracker::new();
        let result = tracker.get(&TaskId::new()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn full_happy_path() {
        let tracker = TaskTracker::new();
        let task = tracker.create(make_request("work")).await.unwrap();
        let agent = AgentId::new();

        tracker.assign(&task.id, agent).await.unwrap();
        assert_eq!(
            tracker.status(&task.id).await.unwrap(),
            TaskStatus::Assigned
        );

        tracker.start(&task.id).await.unwrap();
        assert_eq!(tracker.status(&task.id).await.unwrap(), TaskStatus::Running);

        let result = make_result(task.id);
        tracker.complete(&task.id, result).await.unwrap();
        assert_eq!(
            tracker.status(&task.id).await.unwrap(),
            TaskStatus::Completed
        );

        let stored = tracker.get_result(&task.id).await.unwrap().unwrap();
        assert!(stored.success);
    }

    #[tokio::test]
    async fn fail_from_running() {
        let tracker = TaskTracker::new();
        let task = tracker.create(make_request("fail me")).await.unwrap();
        let agent = AgentId::new();
        tracker.assign(&task.id, agent).await.unwrap();
        tracker.start(&task.id).await.unwrap();
        tracker.fail(&task.id, "oops".into()).await.unwrap();
        assert_eq!(tracker.status(&task.id).await.unwrap(), TaskStatus::Failed);
    }

    #[tokio::test]
    async fn fail_from_pending() {
        let tracker = TaskTracker::new();
        let task = tracker.create(make_request("cancel me")).await.unwrap();
        tracker.fail(&task.id, "cancelled".into()).await.unwrap();
        assert_eq!(tracker.status(&task.id).await.unwrap(), TaskStatus::Failed);
    }

    #[tokio::test]
    async fn invalid_transition_assign_twice() {
        let tracker = TaskTracker::new();
        let task = tracker.create(make_request("t")).await.unwrap();
        let agent = AgentId::new();
        tracker.assign(&task.id, agent).await.unwrap();
        let err = tracker.assign(&task.id, agent).await.unwrap_err();
        assert!(matches!(err, MacacaError::Task(_)));
    }

    #[tokio::test]
    async fn invalid_transition_start_from_pending() {
        let tracker = TaskTracker::new();
        let task = tracker.create(make_request("t")).await.unwrap();
        let err = tracker.start(&task.id).await.unwrap_err();
        assert!(matches!(err, MacacaError::Task(_)));
    }

    #[tokio::test]
    async fn invalid_transition_complete_from_assigned() {
        let tracker = TaskTracker::new();
        let task = tracker.create(make_request("t")).await.unwrap();
        let agent = AgentId::new();
        tracker.assign(&task.id, agent).await.unwrap();
        let result = make_result(task.id);
        let err = tracker.complete(&task.id, result).await.unwrap_err();
        assert!(matches!(err, MacacaError::Task(_)));
    }

    #[tokio::test]
    async fn invalid_transition_fail_from_completed() {
        let tracker = TaskTracker::new();
        let task = tracker.create(make_request("t")).await.unwrap();
        let agent = AgentId::new();
        tracker.assign(&task.id, agent).await.unwrap();
        tracker.start(&task.id).await.unwrap();
        let result = make_result(task.id);
        tracker.complete(&task.id, result).await.unwrap();
        let err = tracker.fail(&task.id, "late".into()).await.unwrap_err();
        assert!(matches!(err, MacacaError::Task(_)));
    }

    #[tokio::test]
    async fn list_by_status() {
        let tracker = TaskTracker::new();
        let t1 = tracker.create(make_request("a")).await.unwrap();
        let t2 = tracker.create(make_request("b")).await.unwrap();
        let agent = AgentId::new();
        tracker.assign(&t1.id, agent).await.unwrap();

        let pending = tracker.list_by_status(TaskStatus::Pending).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, t2.id);

        let assigned = tracker.list_by_status(TaskStatus::Assigned).await.unwrap();
        assert_eq!(assigned.len(), 1);
        assert_eq!(assigned[0].id, t1.id);
    }

    #[tokio::test]
    async fn list_by_agent() {
        let tracker = TaskTracker::new();
        let t1 = tracker.create(make_request("a")).await.unwrap();
        let t2 = tracker.create(make_request("b")).await.unwrap();
        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        tracker.assign(&t1.id, agent1).await.unwrap();
        tracker.assign(&t2.id, agent2).await.unwrap();

        let tasks = tracker.list_by_agent(&agent1).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, t1.id);
    }

    #[tokio::test]
    async fn status_missing_task_returns_not_found() {
        let tracker = TaskTracker::new();
        let err = tracker.status(&TaskId::new()).await.unwrap_err();
        assert!(matches!(err, MacacaError::NotFound(_)));
    }
}
