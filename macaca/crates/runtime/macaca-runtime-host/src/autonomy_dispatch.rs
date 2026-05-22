//! Provider-neutral dispatch strategies used by the autonomy supervisor.
//!
//! Scheduler owns durable run state, but it must not execute target commands
//! directly.  This module keeps dispatch as runtime-host strategy code that
//! routes through existing service boundaries and records only safe status
//! classes.  Unsupported target categories are explicit skipped outcomes rather
//! than panics or fake success.

use std::time::Duration;

use macaca_proto::{
    AgentExecutionCommand, AgentExecutionResult, AgentExecutionStatus, AgentExecutionTargetCommand,
    AutonomousExecutionEnvelope, AutonomousExecutionSourceKind, AutonomyScope,
    HeartbeatWakeCommand, HeartbeatWakeIntent, KernelServiceId, MacacaResult,
    ResolveScheduledAgentTaskPayloadCommand, ScheduledAgentTaskResolvedPayload,
    SchedulerTargetCommand, ServiceBusSource, ServiceCommand, ServiceCommandName, TraceContext,
    AGENT_EXECUTION_SERVICE_ID, HEARTBEAT_SERVICE_ID, SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND,
    SCHEDULED_AGENT_TASK_SERVICE_ID,
};
use tokio::time::timeout;
use tracing::{info, warn};

use crate::autonomy_result_evidence::{AgentExecutionEvidenceDecision, AgentExecutionEvidenceGate};
use crate::ServiceRuntime;

/// Safe result class returned to Scheduler run-control transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomyDispatchOutcome {
    pub succeeded: bool,
    pub retryable: bool,
    pub reason_code: &'static str,
}

impl AutonomyDispatchOutcome {
    /// Build a successful dispatch outcome.
    pub fn succeeded() -> Self {
        Self {
            succeeded: true,
            retryable: false,
            reason_code: "dispatch_succeeded",
        }
    }

    /// Build a bounded failure outcome that Scheduler may retry.
    pub fn retryable(reason_code: &'static str) -> Self {
        Self {
            succeeded: false,
            retryable: true,
            reason_code,
        }
    }

