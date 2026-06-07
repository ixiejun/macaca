//! Service-owned framework runtime agent orchestration (Template Method + Adapter).
//!
//! `ComposedAgentExecutionBackend` injects a [`FrameworkRuntimeAgentPort`].
//! This module provides the **default production implementation** that:
//! 1. Delegates framework agent **construction** to a shell [`FrameworkAgentConstructionPort`].
//! 2. Owns the provider-neutral **execution** step (`ConstructedRuntimeAgent::reply_user_prompt`).
//!
//! Web/CLI shells therefore no longer implement `FrameworkRuntimeAgentPort` directly;
//! they only supply construction wiring until `FrameworkRunner` fully migrates to
//! runtime-host (task 4.3.2).

use std::sync::Arc;

use async_trait::async_trait;
use macaca_framework::model::ToolChoice;
use macaca_proto::{
    AgentContextSnapshot, AgentExecutionCommand, AgentExecutionEvent, AGENT_EXECUTION_SERVICE_ID,
};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::agent_execution_ports::{
    ConstructedRuntimeAgent, FrameworkAgentConstructionPort, FrameworkRuntimeAgentPort,
    OpaqueExecutionControlHandle,
};

/// Provider-neutral reason codes for framework runtime agent audit replay.
pub const REASON_FRAMEWORK_AGENT_CONSTRUCTED: &str =
    "agent_execution.framework_runtime_agent.constructed";
pub const REASON_FRAMEWORK_AGENT_REPLY_COMPLETED: &str =
    "agent_execution.framework_runtime_agent.reply.completed";
pub const REASON_FRAMEWORK_AGENT_REPLY_FAILED: &str =
    "agent_execution.framework_runtime_agent.reply.failed";

/// Runtime-host implementation of [`FrameworkRuntimeAgentPort`].
///
/// # Design pattern
///
/// - **Template Method**: construction is a hook (`FrameworkAgentConstructionPort`);
///   reply orchestration is fixed in this type.
/// - **Adapter**: shells adapt local composition (`AppState`, SSE, middleware) behind
///   the construction port without owning service semantics.
pub struct ServiceBackedFrameworkRuntimeAgentPort {
    construction: Arc<dyn FrameworkAgentConstructionPort>,
    bus_source: String,
}

impl ServiceBackedFrameworkRuntimeAgentPort {
    /// Wire the service-backed framework port with a shell construction adapter.
    pub fn new(
        construction: Arc<dyn FrameworkAgentConstructionPort>,
        bus_source: impl Into<String>,
    ) -> Self {
        let bus_source = bus_source.into();
        info!(
            bus_source = %bus_source,
            service_id = AGENT_EXECUTION_SERVICE_ID,
            pattern = "template_method+adapter",
            "service-backed framework runtime agent port wired"
        );
        Self {
            construction,
            bus_source,
        }
    }
}

