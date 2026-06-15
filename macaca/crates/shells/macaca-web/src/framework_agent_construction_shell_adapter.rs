//! Shell adapter for framework ReAct agent materialization (Adapter pattern).
//!
//! Agent Execution Service owns the reply orchestration via
//! `ServiceBackedFrameworkRuntimeAgentPort` in runtime-host. Runtime-host now
//! owns the framework construction service; this module supplies only the
//! host-local materialization hook required to assemble web tools, hooks, and
//! execution-control middleware.
//!
//! Full materialization logic can continue migrating downward without changing
//! the public runtime-host construction contract.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_host_composition::agent_execution::{
    ConstructedRuntimeAgent, FrameworkAgentMaterializationPort,
};
use macaca_host_composition::execution_control::OpaqueExecutionControlHandle;
use macaca_host_composition::framework::agent::{Agent, HookedAgent};
use macaca_host_composition::framework::message::Msg;
use macaca_host_composition::framework::model::ToolChoice;
use macaca_host_composition::framework::react_agent::ReActAgent;
use macaca_proto::{AgentContextSnapshot, AgentExecutionCommand, AgentExecutionEvent};
use tokio::sync::mpsc;
use tracing::info;

use crate::framework_runner::{FrameworkRunner, RuntimeExecutionControl};
use crate::state::AppState;

/// Web shell adapter that materializes framework ReAct agents through `FrameworkRunner`.
///
/// The adapter downcasts opaque execution-control handles to web-local
/// middleware wiring. Runtime-host never inspects `RuntimeExecutionControl` and
/// owns the higher-level construction service around this materializer.
pub(crate) struct WebFrameworkAgentMaterializationPort {
    state: Arc<AppState>,
}

impl WebFrameworkAgentMaterializationPort {
    /// Create a materialization port bound to the web composition root.
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
impl FrameworkAgentMaterializationPort for WebFrameworkAgentMaterializationPort {
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
            "web framework materialization adapter building runtime react agent"
        );

        let agent =
            FrameworkRunner::materialize_runtime_react_agent_from_context_snapshot_with_execution_policy(
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
