//! Web adapters for runtime-host `ComposedAgentExecutionBackend`.
//!
//! The Web shell injects framework materialization and session-local lifecycle
//! wiring through these ports so `service.agent_execution` semantics remain
//! owned by runtime-host per protocol microkernel governance.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_host_composition::agent_execution::{
    execution_control_execution_id, execution_control_scope, AgentExecutionEvidenceCollector,
    AgentExecutionHostAdapter, AgentExecutionOutputHasher, ComposedAgentExecutionBackend,
    RuntimeHostFrameworkAgentConstructionService, ServiceBackedFrameworkRuntimeAgentPort,
};
use macaca_host_composition::execution_control::{
    ExecutionControlLocalNotification, OpaqueExecutionControlHandle,
};
use macaca_host_composition::executor::{ApplicationExecutor, ExecutorEventFactory};
use macaca_proto::{
    AgentActivity, AgentExecutionCommand, AgentExecutionEvent, ApplicationMetadataQueryCommand,
    ExecutionControlCommandResult, ExecutionControlPolicy,
    ExecutionControlRegisterExecutionCommand, ExecutionControlResolutionStatus,
    ExecutionControlResolvePolicyCommand, ExecutionControlResolvedPolicy, KernelServiceId,
    ServiceBusSource, ServiceError, ServiceResult, TaskId, EXECUTION_CONTROL_SERVICE_ID,
};
use tokio::sync::mpsc;

use crate::agent_execution_evidence::{observed_agent_execution_events, stable_agent_output_hash};
use crate::application_execution_agent_event_bridge::ApplicationExecutionAgentEventMirror;
use crate::application_shell_adapter::resolve_app_scoped_agent_manifest;
use crate::framework_agent_construction_shell_adapter::WebFrameworkAgentMaterializationPort;
use crate::framework_runner::RuntimeExecutionControl;
use crate::runtime_event_bridge::emit_execution_control_events;
use crate::state::AppState;

const WEB_AGENT_EXECUTION_BUS_SOURCE: &str = "macaca.web.agent_execution";

/// Host port that wires Web session state, kernel lifecycle, and execution-control services.
pub(crate) struct WebAgentExecutionHostAdapter {
    state: Arc<AppState>,
    service_runtime: Arc<macaca_host_composition::service_runtime::ServiceRuntime>,
}

impl WebAgentExecutionHostAdapter {
    pub(crate) fn new(
        state: Arc<AppState>,
        service_runtime: Arc<macaca_host_composition::service_runtime::ServiceRuntime>,
    ) -> Self {
        Self {
            state,
            service_runtime,
        }
    }

    async fn executor_for_app(
        &self,
        application_id: &macaca_proto::ApplicationId,
    ) -> Option<Arc<ApplicationExecutor>> {
        self.state.executor_registry.get(application_id).await
    }

    async fn update_kernel_agent_activity(
        &self,
        command: &AgentExecutionCommand,
        activity: AgentActivity,
    ) {
        if let Some(manifest) = resolve_app_scoped_agent_manifest(&self.state, command).await {
            let agent_id = manifest.id;
            self.state
                .kernel
                .update_agent_activity(&agent_id, activity)
                .await;
            tracing::info!(
                application_id = %command.application_id,
                session_id = %command.session_id,
                target_agent = %command.target_agent,
                agent_id = %agent_id.0,
                "agent execution updated app-scoped kernel activity"
            );
        } else {
            tracing::warn!(
                application_id = %command.application_id,
                session_id = %command.session_id,
                target_agent = %command.target_agent,
                "agent execution could not update kernel activity because target agent is not registered"
            );
        }
    }
}