#[async_trait]
impl FrameworkRuntimeAgentPort for ServiceBackedFrameworkRuntimeAgentPort {
    async fn run_react_agent(
        &self,
        command: &AgentExecutionCommand,
        context_snapshot: &AgentContextSnapshot,
        agent_event_tx: mpsc::Sender<AgentExecutionEvent>,
        execution_control: Option<OpaqueExecutionControlHandle>,
        max_iters: usize,
        tool_choice: Option<ToolChoice>,
        user_prompt: String,
    ) -> Result<String, String> {
        info!(
            trace_id = %command.trace.trace_id,
            service_id = AGENT_EXECUTION_SERVICE_ID,
            target_agent = %command.target_agent,
            session_id = %command.session_id,
            bus_source = %self.bus_source,
            max_iters,
            reason_code = REASON_FRAMEWORK_AGENT_CONSTRUCTED,
            "service-backed framework runtime agent starting construction"
        );

        let agent = self
            .construction
            .build_runtime_react_agent(
                command,
                context_snapshot,
                agent_event_tx,
                execution_control,
                max_iters,
                tool_choice,
            )
            .await
            .map_err(|error| {
                warn!(
                    trace_id = %command.trace.trace_id,
                    service_id = AGENT_EXECUTION_SERVICE_ID,
                    target_agent = %command.target_agent,
                    bus_source = %self.bus_source,
                    error = %error,
                    "framework agent construction failed"
                );
                error
            })?;

        info!(
            trace_id = %command.trace.trace_id,
            service_id = AGENT_EXECUTION_SERVICE_ID,
            target_agent = %command.target_agent,
            bus_source = %self.bus_source,
            reason_code = REASON_FRAMEWORK_AGENT_CONSTRUCTED,
            "framework agent constructed, starting react reply loop"
        );

        match agent.reply_user_prompt(user_prompt).await {
            Ok(output) => {
                info!(
                    trace_id = %command.trace.trace_id,
                    service_id = AGENT_EXECUTION_SERVICE_ID,
                    target_agent = %command.target_agent,
                    bus_source = %self.bus_source,
                    output_len = output.len(),
                    reason_code = REASON_FRAMEWORK_AGENT_REPLY_COMPLETED,
                    "framework react reply completed"
                );
                Ok(output)
            }
            Err(error) => {
                warn!(
                    trace_id = %command.trace.trace_id,
                    service_id = AGENT_EXECUTION_SERVICE_ID,
                    target_agent = %command.target_agent,
                    bus_source = %self.bus_source,
                    error = %error,
                    reason_code = REASON_FRAMEWORK_AGENT_REPLY_FAILED,
                    "framework react reply failed"
                );
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{
        AgentContextBuildCommand, AgentExecutionIntent, AgentExecutionPolicyContext,
        ApplicationId, TraceContext,
    };

    /// Stub agent that returns a deterministic string for contract tests.
    struct StubConstructedAgent {
        output: String,
    }

    #[async_trait]
    impl ConstructedRuntimeAgent for StubConstructedAgent {
        async fn reply_user_prompt(&self, _user_prompt: String) -> Result<String, String> {
            Ok(self.output.clone())
        }
    }

    struct StubConstruction {
        output: String,
        should_fail: bool,
    }

    #[async_trait]
    impl FrameworkAgentConstructionPort for StubConstruction {
        async fn build_runtime_react_agent(
            &self,
            _command: &AgentExecutionCommand,
            _context_snapshot: &AgentContextSnapshot,
            _agent_event_tx: mpsc::Sender<AgentExecutionEvent>,
            _execution_control: Option<OpaqueExecutionControlHandle>,
            _max_iters: usize,
            _tool_choice: Option<ToolChoice>,
        ) -> Result<Box<dyn ConstructedRuntimeAgent>, String> {
            if self.should_fail {
                return Err("construction unavailable".into());
            }
            Ok(Box::new(StubConstructedAgent {
                output: self.output.clone(),
            }))
        }
    }

    fn sample_command() -> AgentExecutionCommand {
        AgentExecutionCommand::new(
            ApplicationId::from_name("framework-runtime-agent-contract"),
            "session-contract",
            "runtime-agent",
            AgentExecutionIntent::SdkInvocation,
            "contract user prompt",
            TraceContext::new("framework-runtime-agent-contract"),
        )
        .expect("sample command")
    }

    fn sample_snapshot(command: &AgentExecutionCommand) -> AgentContextSnapshot {
        AgentContextSnapshot::minimal(
            &AgentContextBuildCommand {
                application_id: command.application_id,
                session_id: command.session_id.clone(),
                task_id: command.task_id,
                target_agent: command.target_agent.clone(),
                execution_intent: command.execution_intent.clone(),
                trace: command.trace.clone(),
                policy: AgentExecutionPolicyContext::default(),
                context_budget_tokens: None,
                metadata: Default::default(),
            },
            "system prompt".to_string(),
        )
    }

    #[tokio::test]
    async fn service_backed_port_delegates_construction_then_runs_reply() {
        let port = ServiceBackedFrameworkRuntimeAgentPort::new(
            Arc::new(StubConstruction {
                output: "hello from stub".into(),
                should_fail: false,
            }),
            "test.framework_runtime_agent",
        );
        let command = sample_command();
        let snapshot = sample_snapshot(&command);
        let (tx, _rx) = mpsc::channel(4);

        let output = port
            .run_react_agent(
                &command,
                &snapshot,
                tx,
                None,
                5,
                None,
                "user question".into(),
            )
            .await
            .expect("port should succeed");

        assert_eq!(output, "hello from stub");
    }

    #[tokio::test]
    async fn service_backed_port_surfaces_construction_failures() {
        let port = ServiceBackedFrameworkRuntimeAgentPort::new(
            Arc::new(StubConstruction {
                output: String::new(),
                should_fail: true,
            }),
            "test.framework_runtime_agent",
        );
        let command = sample_command();
        let snapshot = sample_snapshot(&command);
        let (tx, _rx) = mpsc::channel(4);

        let error = port
            .run_react_agent(&command, &snapshot, tx, None, 5, None, "prompt".into())
            .await
            .expect_err("construction failure should propagate");

        assert!(error.contains("construction unavailable"));
    }

}
