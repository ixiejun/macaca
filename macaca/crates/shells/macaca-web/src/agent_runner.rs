//! ServiceRuntime-backed AgentRunner implementation for Macaca Web.
//!
//! This adapter keeps the kernel/executor-facing `AgentRunner` trait stable
//! while routing YAML workflow steps through the Application Service ABI
//! (`application.agent.delegate`) before the shared orchestration bridge
//! forwards work to `service.agent_execution`.  The executor therefore
//! remains a scheduling shell; execution semantics stay serviceized and auditable.

use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use macaca_runtime_host::{AgentInfo, AgentRunner, TaskContext, TaskResult};
use macaca_proto::{
    AgentExecutionEvent, AgentExecutionIntent, ApplicationAgentDelegateCommand,
    ApplicationAgentDelegateResult, ApplicationId, ApplicationServiceScope, KernelServiceId,
    ServiceBusSource, TaskId, TraceContext, APPLICATION_SERVICE_ID,
    AGENT_EXECUTION_INTENT_METADATA_KEY,
};
use tracing::info;

use crate::state::AppState;

/// Web-based agent runner backed by the unified Application Service path.
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

    /// Execute one workflow step through the Application Service delegation ABI.
    ///
    /// Flow:
    /// 1. Build a traced `ApplicationAgentDelegateCommand` with YAML workflow intent.
    /// 2. Call `service.application` / `application.agent.delegate`.
    /// 3. Application provider forwards to `WebApplicationOrchestrationBackend`.
    /// 4. Shared bridge dispatches `service.agent_execution` with `YamlWorkflowStep`.
    ///
    /// This matches the WASM host import path so audit replay can converge both
    /// package kinds on one Application ABI entry before agent execution.
    async fn execute_via_application_delegate(
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
            service_id = APPLICATION_SERVICE_ID,
            "Executing workflow step through application agent delegation service"
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

        let scope = ApplicationServiceScope::session(*application_id, resolved_session_id.clone())
            .map_err(|error| error.to_string())?;

        let mut metadata = BTreeMap::from([
            (
                "entrypoint".into(),
                "kernel.agent_runner".into(),
            ),
            (
                AGENT_EXECUTION_INTENT_METADATA_KEY.into(),
                AgentExecutionIntent::YamlWorkflowStep
                    .metadata_value()
                    .into(),
            ),
        ]);
        if event_tx.is_some() {
            metadata.insert("stream_agent_events".into(), "true".into());
        }

        let delegate_command = ApplicationAgentDelegateCommand {
            trace: trace.clone(),
            scope,
            target_agent: agent_name.to_string(),
            prompt: prompt.to_string(),
            context: delegated_context,
            metadata,
        };

        let service_command = delegate_command
            .into_service_command()
            .map_err(|error| error.to_string())?;

        let reply = state
            .service_runtime
            .call(
                &KernelServiceId::new(APPLICATION_SERVICE_ID),
                ServiceBusSource::new("macaca.web.agent_runner"),
                service_command,
            )
            .await
            .map_err(|error| error.to_string())?;

        let output = reply
            .output
            .ok_or_else(|| "application agent delegation returned no output".to_string())?;
        let result: ApplicationAgentDelegateResult =
            serde_json::from_value(output).map_err(|error| error.to_string())?;

        if result.success {
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
                task_id: result
                    .task_id
                    .as_deref()
                    .and_then(|value| uuid::Uuid::parse_str(value).ok())
                    .map(TaskId)
                    .unwrap_or_else(|| task_id),
                success: !output.is_empty(),
                output,
                error: None,
                artifacts: vec![],
                completed_at: chrono::Utc::now(),
                tokens_used: None,
            })
        } else {
            let error = result
                .output
                .get("error")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("application agent delegation returned {}", result.status));
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

#[async_trait]
impl AgentRunner for WebAgentRunner {
    async fn execute_agent(
        &self,
        application_id: &ApplicationId,
        agent_name: &str,
        prompt: &str,
        context: Option<TaskContext>,
    ) -> Result<TaskResult, String> {
        self.execute_via_application_delegate(application_id, agent_name, prompt, context, None)
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
        self.execute_via_application_delegate(
            application_id,
            agent_name,
            prompt,
            context,
            event_tx,
        )
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