    /// Build a final skipped outcome for unsupported generic categories.
    pub fn skipped(reason_code: &'static str) -> Self {
        Self {
            succeeded: false,
            retryable: false,
            reason_code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use macaca_kernel::SystemService;
    use macaca_proto::{
        AgentExecutionIntent, ApplicationId, AutonomyPayloadRef, CleanupPolicy,
        ScheduledAgentTaskId, ServiceCallResult, ServiceDescriptor, ServiceError, ServiceHealth,
        ServiceResult, ServiceScope, ServiceType, TaskId, TraceSchemaRef,
    };

    use crate::agent_execution_service_provider::{
        AgentExecutionBackend, AgentExecutionSystemServiceProvider,
    };
    use crate::{
        ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntimeConfig,
        StaticServiceProviderFactory,
    };

    /// Test double that records the command crossing into `service.agent_execution`.
    ///
    /// The double keeps the runtime test at the service boundary.  It does not
    /// execute tools, inspect application-specific names, or simulate provider
    /// internals; it only proves the Strategy builds the audited command shape
    /// that Agent Execution owns.
    #[derive(Default)]
    struct RecordingExecutionBackend {
        commands: Mutex<Vec<AgentExecutionCommand>>,
        emit_evidence: bool,
        emit_output_hash: bool,
    }

    #[async_trait]
    impl AgentExecutionBackend for RecordingExecutionBackend {
        async fn execute(
            &self,
            command: AgentExecutionCommand,
        ) -> ServiceResult<AgentExecutionResult> {
            self.commands.lock().unwrap().push(command.clone());
            let mut result =
                AgentExecutionResult::completed(&command, serde_json::json!({"accepted": true}));
            if self.emit_evidence {
                result
                    .metadata
                    .insert("result_evidence_ref".into(), "event/agent-result/1".into());
            }
            if self.emit_output_hash {
                result
                    .metadata
                    .insert("result_output_hash".into(), "hash.agent-output.1".into());
            }
            Ok(result)
        }
    }

    /// ServiceRuntime-facing payload resolver used by dispatch Strategy tests.
    ///
    /// The real Scheduled Agent Task service owns the payload Memento.  This fake
    /// implements only the trusted resolve command so the test can assert that
    /// Runtime Host does not read raw prompts from Scheduler target metadata.
    struct FakeScheduledAgentTaskResolver {
        resolved: ScheduledAgentTaskResolvedPayload,
    }

    impl FakeScheduledAgentTaskResolver {
        fn descriptor() -> ServiceDescriptor {
            let mut descriptor = ServiceDescriptor::new(
                KernelServiceId::new(SCHEDULED_AGENT_TASK_SERVICE_ID),
                ServiceType::new("autonomy.scheduled_agent_task"),
                TraceSchemaRef::new("trace.schema.scheduled_agent_task.test"),
            );
            descriptor.health = ServiceHealth::Healthy;
            descriptor.supported_scopes = vec![ServiceScope::Application("*".into())];
            descriptor
        }
    }

    #[async_trait]
    impl SystemService for FakeScheduledAgentTaskResolver {
        fn descriptor(&self) -> ServiceDescriptor {
            Self::descriptor()
        }

        async fn start(&self) -> ServiceResult<()> {
            Ok(())
        }

        async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
            let trace = command
                .trace
                .clone()
                .ok_or(ServiceError::MissingTraceContext)?;
            if command.name.as_str() != SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND {
                return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
            }
            let _: ResolveScheduledAgentTaskPayloadCommand =
                serde_json::from_value(command.payload)
                    .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
            Ok(ServiceCallResult {
                output: serde_json::to_value(Some(self.resolved.clone()))
                    .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
                trace,
                status: "resolved".into(),
                metadata: BTreeMap::new(),
                cleanup_hint: Some(CleanupPolicy::None),
            })
        }

        async fn stop(&self) -> ServiceResult<()> {
            Ok(())
        }

        async fn cleanup(&self) -> ServiceResult<()> {
            Ok(())
        }

        async fn health(&self) -> ServiceResult<ServiceHealth> {
            Ok(ServiceHealth::Healthy)
        }
    }

    async fn register_static_service(
        runtime: &ServiceRuntime,
        descriptor: ServiceDescriptor,
        service: Arc<dyn SystemService>,
    ) {
        let factory =
            StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service));
        runtime
            .register_provider(&factory, ServiceProviderFactoryContext::new())
            .await
            .unwrap();
    }

    fn payload_ref() -> AutonomyPayloadRef {
        let mut payload_ref = AutonomyPayloadRef::new(
            "scheduled-agent-task://payload/test",
            "Analyze the market and record result.",
        )
        .unwrap();
        payload_ref.content_digest = Some("digest.prompt.123".into());
        payload_ref
    }

    fn resolved_payload(payload_ref: AutonomyPayloadRef) -> ScheduledAgentTaskResolvedPayload {
        ScheduledAgentTaskResolvedPayload {
            task_id: ScheduledAgentTaskId::new("scheduled-agent-task-test").unwrap(),
            application_id: ApplicationId::from_name("scheduled-agent-dispatch-test"),
            session_id: "session-scheduled-agent".into(),
            task_ref: Some(TaskId::new()),
            target_agent: "worker".into(),
            execution_intent: AgentExecutionIntent::TaskWorker,
            user_prompt: "Analyze the market and record result.".into(),
            delegated_context: serde_json::json!({"bounded": true}),
            policy: Default::default(),
            payload_ref,
            payload_digest: Some("digest.prompt.123".into()),
            trace: TraceContext::new("trace-resolved-payload"),
            audit_id: Some("audit.scheduled_agent_task.created.1".into()),
            metadata: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn agent_execution_target_resolves_payload_and_invokes_agent_execution_service() {
        let runtime = ServiceRuntime::new(ServiceRuntimeConfig::default());
        let payload_ref = payload_ref();
        let backend = Arc::new(RecordingExecutionBackend {
            emit_evidence: true,
            ..RecordingExecutionBackend::default()
        });

        register_static_service(
            &runtime,
            FakeScheduledAgentTaskResolver::descriptor(),
            Arc::new(FakeScheduledAgentTaskResolver {
                resolved: resolved_payload(payload_ref.clone()),
            }),
        )
        .await;
        register_static_service(
            &runtime,
            AgentExecutionSystemServiceProvider::new(backend.clone()).descriptor(),
            Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
        )
        .await;

        let target = AgentExecutionTargetCommand {
            application_id: ApplicationId::from_name("scheduled-agent-dispatch-test"),
            session_id: "session-scheduled-agent".into(),
            task_id: Some(TaskId::new()),
            target_agent: Some("worker".into()),
            execution_intent: AgentExecutionIntent::TaskWorker,
            payload_ref,
            metadata: BTreeMap::new(),
        };
        let dispatcher = AutonomyDispatchStrategies::new(&runtime, 1_000);
        let outcome = dispatcher
            .dispatch(
                TraceContext::new("trace-scheduled-agent-dispatch"),
                AutonomyScope::application(ApplicationId::from_name(
                    "scheduled-agent-dispatch-test",
                )),
                SchedulerTargetCommand::AgentExecution(target),
            )
            .await
            .unwrap();

        assert!(outcome.succeeded);
        let commands = backend.commands.lock().unwrap();
        assert_eq!(commands.len(), 1);
        let command = &commands[0];
        assert_eq!(command.execution_intent, AgentExecutionIntent::TaskWorker);
        assert_eq!(command.user_prompt, "Analyze the market and record result.");
        assert_eq!(
            command.metadata["scheduler_run_source"],
            "service.scheduler"
        );
        assert_eq!(command.metadata["payload_digest"], "digest.prompt.123");
        assert_eq!(
            command.metadata["scheduled_agent_task_audit_id"],
            "audit.scheduled_agent_task.created.1"
        );
        let envelope = command
            .execution_envelope
            .as_ref()
            .expect("scheduled dispatch must attach an execution envelope");
        assert_eq!(
            envelope.source_kind,
            macaca_proto::AutonomousExecutionSourceKind::ScheduledAgentTask
        );
        assert_eq!(
            envelope.source_instruction,
            "Analyze the market and record result."
        );
    }

    #[tokio::test]
    async fn completed_agent_execution_without_result_evidence_is_retryable() {
        let runtime = ServiceRuntime::new(ServiceRuntimeConfig::default());
        let payload_ref = payload_ref();
        let backend = Arc::new(RecordingExecutionBackend::default());

        register_static_service(
            &runtime,
            FakeScheduledAgentTaskResolver::descriptor(),
            Arc::new(FakeScheduledAgentTaskResolver {
                resolved: resolved_payload(payload_ref.clone()),
            }),
        )
        .await;
        register_static_service(
            &runtime,
            AgentExecutionSystemServiceProvider::new(backend.clone()).descriptor(),
            Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
        )
        .await;

        let target = AgentExecutionTargetCommand {
            application_id: ApplicationId::from_name("scheduled-agent-dispatch-test"),
            session_id: "session-scheduled-agent".into(),
            task_id: Some(TaskId::new()),
            target_agent: Some("worker".into()),
            execution_intent: AgentExecutionIntent::TaskWorker,
            payload_ref,
            metadata: BTreeMap::new(),
        };
        let dispatcher = AutonomyDispatchStrategies::new(&runtime, 1_000);
        let outcome = dispatcher
            .dispatch(
                TraceContext::new("trace-scheduled-agent-dispatch-missing-evidence"),
                AutonomyScope::application(ApplicationId::from_name(
                    "scheduled-agent-dispatch-test",
                )),
                SchedulerTargetCommand::AgentExecution(target),
            )
            .await
            .unwrap();

        assert!(!outcome.succeeded);
        assert!(outcome.retryable);
        assert_eq!(
            outcome.reason_code,
            "agent_execution_result_evidence_missing"
        );
    }

    #[tokio::test]
    async fn completed_agent_execution_with_result_hash_satisfies_agent_result_policy() {
        let runtime = ServiceRuntime::new(ServiceRuntimeConfig::default());
        let payload_ref = payload_ref();
        let backend = Arc::new(RecordingExecutionBackend {
            emit_output_hash: true,
            ..RecordingExecutionBackend::default()
        });

        register_static_service(
            &runtime,
            FakeScheduledAgentTaskResolver::descriptor(),
            Arc::new(FakeScheduledAgentTaskResolver {
                resolved: resolved_payload(payload_ref.clone()),
            }),
        )
        .await;
        register_static_service(
            &runtime,
            AgentExecutionSystemServiceProvider::new(backend.clone()).descriptor(),
            Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
        )
        .await;

        let target = AgentExecutionTargetCommand {
            application_id: ApplicationId::from_name("scheduled-agent-dispatch-test"),
            session_id: "session-scheduled-agent".into(),
            task_id: Some(TaskId::new()),
            target_agent: Some("worker".into()),
            execution_intent: AgentExecutionIntent::TaskWorker,
            payload_ref,
            metadata: BTreeMap::new(),
        };
        let dispatcher = AutonomyDispatchStrategies::new(&runtime, 1_000);
        let outcome = dispatcher
            .dispatch(
                TraceContext::new("trace-scheduled-agent-dispatch-output-hash"),
                AutonomyScope::application(ApplicationId::from_name(
                    "scheduled-agent-dispatch-test",
                )),
                SchedulerTargetCommand::AgentExecution(target),
            )
            .await
            .unwrap();

        assert!(outcome.succeeded);
        assert!(!outcome.retryable);
        assert_eq!(outcome.reason_code, "dispatch_succeeded");
    }
}

/// Strategy dispatcher for Scheduler target categories.
pub struct AutonomyDispatchStrategies<'a> {
    runtime: &'a ServiceRuntime,
    timeout_ms: u64,
}

