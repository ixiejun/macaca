//! Snapshot builder and Memento cache refresh.

use macaca_proto::{ApplicationId, TodoStatus};

use tracing::info;

use crate::commands::TaskServiceSnapshotCommand;
use crate::events::{
    TaskServiceEvent, TaskServiceEventType, TaskServiceGoalSnapshot, TaskServiceSnapshot,
    TaskServiceTaskSnapshot,
};

use super::TaskServiceRuntime;

impl<S> TaskServiceRuntime<S>
where
    S: super::TaskServiceExecutionStrategy + 'static,
{
    pub async fn snapshot(
        &self,
        command: TaskServiceSnapshotCommand,
    ) -> Result<TaskServiceSnapshot, String> {
        let snapshot = self
            .build_snapshot(&command.app_id, command.session_id.as_deref())
            .await;
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            command.session_id.clone(),
            None,
            None,
            TaskServiceEventType::TaskSnapshotEmitted,
            command.trace.clone(),
            serde_json::json!({
                "goals": snapshot.goals.len(),
                "tasks": snapshot.tasks.len(),
            }),
        ))
        .await;
        Ok(snapshot)
    }

    pub(crate) async fn refresh_snapshot(&self, app_id: &ApplicationId, session_id: Option<&str>) {
        let snapshot = self.build_snapshot(app_id, session_id).await;
        let authoritative_tasks = snapshot
            .tasks
            .iter()
            .filter(|task| task.graph_owner.is_application_execution_authoritative())
            .collect::<Vec<_>>();
        let authoritative_completed = authoritative_tasks
            .iter()
            .filter(|task| task.status == TodoStatus::Completed)
            .count();
        let authoritative_failed = authoritative_tasks
            .iter()
            .filter(|task| task.status == TodoStatus::Failed)
            .count();
        let authoritative_blocked = authoritative_tasks
            .iter()
            .filter(|task| task.status == TodoStatus::Blocked)
            .count();
        info!(
            app_id = %app_id.0,
            session_id = session_id.unwrap_or("none"),
            authoritative_tasks = authoritative_tasks.len(),
            authoritative_completed,
            authoritative_failed,
            authoritative_blocked,
            "task graph terminal projected"
        );
        let mut snapshots = self.snapshots.write().unwrap();
        snapshots.insert(
            (app_id.to_string(), session_id.map(str::to_string)),
            snapshot,
        );
    }

    pub(crate) async fn build_snapshot(
        &self,
        app_id: &ApplicationId,
        session_id: Option<&str>,
    ) -> TaskServiceSnapshot {
        let mut goals = match session_id {
            Some(session_id) => self.store.list_goals_for_session(app_id, session_id).await,
            None => self.store.list_goals(app_id).await,
        }
        .into_iter()
        .map(|goal| TaskServiceGoalSnapshot {
            goal_id: goal.id,
            description: goal.description,
            status: goal.status,
            session_id: goal.session_id,
        })
        .collect::<Vec<_>>();
        goals.sort_by(|left, right| left.goal_id.0.cmp(&right.goal_id.0));

        let mut tasks = match session_id {
            Some(session_id) => {
                self.store
                    .list_all_todos_for_session(app_id, session_id)
                    .await
            }
            None => self.store.list_all_todos(app_id).await,
        }
        .into_iter()
        .map(|task| TaskServiceTaskSnapshot {
            task_id: task.id,
            title: task.title,
            agent_name: task.assigned_agent,
            status: task.status,
            session_id: task.session_id,
            graph_owner: task.graph_owner,
            graph_id: task.graph_id,
        })
        .collect::<Vec<_>>();
        tasks.sort_by(|left, right| left.task_id.0.cmp(&right.task_id.0));

        TaskServiceSnapshot {
            app_id: app_id.clone(),
            session_id: session_id.map(str::to_string),
            goals,
            tasks,
        }
    }
}
