//! Runtime-host provider for `service.application_execution`.
//!
//! This module is the first host-owned Adapter for the application execution
//! protocol platform.  It intentionally starts as a Null Object provider: the
//! service descriptor, lifecycle, trace-required calls, logging, and structured
//! unavailable results exist before concrete hosted/external/remote execution
//! strategies are enabled.  Later provider strategies plug in behind this
//! service boundary without moving application behavior into Web, CLI, SDK, or
//! the kernel.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_persist::EventLog;
use macaca_proto::{
    application_execution_service_descriptor, AppendExecutionEventCommand,
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlResult, ApplicationExecutionEventEnvelope,
    ApplicationExecutionEventType, ApplicationExecutionPayload, ApplicationExecutionProviderKind,
    ApplicationExecutionReplayRequest, ApplicationExecutionReplayResult, ApplicationExecutionScope,
    CleanupPolicy, MacacaError, ReportExecutionCompletionCommand, ReportExecutionFailureCommand,
    ReportExecutionHeartbeatCommand, RequestExecutionApprovalCommand, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    StartApplicationExecutionCommand, StartApplicationExecutionResult, TraceContext,
    APPLICATION_EXECUTION_CONTROL_COMMAND, APPLICATION_EXECUTION_CURRENT_STATE_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_APPROVAL_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_COMPLETION_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_FAILURE_COMMAND, APPLICATION_EXECUTION_GATEWAY_HEARTBEAT_COMMAND,
    APPLICATION_EXECUTION_PROVIDER_HEALTH_COMMAND, APPLICATION_EXECUTION_REPLAY_COMMAND,
    APPLICATION_EXECUTION_SERVICE_ID, APPLICATION_EXECUTION_SNAPSHOT_COMMAND,
    APPLICATION_EXECUTION_START_COMMAND,
};
use tracing::{info, warn};

use crate::application_execution_event_store::ApplicationExecutionEventStore;
use crate::application_execution_provider_registry::ApplicationExecutionProviderRegistry;

/// Host-owned application execution service provider.
pub struct ApplicationExecutionSystemServiceProvider {
    descriptor: ServiceDescriptor,
    unavailable_reason: Option<String>,
    event_store: Option<ApplicationExecutionEventStore>,
    provider_registry: ApplicationExecutionProviderRegistry,
}