impl<'a> AutonomyDispatchStrategies<'a> {
    /// Create a dispatcher over the existing service runtime boundary.
    pub fn new(runtime: &'a ServiceRuntime, timeout_ms: u64) -> Self {
        Self {
            runtime,
            timeout_ms: timeout_ms.max(1),
        }
    }

    /// Dispatch one provider-neutral target without inspecting business data.
    pub async fn dispatch(
        &self,
        trace: TraceContext,
        scope: AutonomyScope,
        target: SchedulerTargetCommand,
    ) -> MacacaResult<AutonomyDispatchOutcome> {
        match target {
            SchedulerTargetCommand::Service(command) => {
                self.dispatch_service(trace, command.service_id, command.command_name)
                    .await
            }
            SchedulerTargetCommand::HeartbeatWake(command) => {
                let mut wake = HeartbeatWakeCommand::new(
                    trace,
                    scope,
                    command.wake_scope_key,
                    HeartbeatWakeIntent::ScheduledTick,
                )?;
                wake.payload_ref = command.payload_ref;
                wake.metadata = command.metadata;
                self.dispatch_heartbeat(wake).await
            }
            SchedulerTargetCommand::AgentExecution(command) => {
                self.dispatch_agent_execution(trace, command).await
            }
            SchedulerTargetCommand::Application(_) => {
                warn!("autonomy supervisor skipped application target because application dispatch strategy is not active yet");
                Ok(AutonomyDispatchOutcome::skipped(
                    "application_strategy_unavailable",
                ))
            }
            SchedulerTargetCommand::Plugin(_) => {
                warn!("autonomy supervisor skipped plugin target because plugin dispatch strategy is not active yet");
                Ok(AutonomyDispatchOutcome::skipped(
                    "plugin_strategy_unavailable",
                ))
            }
        }
    }

