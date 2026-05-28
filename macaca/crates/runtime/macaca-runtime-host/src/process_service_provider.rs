//! Runtime-host provider for `service.process`.
//!
//! The provider owns service lifecycle, command dispatch, result envelopes,
//! structured unavailable behavior, audit refs, and the service.tool bridge.
//! Concrete process mechanics are delegated to a replaceable [`ProcessProvider`]
//! Strategy so local, remote, container, plugin, and mock providers can share
//! one provider-neutral service contract.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::process::*;
use macaca_proto::{
    BoundedSummary, CapabilityToolInvocationResult, CapabilityToolOriginKind, CleanupPolicy,
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, ToolInvokeCommand, TraceContext, WorkbenchCommandResult, WorkbenchCommandStatus,
};
use tracing::{info, warn};

use crate::process_service_local::{LocalProcessProvider, ProcessProvider};
use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// ServiceRuntime provider for process commands.
pub struct ProcessSystemServiceProvider {
    descriptor: ServiceDescriptor,
    provider: Option<Arc<dyn ProcessProvider>>,
    unavailable_reason: Option<String>,
}

/// Register and start the built-in local `service.process` provider.
///
/// This helper is the runtime-host composition point.  It keeps process
/// provider construction out of kernel, SDK, shell, and application code while
/// allowing service.tool to route shell/process capabilities through an owning
/// service boundary.
pub async fn bootstrap_local_process_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(ProcessSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "process service registering local provider"
    );
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(runtime_error)?;
    runtime
        .start(&service_id, trace.clone())
        .await
        .map_err(runtime_error)?;
    Ok(service_id)
}

impl ProcessSystemServiceProvider {
    /// Build the built-in local provider.  This is an Abstract Factory entry
    /// point for runtime-host composition roots.
    pub fn local() -> Self {
        Self::new(Arc::new(LocalProcessProvider::default()))
    }

    /// Build a provider-backed service with an injected process Strategy.
    pub fn new(provider: Arc<dyn ProcessProvider>) -> Self {
        Self {
            descriptor: descriptor().base,
            provider: Some(provider),
            unavailable_reason: None,
        }
    }

    /// Build a Null Object provider for hosts without process access.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            descriptor: descriptor().base,
            provider: None,
            unavailable_reason: Some(reason.into()),
        }
    }

    fn provider(&self) -> ServiceResult<Arc<dyn ProcessProvider>> {
        self.provider.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                self.unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "process provider is not configured".into()),
            )
        })
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result(
        response: ProcessServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
        summary: Option<BoundedSummary>,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary,
            audit_ref: Some(format!("process:{}", trace.trace_id)),
            completed_at: chrono::Utc::now(),
        };
        Ok(ServiceCallResult {
            output: serde_json::to_value(result)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::OnStop),
        })
    }

    async fn unavailable_result(&self, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        Self::result(
            ProcessServiceResponse::Cleanup {
                terminated_handles: Vec::new(),
                retained_handles: Vec::new(),
            },
            trace,
            WorkbenchCommandStatus::Unavailable {
                reason: self
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "process provider is not configured".into()),
            },
            None,
        )
    }
}

