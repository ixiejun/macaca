//! Composed Agent Execution backend owned by runtime-host.
//!
//! This type is the Template Method owner for one `agent.execute` service call.
//! Provider-neutral orchestration lives here; shell and framework specifics are
//! injected through [`AgentExecutionHostAdapter`] and [`FrameworkRuntimeAgentPort`].

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{AgentExecutionCommand, AgentExecutionResult, ServiceResult, TaskId};
use tracing::{info, warn};

use crate::agent_execution_orchestration::{
    build_agent_context_snapshot_via_service, execute_exact_heartbeat_shell,
    failed_execution_result, heartbeat_exact_shell_contract, runtime_agent_max_iters,
    runtime_agent_tool_choice, should_skip_heartbeat_without_source, user_prompt_with_context,
};
use crate::agent_execution_ports::{
    completed_execution_result, AgentExecutionHostAdapter, AgentExecutionOutputHasher,
    FrameworkRuntimeAgentPort,
};
use crate::agent_execution_service_provider::AgentExecutionBackend;
use crate::ServiceRuntime;

/// Runtime-host execution backend composed from injected host + framework ports.
pub struct ComposedAgentExecutionBackend {
    service_runtime: Arc<ServiceRuntime>,
    bus_source: String,
    host: Arc<dyn AgentExecutionHostAdapter>,
    framework: Arc<dyn FrameworkRuntimeAgentPort>,
    output_hasher: Arc<dyn AgentExecutionOutputHasher>,
}

impl ComposedAgentExecutionBackend {
    /// Wire a composed backend with explicit adapter ports.
    pub fn new(
        service_runtime: Arc<ServiceRuntime>,
        bus_source: impl Into<String>,
        host: Arc<dyn AgentExecutionHostAdapter>,
        framework: Arc<dyn FrameworkRuntimeAgentPort>,
        output_hasher: Arc<dyn AgentExecutionOutputHasher>,
    ) -> Self {
        let bus_source = bus_source.into();
        crate::agent_execution_ports::log_composed_backend_wired(&bus_source);
        Self {
            service_runtime,
            bus_source,
            host,
            framework,
            output_hasher,
        }
    }
}

#[async_trait]
impl AgentExecutionBackend for ComposedAgentExecutionBackend {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
        let task_id = command.task_id.unwrap_or_else(TaskId::new);
        info!(
            trace_id = %command.trace.trace_id,
            service_id = macaca_proto::AGENT_EXECUTION_SERVICE_ID,
            target_agent = %command.target_agent,
            intent = ?command.execution_intent,
            bus_source = %self.bus_source,
            "composed agent execution backend starting run"
        );

        let context_snapshot = build_agent_context_snapshot_via_service(
            &self.service_runtime,
            &self.bus_source,
            &command,
        )
        .await?;

        if should_skip_heartbeat_without_source(&command, &context_snapshot) {
            warn!(
                trace_id = %command.trace.trace_id,
                target_agent = %command.target_agent,
                "heartbeat intent skipped because HEARTBEAT.md source evidence is absent"
            );
            return Ok(AgentExecutionResult::skipped(
                &AgentExecutionCommand {
                    task_id: Some(task_id),
                    ..command
                },
                "heartbeat_profile_missing",
                Some(context_snapshot),
            ));
        }

        // Single execution path: ComposedAgentExecutionBackend is the sole owner
        // of coarse executor lifecycle events for every `service.agent_execution` run.
        const EMIT_EXECUTOR_LIFECYCLE: bool = true;
        self.host
            .on_run_started(&command, task_id, EMIT_EXECUTOR_LIFECYCLE)
            .await;
        self.host
            .observe_application_execution_requested(&command)
            .await;

        let (agent_event_tx, evidence_observer) =
            self.host.create_evidence_observer(&command, task_id).await;

        if let Some(command_text) = heartbeat_exact_shell_contract(&command, &context_snapshot) {
            let shell_result = execute_exact_heartbeat_shell(&command_text, &agent_event_tx).await;
            drop(agent_event_tx);
            return match shell_result {
                Ok(shell_output) => {
                    self.host
                        .on_run_completed(
                            &command,
                            task_id,
                            EMIT_EXECUTOR_LIFECYCLE,
                            "heartbeat exact shell completed",
                        )
                        .await;
                    let output = serde_json::json!({
                        "output": "heartbeat exact shell completed",
                        "shell": shell_output,
                    });
                    let mut result = completed_execution_result(
                        &command,
                        task_id,
                        context_snapshot,
                        output.clone(),
                        self.output_hasher.stable_output_hash(&output),
                        evidence_observer.finish().await,
                    );
                    result
                        .metadata
                        .insert("execution_mode".into(), "heartbeat_exact_shell".into());
                    Ok(result)
                }
                Err(error) => {
                    self.host
                        .on_run_failed(&command, task_id, EMIT_EXECUTOR_LIFECYCLE, &error)
                        .await;
                    Ok(failed_execution_result(
                        &command,
                        task_id,
                        error,
                        Some(context_snapshot),
                    ))
                }
            };
        }