#[async_trait]
impl AgentExecutionHostAdapter for WebAgentExecutionHostAdapter {
    async fn resolve_execution_control_policy(
        &self,
        command: &AgentExecutionCommand,
    ) -> ServiceResult<ExecutionControlResolvedPolicy> {
        // Adapter pattern: resolve application defaults from Application Service
        // metadata projection instead of direct registry reads or OS-level intent
        // branches. Missing metadata yields `None`, which disables execution
        // control unless an explicit command override is later allowed by policy.
        let app_default = match ApplicationMetadataQueryCommand::application(
            command.trace.clone(),
            command.application_id,
        ) {
            Ok(metadata_command) => match self
                .state
                .application_client
                .metadata(metadata_command)
                .await
            {
                Ok(view) => view.execution_control,
                Err(error) => {
                    tracing::warn!(
                        application_id = %command.application_id,
                        session_id = %command.session_id,
                        error = %error,
                        "application metadata query failed while resolving execution-control policy"
                    );
                    None
                }
            },
            Err(error) => {
                tracing::warn!(
                    application_id = %command.application_id,
                    session_id = %command.session_id,
                    error = %error,
                    "application metadata command rejected while resolving execution-control policy"
                );
                None
            }
        };
        tracing::info!(
            application_id = %command.application_id,
            session_id = %command.session_id,
            target_agent = %command.target_agent,
            has_manifest_execution_control = app_default.is_some(),
            has_command_override = command.execution_control_override.is_some(),
            "resolving execution-control policy from manifest projection"
        );
        let execution_id = execution_control_execution_id(command);
        let scope = execution_control_scope(command, execution_id, WEB_AGENT_EXECUTION_BUS_SOURCE)?;
        let service_command = ExecutionControlResolvePolicyCommand {
            scope,
            app_default,
            command_override: command.execution_control_override.clone(),
        }
        .into_service_command()
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        let reply = self
            .service_runtime
            .call(
                &KernelServiceId::new(EXECUTION_CONTROL_SERVICE_ID),
                ServiceBusSource::new(WEB_AGENT_EXECUTION_BUS_SOURCE),
                service_command,
            )
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        let output = reply.output.ok_or_else(|| {
            ServiceError::AdapterFailure("execution control service returned no policy".into())
        })?;
        let resolution: ExecutionControlResolvedPolicy =
            serde_json::from_value(output).map_err(|error| {
                ServiceError::AdapterFailure(format!(
                    "execution control policy decode failed: {error}"
                ))
            })?;
        emit_execution_control_events(
            &self.state,
            &command.session_id,
            WEB_AGENT_EXECUTION_BUS_SOURCE,
            &resolution.events,
        )
        .await;
        Ok(resolution)
    }

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
        let execution_id = execution_control_execution_id(command);
        let scope = execution_control_scope(
            command,
            execution_id.clone(),
            WEB_AGENT_EXECUTION_BUS_SOURCE,
        )?;
        let service_command = ExecutionControlRegisterExecutionCommand { scope, policy }
            .into_service_command()
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        let reply = self
            .service_runtime
            .call(
                &KernelServiceId::new(EXECUTION_CONTROL_SERVICE_ID),
                ServiceBusSource::new(WEB_AGENT_EXECUTION_BUS_SOURCE),
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
            WEB_AGENT_EXECUTION_BUS_SOURCE,
            &result.events,
        )
        .await;
        Ok(Some((enabled_policy, execution_id)))
    }

    async fn install_execution_control(
        &self,
        command: &AgentExecutionCommand,
        policy: ExecutionControlPolicy,
        execution_id: String,
    ) -> Option<OpaqueExecutionControlHandle> {
        if policy.triggers.is_empty() {
            return None;
        }
        if !self
            .state
            .sessions
            .active_sessions
            .read()
            .await
            .contains_key(&command.session_id)
        {
            tracing::warn!(
                session_id = %command.session_id,
                target_agent = %command.target_agent,
                "execution control requested without an active session"
            );
            return None;
        }

        let (resume_tx, resume_rx) = mpsc::channel(4);
        let pause_signal = Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.state
            .sessions
            .execution_control_local_notifications
            .install(
                command.session_id.clone(),
                ExecutionControlLocalNotification {
                    pause_signal: Arc::clone(&pause_signal),
                    resume_tx,
                },
            )
            .await;
        tracing::info!(
            session_id = %command.session_id,
            execution_id = %execution_id,
            "Installed runtime-host execution-control local notification handle"
        );
        let control = RuntimeExecutionControl::new(pause_signal, resume_rx, policy, execution_id);
        Some(OpaqueExecutionControlHandle(Arc::new(control)))
    }

    async fn on_run_started(
        &self,
        command: &AgentExecutionCommand,
        task_id: TaskId,
        emit_lifecycle: bool,
    ) {
        if emit_lifecycle {
            if let Some(executor) = self.executor_for_app(&command.application_id).await {
                let event_factory =
                    ExecutorEventFactory::new(task_id, command.target_agent.clone());
                executor.broadcast_event(event_factory.started());
            }
        }
        if emit_lifecycle {
            self.update_kernel_agent_activity(
                command,
                AgentActivity::Working {
                    context: format!("Executing delegated task {}", task_id),
                },
            )
            .await;
        }
    }

    async fn on_run_completed(
        &self,
        command: &AgentExecutionCommand,
        task_id: TaskId,
        emit_lifecycle: bool,
        output_preview: &str,
    ) {
        if emit_lifecycle {
            if let Some(executor) = self.executor_for_app(&command.application_id).await {
                let event_factory =
                    ExecutorEventFactory::new(task_id, command.target_agent.clone());
                let task_result = event_factory.success_result(output_preview.to_string());
                executor.broadcast_event(event_factory.completed_with_result(task_result));
            }
        }
        if emit_lifecycle {
            self.update_kernel_agent_activity(command, AgentActivity::Idle)
                .await;
        }
    }

    async fn on_run_failed(
        &self,
        command: &AgentExecutionCommand,
        task_id: TaskId,
        emit_lifecycle: bool,
        error: &str,
    ) {
        if emit_lifecycle {
            if let Some(executor) = self.executor_for_app(&command.application_id).await {
                let event_factory =
                    ExecutorEventFactory::new(task_id, command.target_agent.clone());
                executor.broadcast_event(event_factory.failed(error.to_string()));
            }
        }
        if emit_lifecycle {
            self.update_kernel_agent_activity(
                command,
                AgentActivity::Error {
                    message: error.to_string(),
                },
            )
            .await;
        }
    }

    async fn observe_application_execution_requested(&self, command: &AgentExecutionCommand) {
        let mirror = ApplicationExecutionAgentEventMirror::from_command(
            self.state.system_facade.application_execution_client(),
            command,
        );
        if let Some(mirror) = mirror.as_ref() {
            mirror.observe_requested(command).await;
        }
    }

    async fn create_evidence_observer(
        &self,
        command: &AgentExecutionCommand,
        task_id: TaskId,
    ) -> (
        mpsc::Sender<AgentExecutionEvent>,
        Box<dyn AgentExecutionEvidenceCollector>,
    ) {
        let expected_artifact_path = command
            .metadata
            .get("evidence.expected_artifact_path")
            .map(String::as_str);
        let application_execution_mirror = ApplicationExecutionAgentEventMirror::from_command(
            self.state.system_facade.application_execution_client(),
            command,
        );
        let executor = self.executor_for_app(&command.application_id).await;
        let (tx, observer) = observed_agent_execution_events(
            executor,
            task_id,
            command.target_agent.clone(),
            expected_artifact_path,
            application_execution_mirror,
        );
        struct WebEvidenceCollector {
            inner: crate::agent_execution_evidence::AgentExecutionEvidenceObserver,
        }
        #[async_trait]
        impl AgentExecutionEvidenceCollector for WebEvidenceCollector {
            async fn finish(self: Box<Self>) -> BTreeMap<String, String> {
                self.inner.finish().await
            }
        }
        (tx, Box::new(WebEvidenceCollector { inner: observer }))
    }
}

