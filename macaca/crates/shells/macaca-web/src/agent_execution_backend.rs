//! Web composition backend for `service.agent_execution`.
//!
//! This module is the host-owned adapter that turns one provider-neutral
//! execution command into a framework-native agent run.  It first obtains a
//! trusted context snapshot through `service.agent_context`, then broadcasts
//! task lifecycle and agent trace events through the existing application
//! executor event channel so delegated WASM runs appear in the same UI traces
//! as YAML and executor-originated work.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_framework::agent::Agent;
use macaca_framework::message::Msg;
use macaca_kernel::executor::ExecutorEventFactory;
use macaca_proto::{
    AgentContextBuildCommand, AgentContextSnapshot, AgentExecutionCommand, AgentExecutionIntent,
    AgentExecutionResult, AgentExecutionStatus, ExecutionControlCheckpointMode,
    ExecutionControlCommandResult, ExecutionControlPolicy,
    ExecutionControlRegisterExecutionCommand, ExecutionControlResolutionStatus,
    ExecutionControlResolvePolicyCommand, ExecutionControlResolvedPolicy,
    ExecutionControlResumeSource, ExecutionControlScope, ExecutionControlTrigger, KernelServiceId,
    ServiceBusSource, ServiceError, ServiceResult, TaskId, AGENT_CONTEXT_SERVICE_ID,
    EXECUTION_CONTROL_SERVICE_ID,
};
#[cfg(test)]
use macaca_runtime_host::ExecutionControlPolicyResolver;
use macaca_runtime_host::{AgentExecutionBackend, ServiceRuntime};
use tokio::sync::mpsc;

use crate::agent_execution_evidence::{observed_agent_execution_events, stable_agent_output_hash};
use crate::framework_runner::{FrameworkRunner, RuntimeExecutionControl};
use crate::runtime_event_bridge::emit_execution_control_events;
use crate::state::AppState;

/// Web-owned implementation of the Agent Execution system service.
pub(crate) struct WebAgentExecutionBackend {
    state: Arc<AppState>,
    service_runtime: Arc<ServiceRuntime>,
}

impl WebAgentExecutionBackend {
    /// Create a backend with access to state and the shared ServiceRuntime.
    pub(crate) fn new(state: Arc<AppState>, service_runtime: Arc<ServiceRuntime>) -> Self {
        Self {
            state,
            service_runtime,
        }
    }