        let control_policy = self.host.resolve_execution_control_policy(&command).await?;
        let execution_control_registration = self
            .host
            .register_execution_control(&command, control_policy)
            .await?;
        let execution_control = match execution_control_registration {
            Some((policy, execution_id)) => {
                self.host
                    .install_execution_control(&command, policy, execution_id)
                    .await
            }
            None => None,
        };

        let max_iters = runtime_agent_max_iters(&command);
        let tool_choice = runtime_agent_tool_choice(&command);
        info!(
            target_agent = %command.target_agent,
            max_iters,
            tool_choice = ?tool_choice,
            "runtime agent policy selected for composed backend run"
        );

        let user_prompt = user_prompt_with_context(&command);
        match self
            .framework
            .run_react_agent(
                &command,
                &context_snapshot,
                agent_event_tx,
                execution_control,
                max_iters,
                tool_choice,
                user_prompt,
            )
            .await
        {
            Ok(output_text) => {
                self.host
                    .on_run_completed(&command, task_id, EMIT_EXECUTOR_LIFECYCLE, &output_text)
                    .await;
                let output = serde_json::json!({ "output": output_text });
                Ok(completed_execution_result(
                    &command,
                    task_id,
                    context_snapshot,
                    output.clone(),
                    self.output_hasher.stable_output_hash(&output),
                    evidence_observer.finish().await,
                ))
            }
            Err(error) => {
                self.host
                    .on_run_failed(&command, task_id, EMIT_EXECUTOR_LIFECYCLE, &error)
                    .await;
                Ok(failed_execution_result(
                    &command,
                    task_id,
                    error,
                    Some(context_snapshot),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use macaca_proto::{AgentContextSnapshot, AgentExecutionIntent, TraceContext};

    struct UnavailableFramework;

    #[async_trait]
    impl FrameworkRuntimeAgentPort for UnavailableFramework {
        async fn run_react_agent(
            &self,
            _command: &AgentExecutionCommand,
            _context_snapshot: &AgentContextSnapshot,
            _agent_event_tx: tokio::sync::mpsc::Sender<macaca_proto::AgentExecutionEvent>,
            _execution_control: Option<crate::agent_execution_ports::OpaqueExecutionControlHandle>,
            _max_iters: usize,
            _tool_choice: Option<macaca_framework::model::ToolChoice>,
            _user_prompt: String,
        ) -> Result<String, String> {
            Err("framework unavailable in test".into())
        }
    }

    struct NoopHost;

    #[async_trait]
    impl AgentExecutionHostAdapter for NoopHost {
        async fn resolve_execution_control_policy(
            &self,
            _command: &AgentExecutionCommand,
        ) -> ServiceResult<macaca_proto::ExecutionControlResolvedPolicy> {
            Ok(macaca_proto::ExecutionControlResolvedPolicy {
                status: macaca_proto::ExecutionControlResolutionStatus::Disabled,
                policy: None,
                reason_code: "test.disabled".into(),
                metadata: std::collections::BTreeMap::new(),
                events: Vec::new(),
            })
        }

        async fn register_execution_control(
            &self,
            _command: &AgentExecutionCommand,
            _policy: macaca_proto::ExecutionControlResolvedPolicy,
        ) -> ServiceResult<Option<(macaca_proto::ExecutionControlPolicy, String)>> {
            Ok(None)
        }

        async fn install_execution_control(
            &self,
            _command: &AgentExecutionCommand,
            _policy: macaca_proto::ExecutionControlPolicy,
            _execution_id: String,
        ) -> Option<crate::agent_execution_ports::OpaqueExecutionControlHandle> {
            None
        }

        async fn on_run_started(
            &self,
            _command: &AgentExecutionCommand,
            _task_id: TaskId,
            _emit_lifecycle: bool,
        ) {
        }

        async fn on_run_completed(
            &self,
            _command: &AgentExecutionCommand,
            _task_id: TaskId,
            _emit_lifecycle: bool,
            _output_preview: &str,
        ) {
        }

        async fn on_run_failed(
            &self,
            _command: &AgentExecutionCommand,
            _task_id: TaskId,
            _emit_lifecycle: bool,
            _error: &str,
        ) {
        }

        async fn observe_application_execution_requested(&self, _command: &AgentExecutionCommand) {}

        async fn create_evidence_observer(
            &self,
            _command: &AgentExecutionCommand,
            _task_id: TaskId,
        ) -> (
            tokio::sync::mpsc::Sender<macaca_proto::AgentExecutionEvent>,
            Box<dyn crate::agent_execution_ports::AgentExecutionEvidenceCollector>,
        ) {
            struct EmptyCollector;
            #[async_trait]
            impl crate::agent_execution_ports::AgentExecutionEvidenceCollector for EmptyCollector {
                async fn finish(self: Box<Self>) -> BTreeMap<String, String> {
                    BTreeMap::new()
                }
            }
            let (tx, _rx) = tokio::sync::mpsc::channel(1);
            (tx, Box::new(EmptyCollector))
        }
    }

    // Composed backend unit tests require a full ServiceRuntime fixture; integration
    // coverage lives in macaca-web agent_execution_backend tests via Web adapters.
}
