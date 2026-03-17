//! Agent Runner implementation for Macaca Web.
//!
//! This module provides a concrete implementation of the AgentRunner trait
//! that connects the executor system to the actual agent execution infrastructure.

use std::sync::{Arc, Weak};
use std::path::PathBuf;

use async_trait::async_trait;
use macaca_kernel::{AgentInfo, AgentRunner, TaskContext, TaskId, TaskResult, TokenUsage};
use macaca_proto::{AgentId, ApplicationId, LlmMessage, LlmOptions, Permission, ToolDefinition};
use macaca_runtime::{AgenticLoop, RuntimeConfig};
use macaca_sdk::AgentPersona;
use macaca_tools::ToolSet;
use tracing::{error, info, warn};

use crate::state::AppState;

/// Web-based agent runner that executes agents using the LLM provider.
///
/// This runner:
/// 1. Loads the agent's persona from the app directory
/// 2. Builds messages with system prompt and user prompt
/// 3. Calls the LLM with tools available
/// 4. Returns the execution result
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
        self.state.upgrade().ok_or_else(|| "AppState has been dropped".to_string())
    }

    /// Load an agent's persona from the app directory.
    async fn load_persona(app_dir: &PathBuf, agent_name: &str) -> Option<AgentPersona> {
        let persona_dir = app_dir.join("personas").join(agent_name);
        if persona_dir.exists() {
            match AgentPersona::load_from_directory(&persona_dir).await {
                Ok(persona) => {
                    info!(agent = agent_name, "Persona loaded");
                    return Some(persona);
                }
                Err(e) => {
                    warn!(agent = agent_name, error = %e, "Failed to load persona");
                }
            }
        }
        None
    }

    /// Build system prompt for an agent.
    fn build_system_prompt(
        agent_name: &str,
        persona: Option<&AgentPersona>,
        capabilities: &[String],
    ) -> String {
        let base_prompt = if let Some(p) = persona {
            p.to_system_prompt(None)
        } else {
            format!("You are the {} agent in Macaca OS.", agent_name)
        };

        if capabilities.is_empty() {
            base_prompt
        } else {
            format!(
                "{}\n\nYour capabilities: {}",
                base_prompt,
                capabilities.join(", ")
            )
        }
    }
}