#[async_trait]
impl SystemService for ProcessSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            configured = self.provider.is_some(),
            "process service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "process service command accepted"
        );
        if self.provider.is_none() {
            warn!(
                service_id = %self.descriptor.id,
                trace_id = %trace.trace_id,
                "process service unavailable provider returned structured unavailable"
            );
            return self.unavailable_result(trace).await;
        }
        match command.name.as_str() {
            EXEC_COMMAND => {
                let output = self.provider()?.exec(decode(command.payload)?).await?;
                Self::result(
                    ProcessServiceResponse::Exec {
                        record: output.record,
                        output: output.output,
                    },
                    trace,
                    WorkbenchCommandStatus::Completed,
                    None,
                )
            }
            SPAWN_COMMAND => Self::result(
                ProcessServiceResponse::Spawned(
                    self.provider()?.spawn(decode(command.payload)?).await?,
                ),
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            STDIN_WRITE_COMMAND => {
                let request: ProcessStdinWriteRequest = decode(command.payload)?;
                let byte_len = self.provider()?.write_stdin(request.clone()).await?;
                Self::result(
                    ProcessServiceResponse::StdinWritten {
                        handle: request.handle,
                        byte_len,
                    },
                    trace,
                    WorkbenchCommandStatus::Completed,
                    None,
                )
            }
            PTY_RESIZE_COMMAND => {
                let request: ProcessPtyResizeRequest = decode(command.payload)?;
                let record = self.provider()?.resize_pty(request.clone()).await?;
                Self::result(
                    ProcessServiceResponse::PtyResized {
                        handle: request.handle,
                        rows: request.rows,
                        cols: request.cols,
                    },
                    trace,
                    WorkbenchCommandStatus::Completed,
                    Some(BoundedSummary {
                        text: format!("process PTY size recorded for {}", record.handle.id),
                        truncated: false,
                        original_byte_len: None,
                        artifact_ref: None,
                    }),
                )
            }
            TERMINATE_COMMAND => Self::result(
                ProcessServiceResponse::Terminated(
                    self.provider()?.terminate(decode(command.payload)?).await?,
                ),
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            OUTPUT_SUBSCRIBE_COMMAND => {
                let request: ProcessOutputSubscribeRequest = decode(command.payload)?;
                let deltas = self.provider()?.subscribe_output(request.clone()).await?;
                Self::result(
                    ProcessServiceResponse::OutputSubscribed {
                        handle: request.handle,
                        deltas,
                    },
                    trace,
                    WorkbenchCommandStatus::Completed,
                    None,
                )
            }
            STATUS_COMMAND => Self::result(
                ProcessServiceResponse::Status(
                    self.provider()?.status(decode(command.payload)?).await?,
                ),
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            CLEANUP_COMMAND => {
                let output = self.provider()?.cleanup(decode(command.payload)?).await?;
                Self::result(
                    ProcessServiceResponse::Cleanup {
                        terminated_handles: output.terminated_handles,
                        retained_handles: output.retained_handles,
                    },
                    trace,
                    WorkbenchCommandStatus::Completed,
                    None,
                )
            }
            TOOL_INVOKE_COMMAND => self.tool_invoke(decode(command.payload)?).await,
            "service.snapshot" => Self::result(
                ProcessServiceResponse::Cleanup {
                    terminated_handles: Vec::new(),
                    retained_handles: Vec::new(),
                },
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "process service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        if let Some(provider) = self.provider.clone() {
            let _ = provider
                .cleanup(ProcessCleanupRequest {
                    application_id: None,
                    session_id: None,
                    thread_ref: None,
                    reason: "service cleanup".into(),
                })
                .await;
        }
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if let Some(reason) = &self.unavailable_reason {
            Ok(ServiceHealth::Unavailable {
                reason: reason.clone(),
            })
        } else {
            Ok(ServiceHealth::Healthy)
        }
    }
}

impl ProcessSystemServiceProvider {
    async fn tool_invoke(&self, command: ToolInvokeCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.clone();
        let result = match command
            .input
            .get("operation")
            .and_then(|value| value.as_str())
            .unwrap_or("exec")
        {
            "exec" => {
                let request: ProcessCommandRequest = serde_json::from_value(command.input.clone())
                    .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
                let output = self.provider()?.exec(request).await?;
                CapabilityToolInvocationResult::ok(
                    SERVICE_ID,
                    CapabilityToolOriginKind::Mcp,
                    command.tool_id,
                    serde_json::to_value(ProcessServiceResponse::Exec {
                        record: output.record,
                        output: output.output,
                    })
                    .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
                    trace.clone(),
                )
            }
            _ => CapabilityToolInvocationResult::failed(
                SERVICE_ID,
                CapabilityToolOriginKind::Mcp,
                command.tool_id,
                "unsupported service.process tool operation",
                trace.clone(),
            ),
        };
        Ok(ServiceCallResult {
            output: serde_json::to_value(result)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}

fn runtime_error(error: crate::ServiceRuntimeError) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}
