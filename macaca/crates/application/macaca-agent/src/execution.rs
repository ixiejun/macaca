//! Provider-neutral execution port adapters for registered agents.
//!
//! The [`AgentExecutionPort`] trait lives in [`macaca_proto`] so the kernel depends
//! only on foundation contracts. This module keeps the legacy `Agent::run(llm, tools,
//! services)` ABI and service-client dispatch outside the microkernel.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionPort, AgentExecutionResult,
    AgentExecutionStatus, AgentId, AgentOutput, ApplicationId, MacacaError, MacacaResult,
    TokenUsage, TraceContext,
};

use crate::{Agent, AgentServices, LlmProvider, ToolCatalog};

/// Side registry holding runtime [`Agent`] instances for legacy in-process execution.
///
/// The kernel registry stores manifests only (identity + metadata). This companion
/// registry maps `agent_id` → `Arc<dyn Agent>` so [`LegacyAgentExecutionAdapter`] can
/// resolve runtime objects without the kernel depending on application-agent types.
#[derive(Default)]
pub struct LegacyAgentSideRegistry {
    agents: RwLock<HashMap<AgentId, Arc<dyn Agent>>>,
}

impl LegacyAgentSideRegistry {
    /// Creates an empty side registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a runtime agent instance keyed by its stable id.
    pub fn register_runtime_agent(&self, agent: Arc<dyn Agent>) -> MacacaResult<()> {
        let agent_id = agent.id();
        let mut agents = self
            .agents
            .write()
            .map_err(|_| MacacaError::Agent("legacy agent side registry lock poisoned".into()))?;
        agents.insert(agent_id, agent);
        tracing::info!(agent_id = %agent_id.0, "legacy runtime agent registered in side registry");
        Ok(())
    }

    /// Resolves a runtime agent by id.
    pub fn get_runtime_agent(&self, agent_id: &AgentId) -> Option<Arc<dyn Agent>> {
        self.agents
            .read()
            .ok()
            .and_then(|agents| agents.get(agent_id).cloned())
    }
}

/// Provider-neutral port for dispatching typed commands to `service.agent_execution`.
///
/// Outbound port in a Hexagonal layout: `ServiceClientAgentExecutionAdapter` stays in
/// this crate while runtime-host composition roots supply a dispatcher backed by
/// `ServiceRuntime` / `SystemFacade`.
#[async_trait]
pub trait AgentExecutionDispatch: Send + Sync {
    /// Dispatch one validated `agent.execute` command through the service runtime.
    async fn dispatch_agent_execution(
        &self,
        command: AgentExecutionCommand,
    ) -> MacacaResult<AgentExecutionResult>;
}

/// Legacy adapter bridging kernel manifest registry to `Agent::run`.
///
/// Adapter pattern: provider handles (LLM/tools) and runtime agent instances live
/// outside the kernel; execution resolves agents from [`LegacyAgentSideRegistry`].
pub struct LegacyAgentExecutionAdapter {
    llm: Arc<dyn LlmProvider>,
    tools: Arc<dyn ToolCatalog>,
    side_registry: Arc<LegacyAgentSideRegistry>,
}

impl LegacyAgentExecutionAdapter {
    /// Shared side registry used by default bootstrap/test wiring.
    pub fn runtime_registry() -> Arc<LegacyAgentSideRegistry> {
        static REGISTRY: std::sync::OnceLock<Arc<LegacyAgentSideRegistry>> =
            std::sync::OnceLock::new();
        REGISTRY
            .get_or_init(|| Arc::new(LegacyAgentSideRegistry::new()))
            .clone()
    }

    /// Creates an adapter using the shared runtime side registry.
    pub fn new(llm: Arc<dyn LlmProvider>, tools: Arc<dyn ToolCatalog>) -> Self {
        Self::new_with_registry(llm, tools, Self::runtime_registry())
    }

