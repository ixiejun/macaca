//! Web composition backend for `service.agent_context`.
//!
//! The runtime-host crate owns the provider-neutral service boundary, while
//! Web owns the concrete context sources: application manifests, personas,
//! workspace guides, skill snapshots, and session-scoped event logs.  This
//! backend is therefore deliberately thin; it delegates composition to
//! `FrameworkRunner` so every application kind receives the same trusted
//! system prompt construction.

use async_trait::async_trait;
use macaca_host_composition::agent_execution::AgentContextBackend;
use macaca_proto::{AgentContextBuildCommand, AgentContextSnapshot, ServiceResult};
use std::sync::Arc;

use crate::framework_runner::FrameworkRunner;
use crate::state::AppState;

/// Web-owned implementation of the Agent Context system service.
pub(crate) struct WebAgentContextBackend {
    state: Arc<AppState>,
}

impl WebAgentContextBackend {
    /// Create a backend bound to the shared Web application state.
    pub(crate) fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl AgentContextBackend for WebAgentContextBackend {
    async fn build_context(
        &self,
        command: AgentContextBuildCommand,
    ) -> ServiceResult<AgentContextSnapshot> {
        Ok(FrameworkRunner::build_agent_context_snapshot(&self.state, command).await)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn context_backend_stays_thin_and_delegates_context_semantics() {
        let source = include_str!("agent_context_backend.rs");
        let forbidden_context_builder = ["build_context_system", "_prompt"].concat();
        let forbidden_prompt_composer = ["Prompt", "Composer"].concat();

        assert!(source.contains("FrameworkRunner::build_agent_context_snapshot"));
        assert!(!source.contains(&forbidden_context_builder));
        assert!(!source.contains(&forbidden_prompt_composer));
    }
}
