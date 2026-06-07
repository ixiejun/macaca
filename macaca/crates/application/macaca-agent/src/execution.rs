//! Provider-neutral execution ports for registered agents.
//!
//! This module keeps the legacy `Agent::run(llm, tools, services)` ABI outside
//! the kernel.  The kernel should coordinate identity, registry, status, and
//! scheduling invariants; replaceable provider handles belong behind an
//! execution port owned by the application-agent boundary until the execution
//! service contract fully replaces the legacy ABI.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, AgentExecutionStatus,
    AgentId, AgentOutput, ApplicationId, MacacaError, MacacaResult, TokenUsage, TraceContext,
};
use tokio::sync::RwLock;

use crate::{Agent, AgentServices, LlmProvider, ToolCatalog};

/// Provider-neutral port for dispatching typed commands to `service.agent_execution`.
///
/// This is the outbound port in a Hexagonal layout: `ServiceClientAgentExecutionAdapter`
/// stays in the application-agent crate while runtime-host or SDK composition roots
/// supply a concrete dispatcher backed by `ServiceRouter` / `SystemFacade`. The trait
/// intentionally exposes only the typed command/result pair so kernel code never
/// imports web shells, LLM providers, or tool catalogs.
#[async_trait]
pub trait AgentExecutionDispatch: Send + Sync {
    /// Dispatch one validated `agent.execute` command through the service runtime.
    ///
    /// Implementations must log `service_id`, `command`, and `trace_id` at start and
    /// must map service-unavailable replies into `MacacaError` without fabricating
    /// successful `AgentOutput` values.
    async fn dispatch_agent_execution(
        &self,
        command: AgentExecutionCommand,
    ) -> MacacaResult<AgentExecutionResult>;
}

/// Hot-swappable wrapper around `AgentExecutionPort` for composition-root wiring.
///
/// Web bootstrap must register agents in the kernel before `service.agent_execution`
/// is available. This Decorator keeps one stable `Arc` identity on the kernel
/// while allowing the host to replace the inner port after service providers
/// start, without re-registering agents or rebuilding the kernel registry.
pub struct SwappableAgentExecutionPort {
    inner: RwLock<Arc<dyn AgentExecutionPort>>,
}

impl SwappableAgentExecutionPort {
    /// Create a swappable port seeded with the bootstrap execution implementation.
    pub fn new(initial: Arc<dyn AgentExecutionPort>) -> Arc<Self> {
        tracing::info!("swappable agent execution port created");
        Arc::new(Self {
            inner: RwLock::new(initial),
        })
    }

    /// Replace the active execution port after service providers become available.
    ///
    /// Callers must ensure the new port routes through `service.agent_execution`
    /// before relying on kernel `execute_agent` in production paths.
    pub async fn replace(&self, port: Arc<dyn AgentExecutionPort>) {
        tracing::info!("swappable agent execution port replacement started");
        let mut guard = self.inner.write().await;
        *guard = port;
        tracing::info!("swappable agent execution port replacement completed");
    }
}

#[async_trait]
impl AgentExecutionPort for SwappableAgentExecutionPort {
    async fn execute_registered_agent(
        &self,
        agent_id: &AgentId,
        agent: &dyn Agent,
    ) -> MacacaResult<AgentOutput> {
        let guard = self.inner.read().await;
        guard.execute_registered_agent(agent_id, agent).await
    }
}

/// Provider-neutral port used by kernel code to execute one registered agent.
///
/// This is a small Port pattern boundary: callers pass an already-registered
/// agent and its stable identifier, while the implementation decides how that
/// agent is actually run.  Service-era implementations can dispatch typed
/// commands through a runtime service client; the temporary legacy adapter below
/// still bridges to `Agent::run` without forcing kernel code to store concrete
/// provider compatibility bundles.
#[async_trait]
pub trait AgentExecutionPort: Send + Sync {
    /// Execute the supplied agent and return its output.
    ///
    /// Implementations must emit trace-friendly logs at key execution points
    /// and return structured errors when execution is unavailable.  The port is
    /// intentionally generic and carries no application-specific workflow,
    /// provider, model, driver, gateway, or business-domain branching.
    async fn execute_registered_agent(
        &self,
        agent_id: &AgentId,
        agent: &dyn Agent,
    ) -> MacacaResult<AgentOutput>;
}

/// Legacy adapter for the current `Agent::run` execution ABI.
///
/// The adapter is an explicit Adapter pattern implementation. It contains the
/// provider-shaped bridge that existing agents still require, while keeping
/// production kernel state provider-neutral. Once agent execution is fully
/// serviceized, this adapter can be replaced by a service-client implementation
/// without changing the kernel registry or status code.
pub struct LegacyAgentExecutionAdapter {
    llm: Arc<dyn LlmProvider>,
    tools: Arc<dyn ToolCatalog>,
}

