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
    CleanupPolicy, MacacaError, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, StartApplicationExecutionCommand,
    StartApplicationExecutionResult, TraceContext, APPLICATION_EXECUTION_CONTROL_COMMAND,
    APPLICATION_EXECUTION_CURRENT_STATE_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_APPROVAL_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_COMPLETION_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_FAILURE_COMMAND, APPLICATION_EXECUTION_GATEWAY_HEARTBEAT_COMMAND,
    APPLICATION_EXECUTION_PROVIDER_HEALTH_COMMAND, APPLICATION_EXECUTION_REPLAY_COMMAND,
    APPLICATION_EXECUTION_SERVICE_ID, APPLICATION_EXECUTION_SNAPSHOT_COMMAND,
    APPLICATION_EXECUTION_START_COMMAND,
};
use tracing::info;

use crate::application_execution_event_builder::build_event;
use crate::application_execution_event_store::ApplicationExecutionEventStore;
use crate::application_execution_gateway_events::{
    append_gateway_approval_request, append_gateway_completion, append_gateway_failure,
    append_gateway_heartbeat,
};
use crate::application_execution_provider_registry::{
    ApplicationExecutionProviderRegistry, ApplicationExecutionProviderSelectionCriteria,
};
use crate::application_execution_service_logs::{
    log_command_received, log_control_route, log_failure, log_gateway_ingress,
    log_provider_assignment, log_service_lifecycle, log_snapshot,
};

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
            .select_with_criteria(
                ApplicationExecutionProviderSelectionCriteria::from_start_command(&command),
            )
            .await
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
        log_provider_assignment(
            &scope.application_id.to_string(),
            &scope.session_id,
            &scope.run_id,
            &selection.descriptor.provider_id,
            selection.descriptor.provider_kind,
            &command.trace.trace_id,
        );
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
            .await
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
        log_control_route(
            &command.scope.application_id.to_string(),
            &command.scope.session_id,
            &command.scope.run_id,
            &selection.descriptor.provider_id,
            selection.descriptor.provider_kind,
            &command.trace.trace_id,
            "requested",
        );
        let mut result = selection.provider.control(command).await?;
        result.event_cursor = persisted.seq.map(|seq| format!("event/{seq}"));
        Ok(result)
    }

    async fn append_gateway_event(
        &self,
        command: AppendExecutionEventCommand,
    ) -> ServiceResult<ApplicationExecutionEventEnvelope> {
        let store = self.event_store()?;
        log_gateway_ingress(
            &command.event.application_id.to_string(),
            &command.event.session_id,
            &command.event.run_id,
            APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND,
            &command.event.trace.trace_id,
            "append_requested",
        );
        store.append_gateway_event(command).await
    }
}

#[async_trait]
impl SystemService for ApplicationExecutionSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        log_service_lifecycle(
            self.descriptor.id.as_str(),
            None,
            "started",
            self.event_store.is_some(),
            self.unavailable_reason.as_deref(),
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        log_command_received(
            self.descriptor.id.as_str(),
            command.name.as_str(),
            &trace.trace_id,
            if self.event_store.is_some() {
                "configured"
            } else {
                "unavailable"
            },
            self.unavailable_reason.as_deref(),
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
            APPLICATION_EXECUTION_GATEWAY_HEARTBEAT_COMMAND => Self::service_result(
                append_gateway_heartbeat(self.event_store()?, command.payload).await?,
                trace,
            ),
            APPLICATION_EXECUTION_GATEWAY_APPROVAL_COMMAND => Self::service_result(
                append_gateway_approval_request(self.event_store()?, command.payload).await?,
                trace,
            ),
            APPLICATION_EXECUTION_GATEWAY_COMPLETION_COMMAND => Self::service_result(
                append_gateway_completion(self.event_store()?, command.payload).await?,
                trace,
            ),
            APPLICATION_EXECUTION_GATEWAY_FAILURE_COMMAND => Self::service_result(
                append_gateway_failure(self.event_store()?, command.payload).await?,
                trace,
            ),
            APPLICATION_EXECUTION_SNAPSHOT_COMMAND => {
                log_snapshot(
                    APPLICATION_EXECUTION_SNAPSHOT_COMMAND,
                    &trace.trace_id,
                    "unavailable",
                    Some(&self.reason()),
                );
                Err(ServiceError::ServiceUnavailable(self.reason()))
            }
            other => {
                log_failure(other, &trace.trace_id, "unsupported_command");
                Err(ServiceError::UnsupportedCommand(other.into()))
            }
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
    register_application_execution_service(runtime, service, trace_id, false).await
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
    register_application_execution_service(runtime, service, trace_id, true).await
}

/// Register and start the default EventLog-backed application execution service.
///
/// This helper is the approved composition seam for presentation shells that
/// have only generic host infrastructure available. The runtime-host owns the
/// default provider registry construction here so Web/CLI adapters do not learn
/// provider lifecycle semantics or instantiate provider strategy containers.
/// Future provider stacks can replace this helper with a richer runtime-host
/// bootstrap without changing shell ownership.
pub async fn bootstrap_default_application_execution_service(
    runtime: Arc<crate::ServiceRuntime>,
    event_log: Arc<EventLog>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    bootstrap_application_execution_service(
        runtime,
        event_log,
        ApplicationExecutionProviderRegistry::new(),
        trace_id,
    )
    .await
}

async fn register_application_execution_service(
    runtime: Arc<crate::ServiceRuntime>,
    service: Arc<dyn SystemService>,
    trace_id: impl Into<String>,
    configured: bool,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    log_service_lifecycle(
        service_id.as_str(),
        Some(&trace.trace_id),
        "registering",
        configured,
        None,
    );
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

pub fn application_execution_service_descriptor_runtime() -> ServiceDescriptor {
    application_execution_service_descriptor()
}

pub fn application_execution_service_id() -> &'static str {
    APPLICATION_EXECUTION_SERVICE_ID
}
