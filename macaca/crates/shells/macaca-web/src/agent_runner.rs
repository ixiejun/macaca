//! ServiceRuntime-backed AgentRunner implementation for Macaca Web.
//!
//! This adapter keeps the kernel/executor-facing `AgentRunner` trait stable
//! while routing YAML workflow and application-executor work through
//! `service.agent_execution` instead of allowing a separate shell-owned agent
//! runtime path.

use std::sync::{Arc, Weak};

use async_trait::async_trait;
use macaca_kernel::{AgentInfo, AgentRunner, TaskContext, TaskResult};
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionEvent, AgentExecutionIntent, AgentExecutionResult,
    AgentExecutionStatus, ApplicationId, KernelServiceId, ServiceBusSource, TaskId, TraceContext,
    AGENT_EXECUTION_SERVICE_ID,
};
use tracing::info;

use crate::state::AppState;

/// Web-based agent runner backed by framework `ReActAgent` builders.
pub struct WebAgentRunner {
    /// Weak reference to the shared application state to avoid cycles.
    state: Weak<AppState>,
}

impl WebAgentRunner {
    /// Create a new WebAgentRunner.
    pub fn new(state: Weak<AppState>) -> Self {
        Self { state }
    }

    /// Get a strong reference to the state.
    /// Returns an error if the state has been dropped.
    fn get_state(&self) -> Result<Arc<AppState>, String> {
        self.state
            .upgrade()
            .ok_or_else(|| "AppState has been dropped".to_string())
    }

    async fn execute_via_agent_service(
        &self,
        application_id: &ApplicationId,
        agent_name: &str,
        prompt: &str,
        context: Option<TaskContext>,
        event_tx: Option<tokio::sync::mpsc::Sender<AgentExecutionEvent>>,
    ) -> Result<TaskResult, String> {
        let state = self.get_state()?;

        info!(
            application_id = %application_id.0,
            agent = agent_name,
            prompt_preview = %prompt.chars().take(80).collect::<String>(),
            has_event_tx = event_tx.is_some(),
            "Executing agent through agent execution service"
        );

        let session_id = context.as_ref().and_then(|c| c.session_id.clone());
        let task_id = TaskId::new();

        if let Some(agent_manifest) = state.kernel.get_agent_by_name(agent_name).await {
            state
                .kernel
                .status_tracker()
                .set_working(
                    &agent_manifest.id,
                    &format!("Executing: {}", prompt.chars().take(50).collect::<String>()),
                )
                .await;
        }

        let resolved_session_id =
            session_id.unwrap_or_else(|| format!("agent-runner:{}", task_id.0));
        let mut trace = TraceContext::new(format!(
            "agent-runner:{}:{}:{}",
            application_id.0, agent_name, task_id.0
        ));
        trace.session_id = Some(resolved_session_id.clone());
        trace.task_id = Some(task_id.0.to_string());
        trace.agent = Some(agent_name.to_string());

        let delegated_context = context
            .as_ref()
            .map(|ctx| {
                serde_json::json!({
                    "artifacts": ctx.artifacts.clone(),
                    "env": ctx.env.clone(),
                })
            })
            .unwrap_or_else(|| serde_json::json!({}));

        let mut command = AgentExecutionCommand::new(
            *application_id,
            resolved_session_id.clone(),
            agent_name,
            AgentExecutionIntent::YamlWorkflowStep,
            prompt,
            trace,
        )
        .map_err(|error| error.to_string())?
        .with_delegated_context(delegated_context);
        command.task_id = Some(task_id);
        command
            .metadata
            .insert("entrypoint".into(), "kernel.agent_runner".into());
        command
            .metadata
            .insert("suppress_executor_lifecycle".into(), "true".into());

        let service_command = command
            .into_service_command()
            .map_err(|error| error.to_string())?;
        let reply = state
            .service_runtime
            .call(
                &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
                ServiceBusSource::new("macaca.web.agent_runner"),
                service_command,
            )
            .await
            .map_err(|error| error.to_string())?;

        let output = reply
            .output
            .ok_or_else(|| "agent execution service returned no output".to_string())?;
        let result: AgentExecutionResult =
            serde_json::from_value(output).map_err(|error| error.to_string())?;

        match result.status {
            AgentExecutionStatus::Completed => {
                if let Some(agent_manifest) = state.kernel.get_agent_by_name(agent_name).await {
                    state
                        .kernel
                        .status_tracker()
                        .set_idle(&agent_manifest.id)
                        .await;
                }

                let output = result
                    .output
                    .get("output")
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| result.output.to_string());
                Ok(TaskResult {
                    task_id: result.task_id.unwrap_or(task_id),
                    success: !output.is_empty(),
                    output,
                    error: None,
                    artifacts: vec![],
                    completed_at: chrono::Utc::now(),
                    tokens_used: None,
                })
            }
            status => {
                let error = result
                    .output
                    .get("error")
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| {
                        format!("agent execution service returned {}", status.as_str())
                    });
                if let Some(agent_manifest) = state.kernel.get_agent_by_name(agent_name).await {
                    state
                        .kernel
                        .status_tracker()
                        .set_error(&agent_manifest.id, &error)
                        .await;
                }
                Err(error)
            }
        }
    }
}

