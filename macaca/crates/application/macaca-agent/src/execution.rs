//! Service-client execution port adapters for registered agents.
//!
//! The [`AgentExecutionPort`] trait lives in [`macaca_proto`] so the kernel depends
//! only on foundation contracts. Production wiring uses [`ServiceClientAgentExecutionAdapter`]
//! to dispatch typed commands through `service.agent_execution`.
//!
//! For in-process test fixtures see [`crate::InProcessAgentExecutionPort`].

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionPort, AgentExecutionResult,
    AgentExecutionStatus, AgentId, AgentOutput, ApplicationId, MacacaError, MacacaResult,
    TokenUsage, TraceContext,
};

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