    async fn build_context_snapshot(
        &self,
        command: &AgentExecutionCommand,
    ) -> ServiceResult<AgentContextSnapshot> {
        let context_command = AgentContextBuildCommand::from_execution(command);
        let reply = self
            .service_runtime
            .call(
                &KernelServiceId::new(AGENT_CONTEXT_SERVICE_ID),
                ServiceBusSource::new("macaca.web.agent_execution"),
                context_command
                    .into_service_command()
                    .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            )
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        let output = reply.output.ok_or_else(|| {
            ServiceError::AdapterFailure("agent context service returned no output".into())
        })?;
        serde_json::from_value(output)
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))
    }

    fn user_prompt_with_context(command: &AgentExecutionCommand) -> String {
        let context = Self::structured_execution_context(command);
        if context.is_null() || context == serde_json::json!({}) {
            return command.user_prompt.clone();
        }
        let rendered_context =
            serde_json::to_string_pretty(&context).unwrap_or_else(|_| context.to_string());
        format!(
            "{}\n\nStructured evidence context for this delegated task:\n```json\n{}\n```",
            command.user_prompt, rendered_context
        )
    }

    /// Build provider-neutral context that is safe to show to the runtime agent.
    ///
    /// `delegated_context` carries source-specific audit facts such as a
    /// heartbeat run id. `evidence.*` metadata carries generic completion
    /// requirements such as an expected artifact path. Rendering both in one
    /// bounded JSON object lets the model use the exact requirement while the
    /// Runtime Host still validates completion through sanitized result
    /// metadata, not through prompt parsing or application-specific branches.
    fn structured_execution_context(command: &AgentExecutionCommand) -> serde_json::Value {
        let mut context = serde_json::Map::new();
        if !command.delegated_context.is_null()
            && command.delegated_context != serde_json::json!({})
        {
            context.insert(
                "delegated_context".into(),
                command.delegated_context.clone(),
            );
        }
        let evidence_requirements = command
            .metadata
            .iter()
            .filter_map(|(key, value)| {
                key.strip_prefix("evidence.")
                    .filter(|_| !value.trim().is_empty())
                    .map(|name| (name.to_string(), serde_json::Value::String(value.clone())))
            })
            .collect::<serde_json::Map<String, serde_json::Value>>();
        if !evidence_requirements.is_empty() {
            context.insert(
                "evidence_requirements".into(),
                serde_json::Value::Object(evidence_requirements),
            );
        }
        serde_json::Value::Object(context)
    }

    /// Some legacy kernel/executor callers already emit task lifecycle events
    /// around the `AgentRunner` trait.  They still need the Agent Execution
    /// service for context/model/tool semantics, but duplicate started/completed
    /// events would make session traces noisy.  The suppression flag only
    /// affects coarse lifecycle events; fine-grained agent events are still
    /// forwarded from framework hooks.
    fn should_emit_executor_lifecycle(command: &AgentExecutionCommand) -> bool {
        command
            .metadata
            .get("suppress_executor_lifecycle")
            .map(|value| value != "true")
            .unwrap_or(true)
    }

    /// Verify that trusted Agent Context supplied HEARTBEAT.md evidence.
    ///
    /// Heartbeat intent is selected by manifest declarations, not by scanning
    /// profile files. Once selected, execution still requires Agent Context to
    /// prove that `HEARTBEAT.md` participated in the trusted source set. This
    /// guard runs before runtime-agent construction, model calls, tool calls,
    /// and executor lifecycle emission so a missing profile is an auditable
    /// no-op instead of an accidental generic prompt run.
    fn has_heartbeat_source_evidence(context_snapshot: &AgentContextSnapshot) -> bool {
        context_snapshot.sources.iter().any(|source| {
            source.kind == "profile_file"
                && (source.name == "HEARTBEAT.md"
                    || source
                        .location
                        .as_deref()
                        .is_some_and(|location| location.ends_with("/HEARTBEAT.md")))
        })
    }

    fn should_skip_heartbeat_without_source(
        command: &AgentExecutionCommand,
        context_snapshot: &AgentContextSnapshot,
    ) -> bool {
        matches!(command.execution_intent, AgentExecutionIntent::Heartbeat)
            && !Self::has_heartbeat_source_evidence(context_snapshot)
    }

    /// Resolve execution-control policy for this run through the system service.
    ///
    /// Stage 1 supports the command override as the dynamic app-selected entry
    /// point. Until manifest projection is wired for legacy YAML chat sessions,
    /// ChatMainThread receives a deprecated compatibility policy that preserves
    /// the existing goal pause contract without branching on application names.
    async fn resolve_execution_control_policy_via_service(
        &self,
        command: &AgentExecutionCommand,
    ) -> ServiceResult<ExecutionControlResolvedPolicy> {
        let compat_policy = Self::legacy_execution_control_policy(command);
        let command_declared_policy =
            command
                .execution_control_override
                .as_ref()
                .map(|override_policy| {
                    ExecutionControlPolicy::enabled(
                        override_policy.triggers.clone(),
                        override_policy.resume_sources.clone(),
                        override_policy.checkpoint_mode.clone(),
                    )
                    .allow_command_overrides(true)
                });
        let app_policy = compat_policy.as_ref().or(command_declared_policy.as_ref());
        let execution_id = Self::execution_control_execution_id(command);
        let scope = Self::execution_control_scope(command, execution_id)?;
        let service_command = ExecutionControlResolvePolicyCommand {
            scope,
            app_default: app_policy.cloned(),
            command_override: command.execution_control_override.clone(),
        }
        .into_service_command()
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        let reply = self
            .service_runtime
            .call(
                &KernelServiceId::new(EXECUTION_CONTROL_SERVICE_ID),
                ServiceBusSource::new("macaca.web.agent_execution"),
                service_command,
            )
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        let output = reply.output.ok_or_else(|| {
            ServiceError::AdapterFailure("execution control service returned no policy".into())
        })?;
        let mut resolution: ExecutionControlResolvedPolicy = serde_json::from_value(output)
            .map_err(|error| {
                ServiceError::AdapterFailure(format!(
                    "execution control policy decode failed: {error}"
                ))
            })?;
        if compat_policy.is_some() && resolution.status == ExecutionControlResolutionStatus::Enabled
        {
            resolution.metadata.insert(
                "compatibility".into(),
                "legacy_chat_main_thread_goal_pause".into(),
            );
        }
        emit_execution_control_events(
            &self.state,
            &command.session_id,
            "macaca.web.agent_execution",
            &resolution.events,
        )
        .await;
        Ok(resolution)
    }

    /// Resolve policy with the same deterministic Strategy used by the service.
    ///
    /// This helper exists only for unit tests that validate compatibility
    /// selection without constructing a full Web `AppState` and service
    /// runtime. Production execution uses `resolve_execution_control_policy_via_service`
    /// so the call still passes through decorators, service logs, and replayable
    /// policy-resolution events before local pause/resume adapters are installed.
    #[cfg(test)]
    fn resolve_execution_control_policy(
        command: &AgentExecutionCommand,
    ) -> ExecutionControlResolvedPolicy {
        let compat_policy = Self::legacy_execution_control_policy(command);
        let command_declared_policy =
            command
                .execution_control_override
                .as_ref()
                .map(|override_policy| {
                    ExecutionControlPolicy::enabled(
                        override_policy.triggers.clone(),
                        override_policy.resume_sources.clone(),
                        override_policy.checkpoint_mode.clone(),
                    )
                    .allow_command_overrides(true)
                });
        let app_policy = compat_policy.as_ref().or(command_declared_policy.as_ref());
        let mut resolution = ExecutionControlPolicyResolver::resolve(
            app_policy,
            command.execution_control_override.as_ref(),
        );
        if compat_policy.is_some() && resolution.status == ExecutionControlResolutionStatus::Enabled
        {
            resolution.metadata.insert(
                "compatibility".into(),
                "legacy_chat_main_thread_goal_pause".into(),
            );
        }
        resolution
    }

    fn legacy_execution_control_policy(
        command: &AgentExecutionCommand,
    ) -> Option<ExecutionControlPolicy> {
        if command.execution_control_override.is_some()
            || !matches!(
                command.execution_intent,
                AgentExecutionIntent::ChatMainThread
            )
        {
            return None;
        }

        Some(
            ExecutionControlPolicy::enabled(
                vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
                vec![ExecutionControlResumeSource::goal_lifecycle()],
                ExecutionControlCheckpointMode::ReferenceOnly,
            )
            .allow_command_overrides(false),
        )
    }

    /// Create a stable provider-neutral execution id for one service run.
    ///
    /// The id is intentionally derived from run identity rather than app names
    /// or agent roles. Stage 2 will pass the same id through
    /// `service.execution_control` commands.
    fn execution_control_execution_id(command: &AgentExecutionCommand) -> String {
        match command.task_id {
            Some(task_id) => format!("{}:{}", command.session_id, task_id),
            None => format!("{}:{}", command.session_id, command.target_agent),
        }
    }

    /// Build the provider-neutral execution-control command scope.
    fn execution_control_scope(
        command: &AgentExecutionCommand,
        execution_id: String,
    ) -> ServiceResult<ExecutionControlScope> {
        let mut scope = ExecutionControlScope::new(
            command.application_id,
            command.session_id.clone(),
            execution_id,
            command.trace.clone(),
            "macaca.web.agent_execution",
        )
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        scope.task_id = command.task_id;
        scope
            .metadata
            .insert("target_agent".into(), command.target_agent.clone());
        scope.metadata.insert(
            "execution_intent".into(),
            format!("{:?}", command.execution_intent),
        );
        Ok(scope)
    }

    /// Register the execution with the stage-1 runtime capability before the
    /// Web adapter installs any local pause/resume channel wiring.
    async fn register_execution_control(
        &self,
        command: &AgentExecutionCommand,
        policy: ExecutionControlResolvedPolicy,
    ) -> ServiceResult<Option<(ExecutionControlPolicy, String)>> {
        if policy.status != ExecutionControlResolutionStatus::Enabled {
            return Ok(None);
        }
        let Some(enabled_policy) = policy.policy.clone() else {
            return Ok(None);
        };
        let execution_id = Self::execution_control_execution_id(command);
        let scope = Self::execution_control_scope(command, execution_id.clone())?;
        let service_command = ExecutionControlRegisterExecutionCommand { scope, policy }
            .into_service_command()
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        let reply = self
            .service_runtime
            .call(
                &KernelServiceId::new(EXECUTION_CONTROL_SERVICE_ID),
                ServiceBusSource::new("macaca.web.agent_execution"),
                service_command,
            )
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        if !reply.success {
            return Err(ServiceError::AdapterFailure(format!(
                "execution control service rejected registration: {}",
                reply.status
            )));
        }
        let output = reply.output.ok_or_else(|| {
            ServiceError::AdapterFailure(
                "execution control service returned no registration".into(),
            )
        })?;
        let result: ExecutionControlCommandResult =
            serde_json::from_value(output).map_err(|error| {
                ServiceError::AdapterFailure(format!(
                    "execution control registration decode failed: {error}"
                ))
            })?;
        emit_execution_control_events(
            &self.state,
            &command.session_id,
            "macaca.web.agent_execution",
            &result.events,
        )
        .await;
        Ok(Some((enabled_policy, execution_id)))
    }

    /// Attach this service execution to the active session's resume path.
    ///
    /// The Web shell creates the user-visible `ActiveSession` before calling the
    /// Agent Execution service. Because service commands are provider-neutral
    /// DTOs, the non-serializable receiver cannot travel inside the command. The
    /// host adapter therefore swaps in a fresh `resume_tx` for this local run and
    /// gives the paired receiver to the runtime execution-control adapter.
    async fn install_execution_control(
        &self,
        command: &AgentExecutionCommand,
        policy: ExecutionControlPolicy,
        execution_id: String,
    ) -> Option<RuntimeExecutionControl> {
        if policy.triggers.is_empty() {
            return None;
        }

        let (resume_tx, resume_rx) = mpsc::channel(4);
        let mut sessions = self.state.sessions.active_sessions.write().await;
        let session = match sessions.get_mut(&command.session_id) {
            Some(session) => session,
            None => {
                tracing::warn!(
                    session_id = %command.session_id,
                    target_agent = %command.target_agent,
                    "execution control requested without an active session"
                );
                return None;
            }
        };
        session.resume_tx = resume_tx;
        Some(RuntimeExecutionControl::new(
            Arc::clone(&session.pause_signal),
            resume_rx,
            policy,
            execution_id,
        ))
    }

    fn failed_result(
        command: &AgentExecutionCommand,
        task_id: TaskId,
        error: impl Into<String>,
        context_snapshot: Option<AgentContextSnapshot>,
    ) -> AgentExecutionResult {
        let error = error.into();
        AgentExecutionResult {
            application_id: command.application_id,
            session_id: command.session_id.clone(),
            task_id: Some(task_id),
            target_agent: command.target_agent.clone(),
            status: AgentExecutionStatus::Failed,
            output: serde_json::json!({ "error": error }),
            context_snapshot,
            trace: command.trace.clone(),
            metadata: Default::default(),
        }
    }
}

