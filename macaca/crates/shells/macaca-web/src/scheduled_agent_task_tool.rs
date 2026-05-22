//! Service-backed scheduled-agent-task creation tool.
//!
//! Entry agents may help users define recurring work, but Web must stay a
//! shell adapter.  This module therefore implements a small Tool adapter that
//! converts schema-validated arguments into `CreateScheduledAgentTaskCommand`
//! and forwards the command to `service.scheduled_agent_task` through the
//! focused SDK client.  It does not persist prompts, register Scheduler jobs,
//! execute agents, or branch on application-specific workflows.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationId, AutonomyScope, CreateScheduledAgentTaskCommand, MacacaError, MacacaResult,
    ScheduledAgentTaskSchedule, TraceContext,
};
use macaca_sdk::SystemScheduledAgentTaskClient;
use macaca_tools::Tool;
use serde::Deserialize;
use serde_json::Value;

/// Generic tool name exposed to eligible entry agents.
pub const SCHEDULED_AGENT_TASK_CREATE_TOOL_NAME: &str = "scheduled_agent_task.create";

#[derive(Debug, Deserialize)]
struct ScheduledAgentTaskToolInput {
    target_agent: String,
    task_prompt: String,
    #[serde(default)]
    schedule: ScheduledAgentTaskToolSchedule,
    #[serde(default)]
    metadata: BTreeMap<String, String>,
    #[serde(default)]
    delegated_context: Value,
}

#[derive(Debug, Deserialize)]
struct ScheduledAgentTaskToolSchedule {
    interval_secs: Option<u64>,
}

impl Default for ScheduledAgentTaskToolSchedule {
    fn default() -> Self {
        Self {
            interval_secs: Some(60),
        }
    }
}

/// Tool adapter installed into entry-agent toolkits.
pub struct ScheduledAgentTaskCreateTool {
    client: Arc<dyn SystemScheduledAgentTaskClient>,
    application_id: ApplicationId,
    agent_name: String,
}

impl ScheduledAgentTaskCreateTool {
    /// Build a tool bound to one application and calling agent.
    pub fn new(
        client: Arc<dyn SystemScheduledAgentTaskClient>,
        application_id: ApplicationId,
        agent_name: impl Into<String>,
    ) -> Self {
        Self {
            client,
            application_id,
            agent_name: agent_name.into(),
        }
    }

    fn command_from_input(
        &self,
        input: ScheduledAgentTaskToolInput,
    ) -> MacacaResult<CreateScheduledAgentTaskCommand> {
        let interval_secs = input.schedule.interval_secs.unwrap_or(60).max(1);
        let mut metadata = sanitize_metadata(input.metadata);
        metadata.insert(
            "created_by_tool".into(),
            SCHEDULED_AGENT_TASK_CREATE_TOOL_NAME.into(),
        );
        metadata.insert("requesting_agent".into(), self.agent_name.clone());
        let command = CreateScheduledAgentTaskCommand::new(
            TraceContext::new(format!(
                "entry-agent-scheduled-task-create-{}",
                self.application_id.0
            )),
            AutonomyScope::application(self.application_id),
            ScheduledAgentTaskSchedule::Every {
                interval_ms: interval_secs * 1_000,
            },
            input.target_agent,
            input.task_prompt,
        )?
        .with_delegated_context(input.delegated_context)
        .with_metadata(metadata);
        Ok(command)
    }
}

#[async_trait]
impl Tool for ScheduledAgentTaskCreateTool {
    fn name(&self) -> &str {
        SCHEDULED_AGENT_TASK_CREATE_TOOL_NAME
    }

    fn description(&self) -> &str {
        "Create a generic recurring scheduled agent task through service.scheduled_agent_task."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "required": ["target_agent", "task_prompt"],
            "properties": {
                "target_agent": {
                    "type": "string",
                    "description": "Provider-neutral agent name that should execute the recurring task."
                },
                "task_prompt": {
                    "type": "string",
                    "description": "Task prompt admitted only by the Scheduled Agent Task service."
                },
                "schedule": {
                    "type": "object",
                    "properties": {
                        "interval_secs": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Recurring interval in seconds."
                        }
                    }
                },
                "metadata": {
                    "type": "object",
                    "additionalProperties": { "type": "string" }
                },
                "delegated_context": {
                    "type": "object"
                }
            }
        })
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let input: ScheduledAgentTaskToolInput = serde_json::from_value(input)
            .map_err(|error| MacacaError::Config(error.to_string()))?;
        tracing::info!(
            app_id = %self.application_id,
            requesting_agent = self.agent_name.as_str(),
            target_agent = input.target_agent.as_str(),
            "entry agent requested scheduled task creation"
        );
        let command = self.command_from_input(input)?;
        tracing::info!(
            app_id = %self.application_id,
            requesting_agent = self.agent_name.as_str(),
            trace_id = %command.trace.trace_id,
            "entry agent scheduled task command admitted"
        );
        let result = self.client.create_task(command).await?;
        tracing::info!(
            app_id = %self.application_id,
            requesting_agent = self.agent_name.as_str(),
            accepted = result.accepted,
            task_id = result.task_id.as_ref().map(|id| id.as_str()).unwrap_or("none"),
            audit_id = result.audit_id.as_deref().unwrap_or("none"),
            "entry agent scheduled task command completed"
        );
        serde_json::to_value(result).map_err(|error| MacacaError::Config(error.to_string()))
    }
}