    /// Dispatch a scheduled `AgentExecution` target through service boundaries.
    ///
    /// Scheduler stores only `AutonomyPayloadRef` and target metadata.  Runtime
    /// Host resolves the prompt through `service.scheduled_agent_task` at the
    /// dispatch boundary, then creates an `AgentExecutionCommand` for
    /// `service.agent_execution`.  This Strategy keeps Scheduler free of prompt
    /// interpretation and keeps LLM/tool execution owned by Agent Execution.
    async fn dispatch_agent_execution(
        &self,
        trace: TraceContext,
        target: AgentExecutionTargetCommand,
    ) -> MacacaResult<AutonomyDispatchOutcome> {
        let payload_digest = target.payload_ref.content_digest.clone();
        let resolved = self
            .resolve_scheduled_agent_payload(trace.clone(), &target)
            .await?;
        let Some(resolved) = resolved else {
            warn!(
                trace_id = trace.trace_id.as_str(),
                payload_digest = payload_digest.as_deref().unwrap_or("none"),
                "scheduled agent dispatch skipped because payload was unavailable"
            );
            return Ok(AutonomyDispatchOutcome::skipped(
                "scheduled_agent_payload_unavailable",
            ));
        };
        let mut command = AgentExecutionCommand::new(
            resolved.application_id,
            resolved.session_id.clone(),
            target
                .target_agent
                .clone()
                .unwrap_or_else(|| resolved.target_agent.clone()),
            resolved.execution_intent.clone(),
            resolved.user_prompt.clone(),
            trace.clone(),
        )?
        .with_delegated_context(resolved.delegated_context.clone());
        command.task_id = resolved.task_ref;
        command.policy = resolved.policy.clone();
        command.metadata = target.metadata.clone();
        for (key, value) in &resolved.metadata {
            if key.starts_with("evidence.") && !value.trim().is_empty() {
                command.metadata.insert(key.clone(), value.clone());
            }
        }
        command
            .metadata
            .insert("source".into(), "service.scheduler".into());
        command
            .metadata
            .insert("scheduler_run_source".into(), "service.scheduler".into());
        command.metadata.insert(
            "scheduled_agent_task_id".into(),
            resolved.task_id.as_str().into(),
        );
        if let Some(digest) = resolved.payload_digest.clone() {
            command.metadata.insert("payload_digest".into(), digest);
        }
        if let Some(audit_id) = resolved.audit_id.clone() {
            command
                .metadata
                .insert("scheduled_agent_task_audit_id".into(), audit_id);
        }
        let envelope = AutonomousExecutionEnvelope::compile(
            AutonomousExecutionSourceKind::ScheduledAgentTask,
            resolved.user_prompt.clone(),
            &command.metadata,
        )?;
        command.metadata.insert(
            "execution_envelope.source_kind".into(),
            envelope.source_kind.as_str().into(),
        );
        command.metadata.insert(
            "execution_envelope.completion_policy".into(),
            envelope.completion_policy.kind.as_str().into(),
        );

        info!(
            service_id = AGENT_EXECUTION_SERVICE_ID,
            target_agent = command.target_agent.as_str(),
            task_id = resolved.task_id.as_str(),
            source_kind = envelope.source_kind.as_str(),
            completion_policy = envelope.completion_policy.kind.as_str(),
            payload_digest = command
                .metadata
                .get("payload_digest")
                .map(String::as_str)
                .unwrap_or("none"),
            trace_id = trace.trace_id.as_str(),
            "scheduled agent dispatch invoking agent execution service"
        );
        command.execution_envelope = Some(envelope.clone());
        let service_command = command.into_service_command()?;
        match timeout(
            Duration::from_millis(self.timeout_ms),
            self.runtime.call(
                &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
                ServiceBusSource::new("runtime.autonomy_supervisor"),
                service_command,
            ),
        )
        .await
        {
            Ok(Ok(reply)) if reply.success => {
                let Some(output) = reply.output else {
                    return Ok(AutonomyDispatchOutcome::retryable(
                        "agent_execution_empty_reply",
                    ));
                };
                let result: AgentExecutionResult = match serde_json::from_value(output) {
                    Ok(result) => result,
                    Err(error) => {
                        warn!(
                            error = %error,
                            trace_id = trace.trace_id.as_str(),
                            "scheduled agent dispatch could not decode agent execution result"
                        );
                        return Ok(AutonomyDispatchOutcome::retryable(
                            "agent_execution_decode_failed",
                        ));
                    }
                };
                info!(
                    service_id = AGENT_EXECUTION_SERVICE_ID,
                    target_agent = result.target_agent.as_str(),
                    status = result.status.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "scheduled agent dispatch completed"
                );
                match result.status {
                    AgentExecutionStatus::Completed => {
                        match AgentExecutionEvidenceGate::evaluate_with_policy(
                            &result,
                            &envelope.completion_policy,
                        ) {
                            AgentExecutionEvidenceDecision::Verified { evidence_key } => {
                                info!(
                                    service_id = AGENT_EXECUTION_SERVICE_ID,
                                    target_agent = result.target_agent.as_str(),
                                    evidence_key,
                                    trace_id = trace.trace_id.as_str(),
                                    "scheduled agent dispatch result evidence verified"
                                );
                                Ok(AutonomyDispatchOutcome::succeeded())
                            }
                            AgentExecutionEvidenceDecision::MissingEvidence => {
                                warn!(
                                    service_id = AGENT_EXECUTION_SERVICE_ID,
                                    target_agent = result.target_agent.as_str(),
                                    trace_id = trace.trace_id.as_str(),
                                    "scheduled agent dispatch completed without result evidence"
                                );
                                Ok(AutonomyDispatchOutcome::retryable(
                                    "agent_execution_result_evidence_missing",
                                ))
                            }
                            AgentExecutionEvidenceDecision::NotCompleted => {
                                Ok(AutonomyDispatchOutcome::retryable(
                                    "agent_execution_result_not_completed",
                                ))
                            }
                        }
                    }
                    AgentExecutionStatus::Skipped | AgentExecutionStatus::Denied => {
                        Ok(AutonomyDispatchOutcome::skipped("agent_execution_skipped"))
                    }
                    AgentExecutionStatus::Unavailable
                    | AgentExecutionStatus::Unsupported
                    | AgentExecutionStatus::Failed => {
                        Ok(AutonomyDispatchOutcome::retryable("agent_execution_failed"))
                    }
                }
            }
            Ok(Ok(_)) => Ok(AutonomyDispatchOutcome::retryable(
                "agent_execution_reply_failed",
            )),
            Ok(Err(error)) => {
                warn!(
                    service_id = AGENT_EXECUTION_SERVICE_ID,
                    error = %error,
                    trace_id = trace.trace_id.as_str(),
                    "scheduled agent dispatch failed when calling agent execution service"
                );
                Ok(AutonomyDispatchOutcome::retryable(
                    "agent_execution_dispatch_failed",
                ))
            }
            Err(_) => Ok(AutonomyDispatchOutcome::retryable(
                "agent_execution_dispatch_timeout",
            )),
        }
    }