#[async_trait]
impl AgentRunner for WebAgentRunner {
    async fn execute_agent(
        &self,
        application_id: &ApplicationId,
        agent_name: &str,
        prompt: &str,
        context: Option<TaskContext>,
    ) -> Result<TaskResult, String> {
        self.execute_via_agent_service(application_id, agent_name, prompt, context, None)
            .await
    }

    async fn execute_agent_with_events(
        &self,
        application_id: &ApplicationId,
        agent_name: &str,
        prompt: &str,
        context: Option<TaskContext>,
        event_tx: Option<tokio::sync::mpsc::Sender<AgentExecutionEvent>>,
    ) -> Result<TaskResult, String> {
        self.execute_via_agent_service(application_id, agent_name, prompt, context, event_tx)
            .await
    }

    async fn list_agents(&self) -> Vec<AgentInfo> {
        let state = match self.get_state() {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let manifests = state.kernel.list_agents().await;
        manifests
            .into_iter()
            .map(|m| AgentInfo {
                id: m.id.0.to_string(),
                name: m.name,
                capabilities: m.capabilities.into_iter().map(|c| c.name).collect(),
                current_load: 0,
                max_load: 4,
                available: true,
            })
            .collect()
    }

    async fn agent_exists(&self, agent_name: &str) -> bool {
        let state = match self.get_state() {
            Ok(s) => s,
            Err(_) => return false,
        };

        let manifests = state.kernel.list_agents().await;
        manifests.iter().any(|m| m.name == agent_name)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn web_agent_runner_enters_agent_execution_service_boundary() {
        let source = include_str!("agent_runner.rs");
        let legacy_runtime_builder = ["FrameworkRunner::build_runtime", "_agent("].concat();

        assert!(source.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(source.contains("AgentExecutionCommand::new"));
        assert!(source.contains("AgentExecutionIntent::YamlWorkflowStep"));
        assert!(source.contains("ServiceBusSource::new(\"macaca.web.agent_runner\")"));
        assert!(!source.contains(&legacy_runtime_builder));
    }

    #[test]
    fn yaml_and_wasm_entries_share_agent_execution_trace_schema() {
        let yaml_entry = include_str!("agent_runner.rs");
        let wasm_entry = include_str!("wasm_orchestration_backend.rs");
        let provider = include_str!(
            "../../../runtime/macaca-runtime-host/src/agent_execution_service_provider.rs"
        );

        assert!(yaml_entry.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(wasm_entry.contains("AGENT_EXECUTION_SERVICE_ID"));
        assert!(yaml_entry.contains("AgentExecutionCommand::new"));
        assert!(wasm_entry.contains("AgentExecutionCommand::new"));
        assert!(provider.contains("trace.system_service.agent_execution.v1"));
    }
}