    /// Creates an adapter bound to an explicit side registry (tests / composition roots).
    pub fn new_with_registry(
        llm: Arc<dyn LlmProvider>,
        tools: Arc<dyn ToolCatalog>,
        side_registry: Arc<LegacyAgentSideRegistry>,
    ) -> Self {
        tracing::info!(
            llm_provider = %llm.name(),
            "legacy agent execution adapter created"
        );
        Self {
            llm,
            tools,
            side_registry,
        }
    }

    /// Returns the side registry used by this adapter.
    pub fn side_registry(&self) -> Arc<LegacyAgentSideRegistry> {
        Arc::clone(&self.side_registry)
    }
}

#[async_trait]
impl AgentExecutionPort for LegacyAgentExecutionAdapter {
    async fn execute_registered_agent(&self, agent_id: &AgentId) -> MacacaResult<AgentOutput> {
        tracing::info!(
            agent_id = %agent_id.0,
            llm_provider = %self.llm.name(),
            "legacy agent execution adapter started"
        );
        let agent = self
            .side_registry
            .get_runtime_agent(agent_id)
            .ok_or_else(|| MacacaError::NotFound(format!("runtime agent not found: {agent_id}")))?;
        let services = AgentServices::builder().build();
        let output = agent
            .run(self.llm.as_ref(), self.tools.as_ref(), &services)
            .await;
        match &output {
            Ok(result) => tracing::info!(
                agent_id = %agent_id.0,
                artifacts = result.artifacts.len(),
                total_tokens = result.tokens_used.total_tokens,
                "legacy agent execution adapter finished"
            ),
            Err(error) => tracing::warn!(
                agent_id = %agent_id.0,
                error = %error,
                "legacy agent execution adapter failed"
            ),
        }
        output
    }
}

/// Service-client execution adapter — preferred production [`AgentExecutionPort`].
pub struct ServiceClientAgentExecutionAdapter {
    dispatch: Arc<dyn AgentExecutionDispatch>,
    application_id: ApplicationId,
}

impl ServiceClientAgentExecutionAdapter {
    /// Wire the adapter to a dispatcher and default application envelope.
    pub fn new(dispatch: Arc<dyn AgentExecutionDispatch>, application_id: ApplicationId) -> Self {
        tracing::info!(
            application_id = %application_id.0,
            "service-client agent execution adapter created"
        );
        Self {
            dispatch,
            application_id,
        }
    }
}

#[async_trait]
impl AgentExecutionPort for ServiceClientAgentExecutionAdapter {
    async fn execute_registered_agent(&self, agent_id: &AgentId) -> MacacaResult<AgentOutput> {
        let target_agent = agent_id.0.to_string();
        let trace = TraceContext::new(format!("kernel-agent-{}", agent_id.0));
        let command = AgentExecutionCommand::new(
            self.application_id,
            format!("kernel-session-{}", agent_id.0),
            target_agent.clone(),
            AgentExecutionIntent::SdkInvocation,
            format!("execute registered agent {}", agent_id.0),
            trace.clone(),
        )?;
        tracing::info!(
            agent_id = %agent_id.0,
            target_agent = %target_agent,
            trace_id = %trace.trace_id,
            service_id = "service.agent_execution",
            command = "agent.execute",
            "service-client agent execution adapter dispatch started"
        );
        let result = self.dispatch.dispatch_agent_execution(command).await?;
        agent_execution_result_to_output(&result)
    }
}