impl ApplicationExecutionSystemServiceProvider {
    /// Build a Null Object provider that exposes the service but rejects work.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            descriptor: application_execution_service_descriptor(),
            unavailable_reason: Some(reason.into()),
            event_store: None,
            provider_registry: ApplicationExecutionProviderRegistry::new(),
        }
    }

    /// Build a configured service backed by EventLog and provider strategies.
    pub fn with_event_log(
        event_log: Arc<EventLog>,
        provider_registry: ApplicationExecutionProviderRegistry,
    ) -> Self {
        Self {
            descriptor: application_execution_service_descriptor(),
            unavailable_reason: None,
            event_store: Some(ApplicationExecutionEventStore::new(event_log)),
            provider_registry,
        }
    }

    fn reason(&self) -> String {
        self.unavailable_reason
            .clone()
            .unwrap_or_else(|| "application execution provider stack is not configured".into())
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn service_result<T: serde::Serialize>(
        output: T,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Ok(ServiceCallResult {
            output: serde_json::to_value(output)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "unavailable".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    fn unavailable_start(&self) -> StartApplicationExecutionResult {
        StartApplicationExecutionResult::unavailable(
            APPLICATION_EXECUTION_START_COMMAND,
            self.reason(),
        )
    }

    fn event_store(&self) -> ServiceResult<&ApplicationExecutionEventStore> {
        self.event_store
            .as_ref()
            .ok_or_else(|| ServiceError::ServiceUnavailable(self.reason()))
    }

    async fn handle_start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> ServiceResult<StartApplicationExecutionResult> {
        let store = self.event_store()?;
        let selection = self
            .provider_registry
            .select(
                command.provider_preference.as_ref(),
                &command.requested_capabilities,
            )
            .map_err(|error| ServiceError::ServiceUnavailable(error.reason))?;
        let session_id = command
            .session_id
            .clone()
            .unwrap_or_else(|| format!("session-{}", command.idempotency_key));
        let run_id = command
            .run_id
            .clone()
            .unwrap_or_else(|| format!("run-{}", command.idempotency_key));
        let scope = ApplicationExecutionScope {
            application_id: command.application_id,
            session_id: session_id.clone(),
            run_id: run_id.clone(),
            tenant_id: command.tenant_id.clone(),
            actor: command.actor.clone(),
        };
        let event = build_event(
            &scope,
            ApplicationExecutionEventType::ProviderAssigned,
            &selection.descriptor.provider_id,
            selection.descriptor.provider_kind,
            command.trace.clone(),
            ApplicationExecutionPayload {
                summary: "provider assigned for application execution".into(),
                data: Some(serde_json::json!({
                    "provider_kind": format!("{:?}", selection.descriptor.provider_kind),
                    "considered_providers": selection.considered.len(),
                })),
                payload_ref: None,
                truncated: false,
            },
            format!("{}:provider-assigned", command.idempotency_key),
        );
        let persisted = store.append_idempotent(event).await?;
        let mut provider_command = command.clone();
        provider_command.session_id = Some(session_id.clone());
        provider_command.run_id = Some(run_id.clone());
        let mut result = selection.provider.start(provider_command).await?;
        if result.status == ApplicationExecutionCommandStatus::Accepted {
            result.session_id = Some(session_id);
            result.run_id = Some(run_id);
            result.provider_id = Some(selection.descriptor.provider_id);
            result.provider_kind = selection.descriptor.provider_kind;
            result.event_cursor = persisted.seq.map(|seq| format!("event/{seq}"));
        }
        Ok(result)
    }

    async fn handle_control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> ServiceResult<ApplicationExecutionControlResult> {
        let store = self.event_store()?;
        let selection = self
            .provider_registry
            .select(None, &[])
            .map_err(|error| ServiceError::ServiceUnavailable(error.reason))?;
        let requested = build_event(
            &command.scope,
            ApplicationExecutionEventType::ControlRequested,
            &selection.descriptor.provider_id,
            selection.descriptor.provider_kind,
            command.trace.clone(),
            ApplicationExecutionPayload {
                summary: "application execution control requested".into(),
                data: Some(serde_json::json!({
                    "control_id": command.control_id,
                    "control_kind": format!("{:?}", command.command),
                    "reason_code": command.reason_code,
                })),
                payload_ref: None,
                truncated: false,
            },
            format!("{}:control-requested", command.idempotency_key),
        );
        let persisted = store.append_idempotent(requested).await?;
        let mut result = selection.provider.control(command).await?;
        result.event_cursor = persisted.seq.map(|seq| format!("event/{seq}"));
        Ok(result)
    }

    async fn append_gateway_event(
        &self,
        command: AppendExecutionEventCommand,
    ) -> ServiceResult<ApplicationExecutionEventEnvelope> {
        let store = self.event_store()?;
        store.append_idempotent(command.event).await
    }
}

#[async_trait]
impl SystemService for ApplicationExecutionSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            configured = false,
            "application execution service unavailable provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        warn!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            reason = %self.reason(),
            "application execution service returning structured unavailable"
        );
        match command.name.as_str() {
            APPLICATION_EXECUTION_START_COMMAND => {
                if self.event_store.is_none() {
                    Self::service_result(self.unavailable_start(), trace)
                } else {
                    let typed: StartApplicationExecutionCommand =
                        serde_json::from_value(command.payload).map_err(adapter_error)?;
                    Self::service_result(self.handle_start(typed).await?, trace)
                }
            }
            APPLICATION_EXECUTION_CONTROL_COMMAND => {
                let typed: ApplicationExecutionControlCommand =
                    serde_json::from_value(command.payload).map_err(adapter_error)?;
                if self.event_store.is_none() {
                    Self::service_result(
                        ApplicationExecutionControlResult {
                            status: ApplicationExecutionCommandStatus::Unavailable,
                            scope: typed.scope,
                            provider_id: None,
                            provider_kind: ApplicationExecutionProviderKind::Unavailable,
                            event_cursor: None,
                            error: Some(macaca_proto::ApplicationExecutionError::unavailable(
                                APPLICATION_EXECUTION_CONTROL_COMMAND,
                                self.reason(),
                            )),
                        },
                        trace,
                    )
                } else {
                    Self::service_result(self.handle_control(typed).await?, trace)
                }
            }
            APPLICATION_EXECUTION_REPLAY_COMMAND => {
                let request: ApplicationExecutionReplayRequest =
                    serde_json::from_value(command.payload).map_err(adapter_error)?;
                let result = if let Some(store) = &self.event_store {
                    store.replay(request).await?
                } else {
                    ApplicationExecutionReplayResult {
                        events: Vec::new(),
                        next_cursor: None,
                        current_state: None,
                    }
                };
                Self::service_result(result, trace)
            }
            APPLICATION_EXECUTION_CURRENT_STATE_COMMAND => {
                let scope: ApplicationExecutionScope =
                    serde_json::from_value(command.payload).map_err(adapter_error)?;
                let state = self.event_store()?.current_state(scope).await?;
                Self::service_result(state, trace)
            }
            APPLICATION_EXECUTION_PROVIDER_HEALTH_COMMAND => {
                Self::service_result(self.provider_registry.descriptors(), trace)
            }
            APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND => {
                let typed: AppendExecutionEventCommand =
                    serde_json::from_value(command.payload).map_err(adapter_error)?;
                Self::service_result(self.append_gateway_event(typed).await?, trace)
            }
            APPLICATION_EXECUTION_GATEWAY_HEARTBEAT_COMMAND => {
                let typed: ReportExecutionHeartbeatCommand =
                    serde_json::from_value(command.payload).map_err(adapter_error)?;
                let event = build_event(
                    &typed.scope,
                    ApplicationExecutionEventType::ProviderHeartbeat,
                    &typed.provider_id,
                    typed.provider_kind,
                    typed.trace,
                    ApplicationExecutionPayload::summary("provider heartbeat reported"),
                    format!("heartbeat:{}", typed.reported_at.timestamp_millis()),
                );
                Self::service_result(self.event_store()?.append_idempotent(event).await?, trace)
            }
            APPLICATION_EXECUTION_GATEWAY_APPROVAL_COMMAND => {
                let typed: RequestExecutionApprovalCommand =
                    serde_json::from_value(command.payload).map_err(adapter_error)?;
                let event = build_event(
                    &typed.scope,
                    ApplicationExecutionEventType::ApprovalRequested,
                    "gateway",
                    ApplicationExecutionProviderKind::ExternalAppBackend,
                    typed.trace,
                    ApplicationExecutionPayload {
                        summary: typed.prompt.summary,
                        data: Some(serde_json::json!({"approval_ref": typed.approval_ref})),
                        payload_ref: typed.prompt.payload_ref,
                        truncated: typed.prompt.truncated,
                    },
                    typed.idempotency_key,
                );
                Self::service_result(self.event_store()?.append_idempotent(event).await?, trace)
            }
            APPLICATION_EXECUTION_GATEWAY_COMPLETION_COMMAND => {
                let typed: ReportExecutionCompletionCommand =
                    serde_json::from_value(command.payload).map_err(adapter_error)?;
                let event = build_event(
                    &typed.scope,
                    ApplicationExecutionEventType::ExecutionCompleted,
                    "gateway",
                    ApplicationExecutionProviderKind::ExternalAppBackend,
                    typed.trace,
                    typed.result,
                    typed.idempotency_key,
                );
                Self::service_result(self.event_store()?.append_idempotent(event).await?, trace)
            }
            APPLICATION_EXECUTION_GATEWAY_FAILURE_COMMAND => {
                let typed: ReportExecutionFailureCommand =
                    serde_json::from_value(command.payload).map_err(adapter_error)?;
                let event = build_event(
                    &typed.scope,
                    ApplicationExecutionEventType::ExecutionFailed,
                    "gateway",
                    ApplicationExecutionProviderKind::ExternalAppBackend,
                    typed.trace,
                    ApplicationExecutionPayload {
                        summary: typed.error.reason.clone(),
                        data: Some(serde_json::to_value(typed.error).map_err(adapter_error)?),
                        payload_ref: None,
                        truncated: false,
                    },
                    typed.idempotency_key,
                );
                Self::service_result(self.event_store()?.append_idempotent(event).await?, trace)
            }
            APPLICATION_EXECUTION_SNAPSHOT_COMMAND => {
                Err(ServiceError::ServiceUnavailable(self.reason()))
            }
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "application execution service stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "application execution service cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.event_store.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: self.reason(),
            })
        }
    }
}