    async fn resolve_scheduled_agent_payload(
        &self,
        trace: TraceContext,
        target: &AgentExecutionTargetCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskResolvedPayload>> {
        let mut resolve = ResolveScheduledAgentTaskPayloadCommand::new(
            trace.clone(),
            target.payload_ref.clone(),
        )?;
        resolve.metadata = target.metadata.clone();
        let command = ServiceCommand::with_trace(
            ServiceCommandName::new(SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND),
            serde_json::to_value(resolve)?,
            trace.clone(),
        );
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            payload_digest = target
                .payload_ref
                .content_digest
                .as_deref()
                .unwrap_or("none"),
            trace_id = trace.trace_id.as_str(),
            "scheduled agent dispatch resolving payload reference"
        );
        match timeout(
            Duration::from_millis(self.timeout_ms),
            self.runtime.call(
                &KernelServiceId::new(SCHEDULED_AGENT_TASK_SERVICE_ID),
                ServiceBusSource::new("runtime.autonomy_supervisor"),
                command,
            ),
        )
        .await
        {
            Ok(Ok(reply)) if reply.success => {
                let Some(output) = reply.output else {
                    return Ok(None);
                };
                let resolved: Option<ScheduledAgentTaskResolvedPayload> =
                    serde_json::from_value(output)?;
                if let Some(resolved) = &resolved {
                    info!(
                        service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
                        task_id = resolved.task_id.as_str(),
                        payload_digest = resolved.payload_digest.as_deref().unwrap_or("none"),
                        trace_id = trace.trace_id.as_str(),
                        "scheduled agent dispatch payload resolved"
                    );
                }
                Ok(resolved)
            }
            Ok(Ok(_)) => Ok(None),
            Ok(Err(error)) => {
                warn!(
                    service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
                    error = %error,
                    trace_id = trace.trace_id.as_str(),
                    "scheduled agent dispatch payload resolution failed"
                );
                Ok(None)
            }
            Err(_) => Ok(None),
        }
    }

