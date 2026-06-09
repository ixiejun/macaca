//! Shell adapter for framework ReAct agent construction (Adapter pattern).
//!
//! Agent Execution Service owns the reply orchestration via
//! `ServiceBackedFrameworkRuntimeAgentPort` in runtime-host.  This module is the
//! **only** web entry that still reaches `FrameworkRunner` for runtime-agent
//! construction; it implements [`FrameworkAgentConstructionPort`] so the web shell
//! does not implement [`FrameworkRuntimeAgentPort`] directly.
//!
//! Full construction logic migrates to runtime-host/framework in task 4.3.2; until
//! then this adapter isolates shell composition behind the port boundary.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_sdk::framework::agent::{Agent, HookedAgent};
use macaca_sdk::framework::message::Msg;
use macaca_sdk::framework::model::ToolChoice;
use macaca_sdk::framework::react_agent::ReActAgent;
use macaca_proto::{AgentContextSnapshot, AgentExecutionCommand, AgentExecutionEvent};
use macaca_runtime_host::{
    ConstructedRuntimeAgent, FrameworkAgentConstructionPort, OpaqueExecutionControlHandle,
};
use tokio::sync::mpsc;
use tracing::info;

use crate::framework_runner::{FrameworkRunner, RuntimeExecutionControl};
use crate::state::AppState;

/// Web shell adapter that builds framework ReAct agents through `FrameworkRunner`.
///
/// The adapter downcasts opaque execution-control handles to web-local middleware
/// wiring.  Runtime-host never inspects `RuntimeExecutionControl`.
pub(crate) struct WebFrameworkAgentConstructionPort {
    state: Arc<AppState>,
}

impl WebFrameworkAgentConstructionPort {
    /// Create a construction port bound to the web composition root.
    pub(crate) fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Decode the opaque execution-control handle installed by the web host adapter.
    fn decode_execution_control(
        handle: Option<OpaqueExecutionControlHandle>,
    ) -> Option<RuntimeExecutionControl> {
        handle.and_then(|opaque| {
            opaque
                .0
                .downcast::<RuntimeExecutionControl>()
                .ok()
                .map(|arc| (*arc).clone())
        })
    }
}

/// Wrapper that exposes a constructed `HookedAgent<ReActAgent>` through the
/// provider-neutral [`ConstructedRuntimeAgent`] trait.
struct WebConstructedRuntimeAgent {
    agent: HookedAgent<ReActAgent>,
}

#[async_trait]
impl ConstructedRuntimeAgent for WebConstructedRuntimeAgent {
    async fn reply_user_prompt(&self, user_prompt: String) -> Result<String, String> {
        let reply = self
            .agent
            .reply(Msg::user("user", user_prompt))
            .await
            .map_err(|error| error.to_string())?;
        Ok(reply.get_text())
    }
}

#[async_trait]
impl FrameworkAgentConstructionPort for WebFrameworkAgentConstructionPort {
    async fn build_runtime_react_agent(
        &self,
        command: &AgentExecutionCommand,
        context_snapshot: &AgentContextSnapshot,
        agent_event_tx: mpsc::Sender<AgentExecutionEvent>,
        execution_control: Option<OpaqueExecutionControlHandle>,
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
    ) -> Result<Box<dyn ConstructedRuntimeAgent>, String> {
        let runtime_control = Self::decode_execution_control(execution_control);

        info!(
            trace_id = %command.trace.trace_id,
            target_agent = %command.target_agent,
            session_id = %command.session_id,
            max_iters,
            has_execution_control = runtime_control.is_some(),
            "web framework construction adapter building runtime react agent"
        );

        let agent = FrameworkRunner::build_runtime_agent_from_context_snapshot_with_execution_policy(
            &self.state,
            context_snapshot,
            Some(agent_event_tx),
            runtime_control,
            max_iters,
            tool_choice,
        )
        .await?;

        Ok(Box::new(WebConstructedRuntimeAgent { agent }))
    }
}