/// Map a typed service result into the kernel-facing `AgentOutput` contract.
fn agent_execution_result_to_output(result: &AgentExecutionResult) -> MacacaResult<AgentOutput> {
    match &result.status {
        AgentExecutionStatus::Completed => {
            let result_text = result
                .output
                .get("result")
                .or_else(|| result.output.get("output"))
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| result.output.to_string());
            let artifacts = result
                .output
                .get("artifacts")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let tokens_used = result
                .output
                .get("tokens_used")
                .and_then(|value| serde_json::from_value(value.clone()).ok())
                .unwrap_or_default();
            tracing::info!(
                target_agent = %result.target_agent,
                trace_id = %result.trace.trace_id,
                status = %result.status.as_str(),
                "service-client agent execution adapter dispatch completed"
            );
            Ok(AgentOutput {
                result: result_text,
                artifacts,
                tokens_used,
            })
        }
        status => {
            let reason = result
                .metadata
                .get("reason_code")
                .cloned()
                .or_else(|| {
                    result
                        .output
                        .get("error")
                        .and_then(|value| value.as_str())
                        .map(ToOwned::to_owned)
                })
                .unwrap_or_else(|| format!("agent execution status {}", status.as_str()));
            tracing::warn!(
                target_agent = %result.target_agent,
                trace_id = %result.trace.trace_id,
                status = %status.as_str(),
                reason_code = %reason,
                "service-client agent execution adapter dispatch unavailable or failed"
            );
            Err(MacacaError::Agent(format!(
                "Agent execution {}: {}",
                status.as_str(),
                reason
            )))
        }
    }
}

#[cfg(test)]
mod service_client_adapter_tests {
    use super::*;
    use async_trait::async_trait;
    use macaca_proto::{AgentState, Capability};

    struct RecordingDispatch {
        last_target: std::sync::Mutex<Option<String>>,
        next_result: AgentExecutionResult,
    }

    #[async_trait]
    impl AgentExecutionDispatch for RecordingDispatch {
        async fn dispatch_agent_execution(
            &self,
            command: AgentExecutionCommand,
        ) -> MacacaResult<AgentExecutionResult> {
            *self.last_target.lock().expect("lock") = Some(command.target_agent.clone());
            Ok(self.next_result.clone())
        }
    }

    fn sample_command() -> AgentExecutionCommand {
        let trace = TraceContext::new("trace-test");
        AgentExecutionCommand::new(
            ApplicationId::from_name("system"),
            "session-test",
            "agent-1",
            AgentExecutionIntent::SdkInvocation,
            "execute",
            trace,
        )
        .expect("command")
    }

    #[tokio::test]
    async fn service_client_adapter_maps_completed_service_result() {
        let command = sample_command();
        let dispatch = Arc::new(RecordingDispatch {
            last_target: std::sync::Mutex::new(None),
            next_result: AgentExecutionResult::completed(
                &command,
                serde_json::json!({
                    "result": "done",
                    "artifacts": ["a.txt"],
                    "tokens_used": TokenUsage::default(),
                }),
            ),
        });
        let adapter = ServiceClientAgentExecutionAdapter::new(
            dispatch.clone(),
            ApplicationId::from_name("system"),
        );
        let agent_id = AgentId::new();
        let output = adapter
            .execute_registered_agent(&agent_id)
            .await
            .expect("output");
        assert_eq!(output.result, "done");
        assert_eq!(output.artifacts, vec!["a.txt".to_string()]);
        assert_eq!(
            dispatch.last_target.lock().expect("lock").as_deref(),
            Some(agent_id.0.to_string().as_str())
        );
    }

    #[tokio::test]
    async fn service_client_adapter_surfaces_unavailable_without_fake_success() {
        let command = sample_command();
        let mut unavailable = AgentExecutionResult::completed(&command, serde_json::json!({}));
        unavailable.status = AgentExecutionStatus::Unavailable;
        unavailable
            .metadata
            .insert("reason_code".into(), "backend_missing".into());
        let dispatch = Arc::new(RecordingDispatch {
            last_target: std::sync::Mutex::new(None),
            next_result: unavailable,
        });
        let adapter =
            ServiceClientAgentExecutionAdapter::new(dispatch, ApplicationId::from_name("system"));
        let agent_id = AgentId::new();
        let error = adapter
            .execute_registered_agent(&agent_id)
            .await
            .expect_err("must fail");
        assert!(error.to_string().contains("unavailable"));
        assert!(error.to_string().contains("backend_missing"));
    }
}