    async fn dispatch_service(
        &self,
        trace: TraceContext,
        service_id: KernelServiceId,
        command_name: ServiceCommandName,
    ) -> MacacaResult<AutonomyDispatchOutcome> {
        info!(
            service_id = %service_id,
            command = %command_name,
            trace_id = trace.trace_id.as_str(),
            "autonomy supervisor dispatching service command through ServiceRuntime"
        );
        let command = ServiceCommand::with_trace(
            command_name,
            serde_json::json!({"payload_ref": null}),
            trace,
        );
        let result = timeout(
            Duration::from_millis(self.timeout_ms),
            self.runtime.call(
                &service_id,
                ServiceBusSource::new("runtime.autonomy_supervisor"),
                command,
            ),
        )
        .await;
        match result {
            Ok(Ok(reply)) if reply.success => Ok(AutonomyDispatchOutcome::succeeded()),
            Ok(Ok(_)) => Ok(AutonomyDispatchOutcome::retryable("service_reply_failed")),
            Ok(Err(error)) => {
                warn!(
                    service_id = %service_id,
                    error = %error,
                    "autonomy supervisor service dispatch failed"
                );
                Ok(AutonomyDispatchOutcome::retryable(
                    "service_dispatch_failed",
                ))
            }
            Err(_) => Ok(AutonomyDispatchOutcome::retryable(
                "service_dispatch_timeout",
            )),
        }
    }