fn build_event(
    scope: &ApplicationExecutionScope,
    event_type: ApplicationExecutionEventType,
    provider_id: &str,
    provider_kind: ApplicationExecutionProviderKind,
    trace: TraceContext,
    payload: ApplicationExecutionPayload,
    idempotency_key: String,
) -> ApplicationExecutionEventEnvelope {
    ApplicationExecutionEventEnvelope {
        application_id: scope.application_id,
        session_id: scope.session_id.clone(),
        run_id: scope.run_id.clone(),
        seq: None,
        timestamp: chrono::Utc::now(),
        event_type,
        trace,
        actor: scope.actor.clone(),
        provider_id: provider_id.into(),
        provider_kind,
        visibility: "session".into(),
        causality: Vec::new(),
        sanitized_payload: payload,
        payload_ref: None,
        schema_version: "application-execution.v1".into(),
        idempotency_key,
    }
}

fn adapter_error(error: serde_json::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}

fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}

/// Register and start the unavailable application execution provider.
pub async fn bootstrap_unavailable_application_execution_service(
    runtime: Arc<crate::ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> =
        Arc::new(ApplicationExecutionSystemServiceProvider::unavailable(
            "application execution provider stack is not configured",
        ));
    register_application_execution_service(runtime, service, trace_id).await
}

