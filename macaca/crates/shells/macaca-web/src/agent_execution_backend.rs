//! Web composition backend for `service.agent_execution`.
//!
//! This module is the host-owned adapter that turns one provider-neutral
//! execution command into a framework-native agent run.  It first obtains a
//! trusted context snapshot through `service.agent_context`, then broadcasts
//! task lifecycle and agent trace events through the existing application
//! executor event channel so delegated WASM runs appear in the same UI traces
//! as YAML and executor-originated work.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_framework::agent::Agent;
use macaca_framework::message::Msg;
use macaca_kernel::executor::{ExecutorEvent, ExecutorEventFactory};
use macaca_proto::{
    AgentContextBuildCommand, AgentContextSnapshot, AgentExecutionCommand, AgentExecutionResult,
    AgentExecutionStatus, KernelServiceId, ServiceBusSource, ServiceError, ServiceResult, TaskId,
    AGENT_CONTEXT_SERVICE_ID,
};
use macaca_runtime_host::{AgentExecutionBackend, ServiceRuntime};
use tokio::sync::mpsc;

use crate::framework_runner::FrameworkRunner;
use crate::state::AppState;

/// Web-owned implementation of the Agent Execution system service.
pub(crate) struct WebAgentExecutionBackend {
    state: Arc<AppState>,
    service_runtime: Arc<ServiceRuntime>,
}

impl WebAgentExecutionBackend {
    /// Create a backend with access to state and the shared ServiceRuntime.
    pub(crate) fn new(state: Arc<AppState>, service_runtime: Arc<ServiceRuntime>) -> Self {
        Self {
            state,
            service_runtime,
        }
    }

    async fn build_context_snapshot(
        &self,
        command: &AgentExecutionCommand,
    ) -> ServiceResult<AgentContextSnapshot> {
        let context_command = AgentContextBuildCommand::from_execution(command);
        let reply = self
            .service_runtime
            .call(
                &KernelServiceId::new(AGENT_CONTEXT_SERVICE_ID),
                ServiceBusSource::new("macaca.web.agent_execution"),
                context_command
                    .into_service_command()
                    .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            )
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        let output = reply.output.ok_or_else(|| {
            ServiceError::AdapterFailure("agent context service returned no output".into())
        })?;
        serde_json::from_value(output)
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))
    }

    fn user_prompt_with_context(command: &AgentExecutionCommand) -> String {
        if command.delegated_context.is_null() || command.delegated_context == serde_json::json!({})
        {
            return command.user_prompt.clone();
        }
        let rendered_context = serde_json::to_string_pretty(&command.delegated_context)
            .unwrap_or_else(|_| command.delegated_context.to_string());
        format!(
            "{}\n\nStructured evidence context for this delegated task:\n```json\n{}\n```",
            command.user_prompt, rendered_context
        )
    }

    /// Some legacy kernel/executor callers already emit task lifecycle events
    /// around the `AgentRunner` trait.  They still need the Agent Execution
    /// service for context/model/tool semantics, but duplicate started/completed
    /// events would make session traces noisy.  The suppression flag only
    /// affects coarse lifecycle events; fine-grained agent events are still
    /// forwarded from framework hooks.
    fn should_emit_executor_lifecycle(command: &AgentExecutionCommand) -> bool {
        command
            .metadata
            .get("suppress_executor_lifecycle")
            .map(|value| value != "true")
            .unwrap_or(true)
    }

    fn failed_result(
        command: &AgentExecutionCommand,
        task_id: TaskId,
        error: impl Into<String>,
        context_snapshot: Option<AgentContextSnapshot>,
    ) -> AgentExecutionResult {
        let error = error.into();
        AgentExecutionResult {
            application_id: command.application_id,
            session_id: command.session_id.clone(),
            task_id: Some(task_id),
            target_agent: command.target_agent.clone(),
            status: AgentExecutionStatus::Failed,
            output: serde_json::json!({ "error": error }),
            context_snapshot,
            trace: command.trace.clone(),
            metadata: Default::default(),
        }
    }
}

#[async_trait]
impl AgentExecutionBackend for WebAgentExecutionBackend {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
        let task_id = command.task_id.unwrap_or_else(TaskId::new);
        let context_snapshot = self.build_context_snapshot(&command).await?;
        let executor = self
            .state
            .executor_registry
            .get(&command.application_id)
            .await;
        let event_factory = ExecutorEventFactory::new(task_id, command.target_agent.clone());
        let emit_lifecycle = Self::should_emit_executor_lifecycle(&command);

        if emit_lifecycle {
            if let Some(executor) = executor.as_ref() {
                executor.broadcast_event(event_factory.started());
            }
        }

        let agent_event_tx = if let Some(executor) = executor.clone() {
            let (agent_event_tx, mut agent_event_rx) = mpsc::channel(64);
            let agent = command.target_agent.clone();
            tokio::spawn(async move {
                while let Some(event) = agent_event_rx.recv().await {
                    executor.broadcast_event(ExecutorEvent::AgentEvent {
                        task_id,
                        agent: agent.clone(),
                        event,
                    });
                }
            });
            Some(agent_event_tx)
        } else {
            None
        };

        let agent = FrameworkRunner::build_runtime_agent_from_context_snapshot(
            &self.state,
            &context_snapshot,
            agent_event_tx,
        )
        .await
        .map_err(ServiceError::AdapterFailure)?;

        let prompt = Self::user_prompt_with_context(&command);
        match agent.reply(Msg::user("user", prompt)).await {
            Ok(reply) => {
                let output = reply.get_text();
                let task_result = event_factory.success_result(output.clone());
                if emit_lifecycle {
                    if let Some(executor) = executor.as_ref() {
                        executor.broadcast_event(event_factory.completed_with_result(task_result));
                    }
                }
                let mut result = AgentExecutionResult::completed(
                    &AgentExecutionCommand {
                        task_id: Some(task_id),
                        ..command
                    },
                    serde_json::json!({ "output": output }),
                );
                result.context_snapshot = Some(context_snapshot);
                Ok(result)
            }
            Err(error) => {
                let error = error.to_string();
                if emit_lifecycle {
                    if let Some(executor) = executor.as_ref() {
                        executor.broadcast_event(event_factory.failed(error.clone()));
                    }
                }
                Ok(Self::failed_result(
                    &command,
                    task_id,
                    error,
                    Some(context_snapshot),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn execution_backend_consumes_context_snapshot_without_rebuilding_context() {
        let source = include_str!("agent_execution_backend.rs");
        let service_context_call = "build_context_snapshot";
        let snapshot_runtime_builder = "build_runtime_agent_from_context_snapshot";
        let legacy_runtime_builder = ["FrameworkRunner::build_runtime", "_agent("].concat();

        assert!(source.contains(service_context_call));
        assert!(source.contains(snapshot_runtime_builder));
        assert!(!source.contains(&legacy_runtime_builder));
    }

    #[test]
    fn execution_backend_returns_context_snapshot_for_audit_replay() {
        let source = include_str!("agent_execution_backend.rs");

        assert!(source.contains("result.context_snapshot = Some(context_snapshot)"));
        assert!(source.contains("AgentExecutionResult::completed"));
        assert!(source.contains("AgentExecutionStatus::Failed"));
        assert!(source.contains("ServiceBusSource::new(\"macaca.web.agent_execution\")"));
    }
}
