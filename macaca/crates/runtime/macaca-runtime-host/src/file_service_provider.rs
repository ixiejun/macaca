//! Runtime-host provider for `service.file`.
//!
//! This service provider owns lifecycle, command dispatch, trace/audit result
//! envelopes, stable watch ids, and the `service.tool` bridge.  Concrete
//! filesystem behavior is delegated to a replaceable [`FileProvider`] Strategy
//! so future remote, virtual, artifact-backed, or policy-specialized providers
//! can enter through the same service boundary.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::file::*;
use macaca_proto::{
    BoundedSummary, CapabilityToolInvocationResult, CapabilityToolOriginKind, CleanupPolicy,
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, ToolInvokeCommand, TraceContext, WorkbenchCommandResult, WorkbenchCommandStatus,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

use crate::file_service_local::{resolve_path, FileProvider, LocalFileProvider};
use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// ServiceRuntime provider for filesystem commands.
pub struct FileSystemServiceProvider {
    descriptor: ServiceDescriptor,
    provider: Option<Arc<dyn FileProvider>>,
    watches: RwLock<HashMap<String, FileWatchRecord>>,
    notify_tx: broadcast::Sender<FileChangeNotification>,
    unavailable_reason: Option<String>,
}

/// Register and start the built-in local `service.file` provider.
///
/// This helper is the runtime-host composition point for local filesystem
/// access.  It keeps provider construction out of the kernel, SDK, Web routes,
/// CLI commands, and applications while making the service available to
/// `service.tool` owning-service dispatch.
pub async fn bootstrap_local_file_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(FileSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "file service registering local provider"
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
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "file service local provider started"
    );
    Ok(service_id)
}

impl FileSystemServiceProvider {
    /// Build the built-in local provider.  This is an Abstract Factory entry
    /// point for runtime-host composition roots; SDK and shells must not call
    /// host filesystem APIs directly.
    pub fn local() -> Self {
        Self::new(Arc::new(LocalFileProvider))
    }

    /// Build a provider-backed service with an injected filesystem Strategy.
    pub fn new(provider: Arc<dyn FileProvider>) -> Self {
        let (notify_tx, _) = broadcast::channel(1024);
        Self {
            descriptor: descriptor().base,
            provider: Some(provider),
            watches: RwLock::new(HashMap::new()),
            notify_tx,
            unavailable_reason: None,
        }
    }

    /// Build a Null Object provider for hosts without filesystem access.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let (notify_tx, _) = broadcast::channel(16);
        Self {
            descriptor: descriptor().base,
            provider: None,
            watches: RwLock::new(HashMap::new()),
            notify_tx,
            unavailable_reason: Some(reason.into()),
        }
    }

    /// Subscribe to bounded change notifications emitted after mutations.
    pub fn subscribe(&self) -> broadcast::Receiver<FileChangeNotification> {
        self.notify_tx.subscribe()
    }

    fn provider(&self) -> ServiceResult<Arc<dyn FileProvider>> {
        self.provider.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                self.unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "file provider is not configured".into()),
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
        response: FileServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
        summary: Option<BoundedSummary>,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary,
            audit_ref: Some(format!("file:{}", trace.trace_id)),
            completed_at: chrono::Utc::now(),
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

    async fn emit_change(&self, path: &str, kind: &str, trace: &TraceContext) {
        let watches = self.watches.read().await;
        for watch in watches.values() {
            let matches = if watch.recursive {
                path == watch.path || path.starts_with(&format!("{}/", watch.path))
            } else {
                path == watch.path
            };
            if matches {
                let event = FileChangeNotification {
                    watch_id: watch.watch_id.clone(),
                    path: path.into(),
                    change_kind: kind.into(),
                    trace_id: trace.trace_id.clone(),
                };
                let _ = self.notify_tx.send(event);
            }
        }
    }

    async fn unavailable_result(&self, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        Self::result(
            FileServiceResponse::DirectoryList {
                path: String::new(),
                entries: Vec::new(),
                truncated: false,
            },
            trace,
            WorkbenchCommandStatus::Unavailable {
                reason: self
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "file provider is not configured".into()),
            },
            None,
        )
    }
}

