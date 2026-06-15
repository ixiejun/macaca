//! Runtime-host adapter for the Memory Service.
//!
//! The adapter owns service lifecycle and command translation.  Memory storage,
//! routing, governance, and topology decisions remain behind the injected
//! `MemoryFacade` strategy from `macaca-memory`.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_memory::{
    memory_service_descriptor, topology_labels_for_scope, MemoryDeleteRequest, MemoryFacade,
    MemoryGetCommand, MemoryGetRequest, MemoryGetResult, MemoryPrefetchCommand,
    MemoryPrefetchRequest, MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand,
    MemoryRememberResult, MemorySearchRequest, MemoryServiceSnapshot, MemoryServiceSnapshotCommand,
    MemoryStatusCommand, MemoryWriteRequest, MEMORY_FORGET_COMMAND, MEMORY_GET_COMMAND,
    MEMORY_PREFETCH_COMMAND, MEMORY_RECALL_COMMAND, MEMORY_REMEMBER_COMMAND,
    MEMORY_SNAPSHOT_COMMAND, MEMORY_STATUS_COMMAND,
};
use macaca_proto::{
    CleanupPolicy, MacacaError, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, TraceContext,
};

/// Host-owned Memory service provider that delegates to a memory facade.
pub struct MemorySystemServiceProvider {
    descriptor: ServiceDescriptor,
    facade: Arc<dyn MemoryFacade>,
}

impl MemorySystemServiceProvider {
    /// Create a provider-neutral memory service adapter.
    pub fn new(facade: Arc<dyn MemoryFacade>) -> Self {
        Self {
            descriptor: memory_service_descriptor(),
            facade,
        }
    }

    fn result(output: serde_json::Value, trace: TraceContext) -> ServiceCallResult {
        ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::OnStop),
        }
    }
}

#[async_trait]
impl SystemService for MemorySystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "memory service provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "memory service command accepted"
        );

        match command.name.as_str() {
            MEMORY_REMEMBER_COMMAND => {
                let typed: MemoryRememberCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                let request = MemoryWriteRequest::new(typed.scope, typed.content)
                    .layer(typed.layer)
                    .metadata(typed.metadata);
                let id = self.facade.remember(request).await.map_err(service_error)?;
                let result = MemoryRememberResult {
                    id,
                    stored_at: chrono::Utc::now(),
                };
                tracing::info!(trace_id = %typed.trace.trace_id, "memory service remember completed");
                Ok(Self::result(
                    serde_json::to_value(result).map_err(json_error)?,
                    typed.trace,
                ))
            }
            MEMORY_RECALL_COMMAND => {
                let typed: MemoryRecallCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                let entries = self
                    .facade
                    .search(MemorySearchRequest::new(
                        typed.scope,
                        typed.query,
                        typed.limit,
                    ))
                    .await
                    .map_err(service_error)?;
                let result = MemoryRecallResult::new(entries);
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = result.total_candidates,
                    "memory service recall completed"
                );
                Ok(Self::result(
                    serde_json::to_value(result).map_err(json_error)?,
                    typed.trace,
                ))
            }
            MEMORY_PREFETCH_COMMAND => {
                let typed: MemoryPrefetchCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                let entries = self
                    .facade
                    .prefetch(MemoryPrefetchRequest {
                        scope: typed.scope,
                        query: typed.query,
                        limit: typed.limit,
                    })
                    .await
                    .map_err(service_error)?;
                let result = MemoryRecallResult::new(entries);
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = result.total_candidates,
                    "memory service prefetch completed"
                );
                Ok(Self::result(
                    serde_json::to_value(result).map_err(json_error)?,
                    typed.trace,
                ))
            }
            MEMORY_GET_COMMAND => {
                let typed: MemoryGetCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                let entry = self
                    .facade
                    .get(MemoryGetRequest {
                        scope: typed.scope,
                        id: typed.id,
                    })
                    .await
                    .map_err(service_error)?;
                let result = MemoryGetResult::new(entry);
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    found = result.entry.is_some(),
                    "memory service get completed"
                );
                Ok(Self::result(
                    serde_json::to_value(result).map_err(json_error)?,
                    typed.trace,
                ))
            }
            MEMORY_FORGET_COMMAND => {
                let typed: macaca_memory::MemoryForgetCommand =
                    serde_json::from_value(command.payload)
                        .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                self.facade
                    .delete(MemoryDeleteRequest {
                        scope: typed.scope,
                        id: typed.id,
                    })
                    .await
                    .map_err(service_error)?;
                tracing::info!(trace_id = %typed.trace.trace_id, "memory service forget completed");
                Ok(Self::result(
                    serde_json::json!({"forgot": true}),
                    typed.trace,
                ))
            }
            MEMORY_STATUS_COMMAND => {
                let typed: MemoryStatusCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                let status = self.facade.status();
                tracing::info!(trace_id = %typed.trace.trace_id, "memory service status emitted");
                Ok(Self::result(
                    serde_json::to_value(status).map_err(json_error)?,
                    typed.trace,
                ))
            }
            MEMORY_SNAPSHOT_COMMAND => {
                let typed: MemoryServiceSnapshotCommand =
                    serde_json::from_value(command.payload)
                        .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                let status = self.facade.status();
                let topology = topology_labels_for_scope(&typed.scope).ok();
                let snapshot = MemoryServiceSnapshot::new(
                    status.provider_id,
                    status.healthy,
                    status.capabilities,
                    topology,
                );
                tracing::info!(trace_id = %typed.trace.trace_id, "memory service snapshot emitted");
                Ok(Self::result(
                    serde_json::to_value(snapshot).map_err(json_error)?,
                    typed.trace,
                ))
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Memory service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "memory service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "memory service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        let status = self.facade.status();
        if status.healthy {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: status
                    .message
                    .unwrap_or_else(|| "memory provider unhealthy".into()),
            })
        }
    }
}

fn service_error(err: MacacaError) -> ServiceError {
    ServiceError::AdapterFailure(err.to_string())
}

fn json_error(err: serde_json::Error) -> ServiceError {
    ServiceError::AdapterFailure(err.to_string())
}
