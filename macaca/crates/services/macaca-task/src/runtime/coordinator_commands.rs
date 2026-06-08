//! Coordinator resume command handler.

use crate::commands::ResumeCoordinatorCommand;
use crate::events::{TaskServiceEvent, TaskServiceEventType};

use super::TaskServiceRuntime;

impl<S> TaskServiceRuntime<S>
where
    S: super::TaskServiceExecutionStrategy + 'static,
{
    pub async fn resume_coordinator(
        &self,
        command: ResumeCoordinatorCommand,
    ) -> Result<(), String> {
        self.execution.resume_coordinator(&command).await?;
        self.emit(TaskServiceEvent::new(
            command.app_id.clone(),
            command.session_id.clone(),
            command.goal_id,
            command.goal_id,
            TaskServiceEventType::CoordinatorResumeRequested,
            command.trace.clone(),
            serde_json::json!({
                "reason": command.reason,
            }),
        ))
        .await;
        self.refresh_snapshot(&command.app_id, command.session_id.as_deref())
            .await;
        Ok(())
    }
}