#[async_trait]
impl AgentRunner for WebAgentRunner {
    /// Execute an agent with the given prompt.
    ///
    /// This is the core method that actually runs an agent:
    /// 1. Find the app directory for this agent (by application_id)
    /// 2. Load the agent's persona
    /// 3. Build messages with system prompt and user prompt
    /// 4. Call the LLM with available tools
    /// 5. Return the result
    async fn execute_agent(
        &self,
        application_id: &ApplicationId,
        agent_name: &str,
        prompt: &str,
        context: Option<TaskContext>,
    ) -> Result<TaskResult, String> {
        let state = self.get_state()?;

        info!(
            application_id = %application_id.0,
            agent = agent_name,
            prompt_preview = %prompt.chars().take(50).collect::<String>(),
            "Executing agent"
        );

        // Find the app directory for this application_id
        let app_dirs = state.app_dirs.read().await;
        let app_dir = app_dirs
            .iter()
            .find(|(id, _)| **id == *application_id)
            .map(|(_, path)| path.clone())
            .ok_or_else(|| format!("Application '{}' not found", application_id))?;
        drop(app_dirs);

        // Get agent's capabilities from kernel
        let agent_manifests = state.kernel.list_agents().await;
        let agent_info = agent_manifests
            .iter()
            .find(|m| m.name == agent_name);

        let capabilities: Vec<String> = agent_info
            .map(|a| a.capabilities.iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default();

        // Load agent's persona
        let persona = Self::load_persona(&app_dir, agent_name).await;

        // Debug: log persona status
        if persona.is_some() {
            info!(agent = agent_name, "Persona loaded successfully");
        } else {
            warn!(agent = agent_name, app_dir = %app_dir.display(), "No persona found");
        }

        // Build system prompt
        let system_prompt = Self::build_system_prompt(agent_name, persona.as_ref(), &capabilities);
        info!(agent = agent_name, system_prompt_len = system_prompt.len(), "System prompt built");

        // Build messages
        let mut messages = vec![LlmMessage::system(system_prompt)];

        // Add context if provided
        if let Some(ref ctx) = context {
            if !ctx.artifacts.is_empty() {
                let context_msg = format!(
                    "Context artifacts available:\n{}",
                    ctx.artifacts.join("\n")
                );
                messages.push(LlmMessage::user(context_msg));
            }
        }

        // Add the main prompt
        messages.push(LlmMessage::user(prompt.to_string()));

        // Get tool definitions
        let tool_defs = state.tools.to_definitions();

        // Model resolution priority:
        // 1. Agent's preferred model (if non-empty)
        // 2. Application's default model (from llm_config)
        // 3. System default model
        let model = if let Some(manifest) = state.kernel.get_agent_by_name(agent_name).await {
            if !manifest.model.is_empty() {
                // Agent has a specific model preference
                manifest.model
            } else {
                // Check for application-level model configuration
                // For now, use system default (application-level config can be added later)
                state.default_model.clone()
            }
        } else {
            state.default_model.clone()
        };

        // Build LLM options - use resolved model
        let options = LlmOptions {
            model,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            stop_sequences: vec![],
            tools: Some(tool_defs),
        };

        // Use AgenticLoop for proper tool execution
        let loop_config = RuntimeConfig {
            max_iterations: 25,
            tool_timeout: std::time::Duration::from_secs(300), // 5 minutes, matches Claude Code Driver default
        };
        let agentic_loop = AgenticLoop::new(loop_config);

        // Create a dummy agent ID for permission checking
        let agent_id = AgentId::new();
        let permission = Permission {
            level: macaca_proto::PermissionLevel::User,
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: true,
        };

        // Run the agentic loop
        info!(
            application_id = %application_id.0,
            agent = agent_name,
            "Starting agentic loop"
        );

        let loop_result = agentic_loop
            .run(
                &agent_id,
                state.llm.as_ref(),
                state.tools.as_ref(),
                messages,
                &options,
                &permission,
                None, // No permission checker - allow all tools
            )
            .await
            .map_err(|e| format!("Agentic loop failed: {}", e))?;

        info!(
            application_id = %application_id.0,
            agent = agent_name,
            success = !loop_result.content.is_empty(),
            output_len = loop_result.content.len(),
            iterations = loop_result.iterations,
            total_tokens = loop_result.total_usage.total_tokens,
            "Agent execution completed"
        );

        // Debug: if content is empty, log warning
        if loop_result.content.is_empty() {
            error!(
                application_id = %application_id.0,
                agent = agent_name,
                "WARNING: Agent returned empty content!"
            );
        }

        // Build and return result
        Ok(TaskResult {
            task_id: TaskId::new(), // Will be overwritten by caller
            success: !loop_result.content.is_empty(),
            output: loop_result.content,
            error: None,
            artifacts: vec![],
            completed_at: chrono::Utc::now(),
            tokens_used: Some(TokenUsage {
                prompt_tokens: loop_result.total_usage.prompt_tokens,
                completion_tokens: loop_result.total_usage.completion_tokens,
                total_tokens: loop_result.total_usage.total_tokens,
            }),
        })
    }

    /// List all available agents.
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

    /// Check if a specific agent exists.
    async fn agent_exists(&self, agent_name: &str) -> bool {
        let state = match self.get_state() {
            Ok(s) => s,
            Err(_) => return false,
        };

        let manifests = state.kernel.list_agents().await;
        manifests.iter().any(|m| m.name == agent_name)
    }

    /// Execute an agent with event callback for progress tracking.
    async fn execute_agent_with_events(
        &self,
        application_id: &ApplicationId,
        agent_name: &str,
        prompt: &str,
        context: Option<TaskContext>,
        event_tx: Option<tokio::sync::mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
    ) -> Result<TaskResult, String> {
        let state = self.get_state()?;

        info!(
            application_id = %application_id.0,
            agent = agent_name,
            prompt_preview = %prompt.chars().take(50).collect::<String>(),
            has_event_tx = event_tx.is_some(),
            "Executing agent with events"
        );

        // Find the app directory for this application_id
        let app_dirs = state.app_dirs.read().await;
        let app_dir = app_dirs
            .iter()
            .find(|(id, _)| **id == *application_id)
            .map(|(_, path)| path.clone())
            .ok_or_else(|| format!("Application '{}' not found", application_id))?;
        drop(app_dirs);

        // Load agent's persona
        let persona = Self::load_persona(&app_dir, agent_name).await;

        // Get agent's capabilities from kernel
        let agent_manifests = state.kernel.list_agents().await;
        let agent_info = agent_manifests
            .iter()
            .find(|m| m.name == agent_name);

        let capabilities: Vec<String> = agent_info
            .map(|a| a.capabilities.iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default();

        // Build system prompt
        let system_prompt = Self::build_system_prompt(agent_name, persona.as_ref(), &capabilities);

        // Build messages
        let mut messages = vec![LlmMessage::system(system_prompt)];

        // Add context if provided
        if let Some(ref ctx) = context {
            if !ctx.artifacts.is_empty() {
                let context_msg = format!(
                    "Context artifacts available:\n{}",
                    ctx.artifacts.join("\n")
                );
                messages.push(LlmMessage::user(context_msg));
            }
        }

        // Add the main prompt
        messages.push(LlmMessage::user(prompt.to_string()));

        // Get tool definitions
        let tool_defs = state.tools.to_definitions();

        // Model resolution
        let model = if let Some(manifest) = state.kernel.get_agent_by_name(agent_name).await {
            if !manifest.model.is_empty() {
                manifest.model
            } else {
                state.default_model.clone()
            }
        } else {
            state.default_model.clone()
        };

        // Build LLM options
        let options = LlmOptions {
            model,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            stop_sequences: vec![],
            tools: Some(tool_defs),
        };

        // Use AgenticLoop with events
        let loop_config = RuntimeConfig {
            max_iterations: 25,
            tool_timeout: std::time::Duration::from_secs(300), // 5 minutes, matches Claude Code Driver default
        };
        let agentic_loop = AgenticLoop::new(loop_config);

        let agent_id = AgentId::new();
        let permission = Permission {
            level: macaca_proto::PermissionLevel::User,
            allowed_tools: vec![],
            allowed_paths: vec![],
            network_access: true,
        };

        info!(
            application_id = %application_id.0,
            agent = agent_name,
            "Starting agentic loop with events"
        );

        // Update agent activity to "working" before execution
        if let Some(agent_manifest) = state.kernel.get_agent_by_name(agent_name).await {
            state.kernel.status_tracker()
                .set_working(&agent_manifest.id, &format!("Executing: {}", prompt.chars().take(50).collect::<String>()))
                .await;
        }

        let loop_result = match agentic_loop
            .run_with_events(
                &agent_id,
                state.llm.as_ref(),
                state.tools.as_ref(),
                messages,
                &options,
                &permission,
                None,
                event_tx,
            )
            .await
        {
            Ok(result) => result,
            Err(e) => {
                // Update agent activity to "error" on failure
                let error_msg = format!("{}", e);
                if let Some(agent_manifest) = state.kernel.get_agent_by_name(agent_name).await {
                    state.kernel.status_tracker()
                        .set_error(&agent_manifest.id, &error_msg)
                        .await;
                }
                return Err(format!("Agentic loop failed: {}", e));
            }
        };

        // Update agent activity to "idle" after successful execution
        if let Some(agent_manifest) = state.kernel.get_agent_by_name(agent_name).await {
            state.kernel.status_tracker()
                .set_idle(&agent_manifest.id)
                .await;
        }

        info!(
            application_id = %application_id.0,
            agent = agent_name,
            success = !loop_result.content.is_empty(),
            output_len = loop_result.content.len(),
            iterations = loop_result.iterations,
            total_tokens = loop_result.total_usage.total_tokens,
            "Agent execution with events completed"
        );

        Ok(TaskResult {
            task_id: TaskId::new(),
            success: !loop_result.content.is_empty(),
            output: loop_result.content,
            error: None,
            artifacts: vec![],
            completed_at: chrono::Utc::now(),
            tokens_used: Some(TokenUsage {
                prompt_tokens: loop_result.total_usage.prompt_tokens,
                completion_tokens: loop_result.total_usage.completion_tokens,
                total_tokens: loop_result.total_usage.total_tokens,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt_with_capabilities() {
        let prompt = WebAgentRunner::build_system_prompt(
            "test_agent",
            None,
            &["code_generation".to_string(), "testing".to_string()],
        );
        assert!(prompt.contains("test_agent"));
        assert!(prompt.contains("code_generation"));
        assert!(prompt.contains("testing"));
    }

    #[test]
    fn test_build_system_prompt_without_capabilities() {
        let prompt = WebAgentRunner::build_system_prompt("simple_agent", None, &[]);
        assert!(prompt.contains("simple_agent"));
    }
}