/// Register and start the EventLog-backed application execution service.
pub async fn bootstrap_application_execution_service(
    runtime: Arc<crate::ServiceRuntime>,
    event_log: Arc<EventLog>,
    provider_registry: ApplicationExecutionProviderRegistry,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(
        ApplicationExecutionSystemServiceProvider::with_event_log(event_log, provider_registry),
    );
    register_application_execution_service(runtime, service, trace_id).await
}

async fn register_application_execution_service(
    runtime: Arc<crate::ServiceRuntime>,
    service: Arc<dyn SystemService>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(service_id = %service_id, trace_id = %trace.trace_id, "application execution service registering unavailable provider");
    runtime
        .register_provider(
            &crate::StaticServiceProviderFactory::new(crate::ServiceProviderInstance::new(
                descriptor, service,
            )),
            crate::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(runtime_error)?;
    runtime
        .start(&service_id, trace)
        .await
        .map_err(runtime_error)?;
    Ok(service_id)
}

/// Return the service descriptor for tests and runtime catalog code.
pub fn application_execution_service_descriptor_runtime() -> ServiceDescriptor {
    application_execution_service_descriptor()
}

/// Return the stable service id for runtime-host code that avoids proto imports.
pub fn application_execution_service_id() -> &'static str {
    APPLICATION_EXECUTION_SERVICE_ID
}