impl LegacyAgentExecutionAdapter {
    /// Create the adapter from shared provider handles.
    ///
    /// Handles are shared so composition roots can own provider lifecycles while
    /// this adapter only borrows them during execution. The log records the
    /// provider family name for operational traceability without exposing raw
    /// prompts, credentials, or provider payloads.
    pub fn new(llm: Arc<dyn LlmProvider>, tools: Arc<dyn ToolCatalog>) -> Self {
        tracing::info!(
            llm_provider = %llm.name(),
            "legacy agent execution adapter created"
        );
        Self { llm, tools }
    }
}

#[async_trait]
impl AgentExecutionPort for LegacyAgentExecutionAdapter {
    async fn execute_registered_agent(
        &self,
        agent_id: &AgentId,
        agent: &dyn Agent,
    ) -> MacacaResult<AgentOutput> {
        tracing::info!(
            agent_id = %agent_id.0,
            llm_provider = %self.llm.name(),
            "legacy agent execution adapter started"
        );
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

/// Service-client execution adapter — preferred production `AgentExecutionPort`.
///
/// The adapter implements the Adapter pattern on top of `AgentExecutionDispatch`.
/// Kernel and composition roots keep depending on `AgentExecutionPort`, while the
/// actual model/tool/loop work happens inside runtime-host's Agent Execution
/// service provider. Each kernel invocation is translated into a bounded
/// `AgentExecutionCommand` with `SdkInvocation` intent so audit tools can
/// distinguish registry-driven runs from chat or workflow paths.
pub struct ServiceClientAgentExecutionAdapter {
    dispatch: Arc<dyn AgentExecutionDispatch>,
    application_id: ApplicationId,
}

impl ServiceClientAgentExecutionAdapter {
    /// Wire the adapter to a dispatcher and default application envelope.
    ///
    /// `application_id` is a provider-neutral envelope used when the kernel
    /// executes a registered agent without an active application session. Hosts
    /// that always know the active application should supply the real id here.
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
    async fn execute_registered_agent(
        &self,
        agent_id: &AgentId,
        agent: &dyn Agent,
    ) -> MacacaResult<AgentOutput> {
        let target_agent = agent_id.0.to_string();
        let trace = TraceContext::new(format!("kernel-agent-{}", agent_id.0));
        let command = AgentExecutionCommand::new(
            self.application_id,
            format!("kernel-session-{}", agent_id.0),
            target_agent.clone(),
            AgentExecutionIntent::SdkInvocation,
            format!("execute registered agent {}", agent.id().0),
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
///
/// The conversion is intentionally conservative: only `Completed` statuses become
/// success values. Every other status is surfaced as a structured `MacacaError`
/// with the service reason code preserved for operators and audit replay.
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

/// Null Object execution port used when no execution service bridge is wired.
///
/// This object preserves service-client construction safety. A kernel can be
/// assembled before the legacy execution bridge exists, but any attempt to run
/// an agent receives a clear unavailable error instead of a fake success.
pub struct UnavailableAgentExecutionPort {
    reason: String,
}

impl UnavailableAgentExecutionPort {
    /// Create a reusable unavailable execution port with an operator-facing
    /// reason. The reason must stay generic and must not include secrets,
    /// prompts, manifests, package bytes, or application-specific payloads.
    pub fn new(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        tracing::info!(
            reason = %reason,
            "unavailable agent execution port created"
        );
        Self { reason }
    }
}

#[async_trait]
impl AgentExecutionPort for UnavailableAgentExecutionPort {
    async fn execute_registered_agent(
        &self,
        agent_id: &AgentId,
        _agent: &dyn Agent,
    ) -> MacacaResult<AgentOutput> {
        tracing::warn!(
            agent_id = %agent_id.0,
            reason = %self.reason,
            "agent execution requested without an execution bridge"
        );
        Err(MacacaError::Agent(format!(
            "Agent execution unavailable: {}",
            self.reason
        )))
    }
}

#[cfg(test)]
mod service_client_adapter_tests {
    use super::*;
    use crate::{Agent, AgentServices, LlmProvider, ToolCatalog};
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

    struct StaticAgent {
        id: AgentId,
    }

    #[async_trait]
    impl Agent for StaticAgent {
        fn id(&self) -> AgentId {
            self.id
        }

        fn capabilities(&self) -> &[Capability] {
            &[]
        }

        fn state(&self) -> AgentState {
            AgentState::Running
        }

        async fn run(
            &self,
            _llm: &dyn LlmProvider,
            _tools: &dyn ToolCatalog,
            _services: &AgentServices,
        ) -> MacacaResult<AgentOutput> {
            Ok(AgentOutput {
                result: "legacy".into(),
                artifacts: Vec::new(),
                tokens_used: TokenUsage::default(),
            })
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
        let agent = StaticAgent { id: agent_id };
        let output = adapter
            .execute_registered_agent(&agent_id, &agent)
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
        let agent = StaticAgent { id: agent_id };
        let error = adapter
            .execute_registered_agent(&agent_id, &agent)
            .await
            .expect_err("must fail");
        assert!(error.to_string().contains("unavailable"));
        assert!(error.to_string().contains("backend_missing"));
    }
}