    async fn dispatch_heartbeat(
        &self,
        wake: HeartbeatWakeCommand,
    ) -> MacacaResult<AutonomyDispatchOutcome> {
        let trace_id = wake.trace.trace_id.clone();
        let command = wake.into_service_command()?;
        let service_id = KernelServiceId::new(HEARTBEAT_SERVICE_ID);
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            trace_id = trace_id.as_str(),
            "autonomy supervisor dispatching heartbeat wake through ServiceRuntime"
        );
        match timeout(
            Duration::from_millis(self.timeout_ms),
            self.runtime.call(
                &service_id,
                ServiceBusSource::new("runtime.autonomy_supervisor"),
                command,
            ),
        )
        .await
        {
            Ok(Ok(reply)) if reply.success => Ok(AutonomyDispatchOutcome::succeeded()),
            Ok(Ok(_)) => Ok(AutonomyDispatchOutcome::retryable("heartbeat_reply_failed")),
            Ok(Err(error)) => {
                warn!(
                    service_id = HEARTBEAT_SERVICE_ID,
                    error = %error,
                    "autonomy supervisor heartbeat dispatch failed"
                );
                Ok(AutonomyDispatchOutcome::retryable(
                    "heartbeat_dispatch_failed",
                ))
            }
            Err(_) => Ok(AutonomyDispatchOutcome::retryable(
                "heartbeat_dispatch_timeout",
            )),
        }
    }
}
