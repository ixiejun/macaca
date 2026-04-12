//! Framework-native AgentRunner implementation for Macaca Web.
//!
//! This adapter keeps the kernel/executor-facing `AgentRunner` trait stable
//! while routing business execution through `macaca-framework` instead of the
//! legacy `AgenticLoop`.

use std::sync::{Arc, Weak};

use async_trait::async_trait;
use macaca_framework::agent::Agent;
use macaca_framework::message::Msg;
use macaca_kernel::{AgentInfo, AgentRunner, TaskContext, TaskId, TaskResult};
use macaca_proto::{AgentExecutionEvent, ApplicationId};
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

    fn compose_prompt(prompt: &str, context: Option<&TaskContext>) -> String {
        if let Some(ctx) = context {
            if !ctx.artifacts.is_empty() {
                return format!(
                    "Context artifacts available:\n{}\n\nTask:\n{}",
                    ctx.artifacts.join("\n"),
                    prompt
                );
            }
        }
        prompt.to_string()
    }

    async fn execute_via_framework(
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
            "Executing agent through framework runner"
        );

        let session_id = context.as_ref().and_then(|c| c.session_id.clone());
        let user_prompt = Self::compose_prompt(prompt, context.as_ref());

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

        let agent = crate::framework_runner::FrameworkRunner::build_runtime_agent(
            &state,
            application_id,
            agent_name,
            session_id,
            None,
            event_tx,
        )
        .await;

        let agent = match agent {
            Ok(agent) => agent,
            Err(e) => {
                if let Some(agent_manifest) = state.kernel.get_agent_by_name(agent_name).await {
                    state
                        .kernel
                        .status_tracker()
                        .set_error(&agent_manifest.id, &e)
                        .await;
                }
                return Err(format!("Failed to build framework agent: {e}"));
            }
        };

        let result = agent.reply(Msg::user("user", user_prompt)).await;

        match result {
            Ok(reply) => {
                if let Some(agent_manifest) = state.kernel.get_agent_by_name(agent_name).await {
                    state
                        .kernel
                        .status_tracker()
                        .set_idle(&agent_manifest.id)
                        .await;
                }

                let output = reply.get_text();
                Ok(TaskResult {
                    task_id: TaskId::new(),
                    success: !output.is_empty(),
                    output,
                    error: None,
                    artifacts: vec![],
                    completed_at: chrono::Utc::now(),
                    tokens_used: None,
                })
            }
            Err(e) => {
                let error = e.to_string();
                if let Some(agent_manifest) = state.kernel.get_agent_by_name(agent_name).await {
                    state
                        .kernel
                        .status_tracker()
                        .set_error(&agent_manifest.id, &error)
                        .await;
                }
                Err(format!("Framework agent execution failed: {error}"))
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
        self.execute_via_framework(application_id, agent_name, prompt, context, None)
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
        self.execute_via_framework(application_id, agent_name, prompt, context, event_tx)
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