fn sanitize_metadata(metadata: BTreeMap<String, String>) -> BTreeMap<String, String> {
    metadata
        .into_iter()
        .filter(|(key, _)| {
            let lowered = key.to_ascii_lowercase();
            !lowered.contains("prompt")
                && !lowered.contains("secret")
                && !lowered.contains("token")
                && !lowered.contains("credential")
                && !lowered.contains("private_key")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use macaca_proto::{
        CancelScheduledAgentTaskCommand, ResolveScheduledAgentTaskPayloadCommand,
        ScheduledAgentTaskCommandResult, ScheduledAgentTaskQueryCommand,
        ScheduledAgentTaskResolvedPayload, ScheduledAgentTaskServiceSnapshot,
        ScheduledAgentTaskSummary,
    };

    #[derive(Default)]
    struct RecordingScheduledAgentTaskClient {
        commands: std::sync::Mutex<Vec<CreateScheduledAgentTaskCommand>>,
    }

    #[async_trait]
    impl SystemScheduledAgentTaskClient for RecordingScheduledAgentTaskClient {
        async fn create_task(
            &self,
            command: CreateScheduledAgentTaskCommand,
        ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
            self.commands.lock().unwrap().push(command.clone());
            Ok(ScheduledAgentTaskCommandResult {
                accepted: true,
                task_id: None,
                scheduler_job_id: None,
                scheduler_run_id: None,
                payload_ref: None,
                payload_digest: None,
                trace: command.trace,
                audit_id: Some("audit.tool.test".into()),
                summary: None,
                error: None,
                metadata: BTreeMap::new(),
            })
        }

        async fn get_task(
            &self,
            _command: ScheduledAgentTaskQueryCommand,
        ) -> MacacaResult<Option<ScheduledAgentTaskSummary>> {
            Ok(None)
        }

        async fn list_tasks(
            &self,
            _command: ScheduledAgentTaskQueryCommand,
        ) -> MacacaResult<Vec<ScheduledAgentTaskSummary>> {
            Ok(Vec::new())
        }

        async fn cancel_task(
            &self,
            _command: CancelScheduledAgentTaskCommand,
        ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
            Err(MacacaError::Config("not used in this test".into()))
        }

        async fn resolve_payload(
            &self,
            _command: ResolveScheduledAgentTaskPayloadCommand,
        ) -> MacacaResult<Option<ScheduledAgentTaskResolvedPayload>> {
            Ok(None)
        }

        async fn health(
            &self,
            _trace: TraceContext,
        ) -> MacacaResult<ScheduledAgentTaskServiceSnapshot> {
            Ok(ScheduledAgentTaskServiceSnapshot::unavailable(
                "not used in this test",
            ))
        }
    }

    #[tokio::test]
    async fn tool_invocation_builds_the_same_service_create_shape_as_manual_entry() {
        let client = Arc::new(RecordingScheduledAgentTaskClient::default());
        let tool = ScheduledAgentTaskCreateTool::new(
            client.clone(),
            ApplicationId::from_name("scheduled-agent-tool-test"),
            "entry",
        );

        let output = tool
            .execute(serde_json::json!({
                "target_agent": "worker",
                "task_prompt": "Run a bounded recurring analysis.",
                "schedule": { "interval_secs": 120 },
                "metadata": { "schedule.name": "Recurring analysis" },
                "delegated_context": { "source": "entry-agent" }
            }))
            .await
            .unwrap();

        assert_eq!(output["accepted"], true);
        let commands = client.commands.lock().unwrap();
        assert_eq!(commands.len(), 1);
        let command = &commands[0];
        assert_eq!(command.target_agent, "worker");
        assert_eq!(command.user_prompt, "Run a bounded recurring analysis.");
        assert_eq!(command.metadata["schedule.name"], "Recurring analysis");
        assert_eq!(
            command.metadata["created_by_tool"],
            SCHEDULED_AGENT_TASK_CREATE_TOOL_NAME
        );
        assert_eq!(command.delegated_context["source"], "entry-agent");
        assert!(matches!(
            command.schedule,
            ScheduledAgentTaskSchedule::Every {
                interval_ms: 120_000
            }
        ));
    }
}