#[async_trait]
impl SystemService for FileSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            configured = self.provider.is_some(),
            "file service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "file service command accepted"
        );
        if self.provider.is_none() {
            warn!(
                service_id = %self.descriptor.id,
                trace_id = %trace.trace_id,
                "file service unavailable provider returned structured unavailable"
            );
            return self.unavailable_result(trace).await;
        }
        match command.name.as_str() {
            READ_COMMAND => {
                let read = self.provider()?.read(decode(command.payload)?).await?;
                Self::result(
                    read.response,
                    trace,
                    read.artifact_status
                        .unwrap_or(WorkbenchCommandStatus::Completed),
                    read.summary,
                )
            }
            WRITE_COMMAND => {
                let response = self.provider()?.write(decode(command.payload)?).await?;
                if let FileServiceResponse::Written { path, .. } = &response {
                    self.emit_change(path, "written", &trace).await;
                }
                Self::result(response, trace, WorkbenchCommandStatus::Completed, None)
            }
            PATCH_COMMAND => {
                let response = self.provider()?.patch(decode(command.payload)?).await?;
                if let FileServiceResponse::Patched { path, .. } = &response {
                    self.emit_change(path, "patched", &trace).await;
                }
                Self::result(response, trace, WorkbenchCommandStatus::Completed, None)
            }
            COPY_COMMAND => Self::result(
                self.provider()?.copy(decode(command.payload)?).await?,
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            REMOVE_COMMAND => {
                let response = self.provider()?.remove(decode(command.payload)?).await?;
                if let FileServiceResponse::Removed { path, .. } = &response {
                    self.emit_change(path, "removed", &trace).await;
                }
                Self::result(response, trace, WorkbenchCommandStatus::Completed, None)
            }
            METADATA_COMMAND => Self::result(
                self.provider()?.metadata(decode(command.payload)?).await?,
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            DIRECTORY_LIST_COMMAND => Self::result(
                self.provider()?.list(decode(command.payload)?).await?,
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            WATCH_COMMAND => self.watch(decode(command.payload)?, trace).await,
            UNWATCH_COMMAND => self.unwatch(decode(command.payload)?, trace).await,
            DIFF_COMMAND => Self::result(
                self.provider()?.diff(decode(command.payload)?).await?,
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            TOOL_INVOKE_COMMAND => self.tool_invoke(decode(command.payload)?).await,
            "service.snapshot" => Self::result(
                FileServiceResponse::DirectoryList {
                    path: "snapshot".into(),
                    entries: Vec::new(),
                    truncated: false,
                },
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "file service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.watches.write().await.clear();
        info!(service_id = %self.descriptor.id, "file service cleanup cleared watches");
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

impl FileSystemServiceProvider {
    async fn watch(
        &self,
        request: FileWatchRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let resolved = resolve_path(&request.target, false)?;
        let record = FileWatchRecord {
            watch_id: format!("file-watch-{}", Uuid::new_v4()),
            path: resolved.public_path,
            recursive: request.recursive,
            created_at: chrono::Utc::now(),
        };
        self.watches
            .write()
            .await
            .insert(record.watch_id.clone(), record.clone());
        Self::result(
            FileServiceResponse::Watch(record),
            trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    async fn unwatch(
        &self,
        request: FileUnwatchRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let removed = self
            .watches
            .write()
            .await
            .remove(&request.watch_id)
            .is_some();
        Self::result(
            FileServiceResponse::Unwatch {
                watch_id: request.watch_id,
                removed,
            },
            trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    async fn tool_invoke(&self, command: ToolInvokeCommand) -> ServiceResult<ServiceCallResult> {
        let operation = command
            .input
            .get("operation")
            .and_then(|value| value.as_str())
            .unwrap_or("read");
        let trace = command.trace.clone();
        let result = match operation {
            "read" => {
                let request: FileReadRequest = serde_json::from_value(command.input.clone())
                    .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
                let read = self.provider()?.read(request).await?;
                CapabilityToolInvocationResult::ok(
                    SERVICE_ID,
                    CapabilityToolOriginKind::Mcp,
                    command.tool_id,
                    serde_json::to_value(read.response)
                        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
                    trace.clone(),
                )
            }
            _ => CapabilityToolInvocationResult::failed(
                SERVICE_ID,
                CapabilityToolOriginKind::Mcp,
                command.tool_id,
                "unsupported service.file tool operation",
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