/// Web output hasher used for audit replay metadata.
pub(crate) struct WebAgentExecutionOutputHasher;

impl AgentExecutionOutputHasher for WebAgentExecutionOutputHasher {
    fn stable_output_hash(&self, output: &serde_json::Value) -> String {
        stable_agent_output_hash(output)
    }
}

/// Build the Web-wired composed agent execution backend for service registration.
pub(crate) fn build_composed_web_agent_execution_backend(
    state: Arc<AppState>,
    service_runtime: Arc<macaca_host_composition::service_runtime::ServiceRuntime>,
) -> ComposedAgentExecutionBackend {
    ComposedAgentExecutionBackend::new(
        service_runtime,
        WEB_AGENT_EXECUTION_BUS_SOURCE,
        Arc::new(WebAgentExecutionHostAdapter::new(
            Arc::clone(&state),
            Arc::clone(&state.service_runtime),
        )),
        Arc::new(ServiceBackedFrameworkRuntimeAgentPort::new(
            Arc::new(RuntimeHostFrameworkAgentConstructionService::new(
                Arc::new(WebFrameworkAgentMaterializationPort::new(Arc::clone(
                    &state,
                ))),
                WEB_AGENT_EXECUTION_BUS_SOURCE,
            )),
            WEB_AGENT_EXECUTION_BUS_SOURCE,
        )),
        Arc::new(WebAgentExecutionOutputHasher),
    )
}