#[async_trait]
impl AgentExecutionBackend for WebAgentExecutionBackend {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
        let task_id = command.task_id.unwrap_or_else(TaskId::new);
        let context_snapshot = self.build_context_snapshot(&command).await?;
        if Self::should_skip_heartbeat_without_source(&command, &context_snapshot) {
            tracing::warn!(
                trace_id = %command.trace.trace_id,
                app_id = %command.application_id,
                target_agent = %command.target_agent,
                "agent execution heartbeat intent skipped because HEARTBEAT.md source evidence is absent"
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
        let executor = self
            .state
            .executor_registry
            .get(&command.application_id)
            .await;
        let event_factory = ExecutorEventFactory::new(task_id, command.target_agent.clone());
        let emit_lifecycle = Self::should_emit_executor_lifecycle(&command);

        if emit_lifecycle {
            if let Some(executor) = executor.as_ref() {
                executor.broadcast_event(event_factory.started());
            }
        }

        let expected_artifact_path = command
            .metadata
            .get("evidence.expected_artifact_path")
            .map(String::as_str);
        let (agent_event_tx, evidence_observer) = observed_agent_execution_events(
            executor.clone(),
            task_id,
            command.target_agent.clone(),
            expected_artifact_path,
        );

        let control_policy = self
            .resolve_execution_control_policy_via_service(&command)
            .await?;
        let execution_control_registration = self
            .register_execution_control(&command, control_policy)
            .await?;
        let execution_control = match execution_control_registration {
            Some((policy, execution_id)) => {
                self.install_execution_control(&command, policy, execution_id)
                    .await
            }
            None => None,
        };
        let agent = match execution_control {
            Some(execution_control) => {
                FrameworkRunner::build_runtime_agent_from_context_snapshot_with_execution_control(
                    &self.state,
                    &context_snapshot,
                    Some(agent_event_tx),
                    execution_control,
                )
                .await
            }
            None => {
                FrameworkRunner::build_runtime_agent_from_context_snapshot(
                    &self.state,
                    &context_snapshot,
                    Some(agent_event_tx),
                )
                .await
            }
        }
        .map_err(ServiceError::AdapterFailure)?;

        let prompt = Self::user_prompt_with_context(&command);
        match agent.reply(Msg::user("user", prompt)).await {
            Ok(reply) => {
                let output = reply.get_text();
                let task_result = event_factory.success_result(output.clone());
                if emit_lifecycle {
                    if let Some(executor) = executor.as_ref() {
                        executor.broadcast_event(event_factory.completed_with_result(task_result));
                    }
                }
                let mut result = AgentExecutionResult::completed(
                    &AgentExecutionCommand {
                        task_id: Some(task_id),
                        ..command.clone()
                    },
                    serde_json::json!({ "output": output }),
                );
                result.context_snapshot = Some(context_snapshot);
                result.metadata.insert(
                    "result_output_hash".into(),
                    stable_agent_output_hash(&result.output),
                );
                drop(agent);
                result.metadata.extend(evidence_observer.finish().await);
                Ok(result)
            }
            Err(error) => {
                let error = error.to_string();
                if emit_lifecycle {
                    if let Some(executor) = executor.as_ref() {
                        executor.broadcast_event(event_factory.failed(error.clone()));
                    }
                }
                Ok(Self::failed_result(
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
mod tests;
