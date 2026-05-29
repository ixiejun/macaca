//! Runtime-host provider for optional `service.remote_environment`.
//!
//! Remote execution environments are optional Adapter modules.  This provider
//! owns the generic registration, health, workspace-root, cleanup, and
//! selection state machine while concrete transport details stay outside the
//! base OS.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::remote_environment::*;
use macaca_proto::{
    CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, TraceContext, WorkbenchCommandResult, WorkbenchCommandStatus,
};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// In-memory remote environment registry used by local hosts and tests.
pub struct RemoteEnvironmentSystemServiceProvider {
    descriptor: ServiceDescriptor,
    environments: RwLock<HashMap<String, RemoteEnvironmentRecord>>,
    unavailable_reason: Option<String>,
}

/// Register and start the local `service.remote_environment` registry.
///
/// The provider records provider-neutral remote environment metadata only; it
/// does not embed SSH, container, cloud, or product-specific execution logic.
/// Concrete remote adapters can later be installed behind the same service
/// boundary without changing shell or application code.
pub async fn bootstrap_local_remote_environment_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(RemoteEnvironmentSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "remote environment service registering local provider"
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
        "remote environment service local provider started"
    );
    Ok(service_id)
}

impl RemoteEnvironmentSystemServiceProvider {
    /// Build a local registry provider. Concrete remote execution happens in
    /// replaceable adapters; this provider records provider-neutral state.
    pub fn local() -> Self {
        Self::empty(None)
    }

    /// Build an in-memory mock provider for focused tests.
    pub fn mock() -> Self {
        Self::local()
    }

    /// Build a Null Object provider for hosts without remote environments.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::empty(Some(reason.into()))
    }

    fn empty(unavailable_reason: Option<String>) -> Self {
        Self {
            descriptor: descriptor().base,
            environments: RwLock::new(HashMap::new()),
            unavailable_reason,
        }
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result(
        response: RemoteEnvironmentServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary: None,
            audit_ref: Some(format!("remote-environment:{}", trace.trace_id)),
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

    async fn unavailable_result(&self, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        let reason = self
            .unavailable_reason
            .clone()
            .unwrap_or_else(|| "remote environment provider is not configured".into());
        warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason = %reason, "remote environment optional provider unavailable");
        let record = unavailable_record(reason.clone());
        Self::result(
            RemoteEnvironmentServiceResponse::Health(record),
            trace,
            WorkbenchCommandStatus::Unavailable { reason },
        )
    }

    async fn add(
        &self,
        request: RemoteEnvironmentAddRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        if request.endpoint_ref.trim().is_empty() {
            return Err(ServiceError::InvalidArgument(
                "remote_environment.add requires endpoint_ref".into(),
            ));
        }
        let now = chrono::Utc::now();
        let environment_ref = request
            .environment_ref
            .unwrap_or_else(|| format!("remote-env-{}", Uuid::new_v4()));
        let record = RemoteEnvironmentRecord {
            environment_ref: environment_ref.clone(),
            provider_kind: request.provider_kind,
            endpoint_ref: request.endpoint_ref,
            workspace_roots: request.workspace_roots,
            status: RemoteEnvironmentStatus::Registered,
            health: ServiceHealth::Healthy,
            connection_summary: request.connection_summary,
            created_at: now,
            updated_at: now,
        };
        self.environments
            .write()
            .await
            .insert(environment_ref.clone(), record.clone());
        info!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, environment_ref = %environment_ref, "remote environment registered");
        Self::result(
            RemoteEnvironmentServiceResponse::Environment(record),
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }

    async fn get_record(&self, environment_ref: &str) -> ServiceResult<RemoteEnvironmentRecord> {
        self.environments
            .read()
            .await
            .get(environment_ref)
            .cloned()
            .ok_or_else(|| ServiceError::InvalidArgument("remote environment not found".into()))
    }

    async fn health_command(
        &self,
        request: RemoteEnvironmentRefRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let record = self.get_record(&request.environment_ref).await?;
        Self::result(
            RemoteEnvironmentServiceResponse::Health(record),
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }

    async fn workspace_roots(
        &self,
        request: RemoteEnvironmentRefRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let record = self.get_record(&request.environment_ref).await?;
        Self::result(
            RemoteEnvironmentServiceResponse::WorkspaceRoots {
                environment_ref: record.environment_ref,
                workspace_roots: record.workspace_roots,
            },
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }

    async fn cleanup(
        &self,
        request: RemoteEnvironmentRefRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let mut environments = self.environments.write().await;
        let record = environments
            .get_mut(&request.environment_ref)
            .ok_or_else(|| ServiceError::InvalidArgument("remote environment not found".into()))?;
        record.updated_at = chrono::Utc::now();
        info!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, environment_ref = %record.environment_ref, "remote environment cleanup recorded");
        Self::result(
            RemoteEnvironmentServiceResponse::Cleanup {
                environment_ref: record.environment_ref.clone(),
                cleaned: true,
            },
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }

    async fn select(
        &self,
        request: RemoteEnvironmentSelectRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let mut environments = self.environments.write().await;
        let record = environments
            .get_mut(&request.environment_ref)
            .ok_or_else(|| ServiceError::InvalidArgument("remote environment not found".into()))?;
        record.status = RemoteEnvironmentStatus::Selected;
        record.updated_at = chrono::Utc::now();
        info!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, environment_ref = %record.environment_ref, "remote environment selected");
        Self::result(
            RemoteEnvironmentServiceResponse::Environment(record.clone()),
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }

    async fn remove(
        &self,
        request: RemoteEnvironmentRefRequest,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        self.environments
            .write()
            .await
            .remove(&request.environment_ref);
        info!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, environment_ref = %request.environment_ref, "remote environment removed");
        Self::result(
            RemoteEnvironmentServiceResponse::Removed {
                environment_ref: request.environment_ref,
            },
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }

    async fn snapshot(&self, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        let environments = self.environments.read().await.values().cloned().collect();
        info!(
            service_id = %self.descriptor.id,
            trace_id = %trace.trace_id,
            "remote environment snapshot created"
        );
        Self::result(
            RemoteEnvironmentServiceResponse::Snapshot(environments),
            trace,
            WorkbenchCommandStatus::Completed,
        )
    }
}

#[async_trait]
impl SystemService for RemoteEnvironmentSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, configured = self.unavailable_reason.is_none(), "remote environment service provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "remote environment service command accepted");
        if self.unavailable_reason.is_some() {
            return self.unavailable_result(trace).await;
        }
        match command.name.as_str() {
            ADD_COMMAND => self.add(decode(command.payload)?, trace).await,
            HEALTH_COMMAND => self.health_command(decode(command.payload)?, trace).await,
            WORKSPACE_ROOTS_COMMAND => self.workspace_roots(decode(command.payload)?, trace).await,
            CLEANUP_COMMAND => self.cleanup(decode(command.payload)?, trace).await,
            SELECT_COMMAND => self.select(decode(command.payload)?, trace).await,
            REMOVE_COMMAND => self.remove(decode(command.payload)?, trace).await,
            "service.snapshot" => self.snapshot(trace).await,
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "remote environment service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.environments.write().await.clear();
        info!(service_id = %self.descriptor.id, "remote environment cleanup cleared registry");
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

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}

fn runtime_error(error: crate::ServiceRuntimeError) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}

fn unavailable_record(reason: String) -> RemoteEnvironmentRecord {
    let now = chrono::Utc::now();
    RemoteEnvironmentRecord {
        environment_ref: "remote-environment-unavailable".into(),
        provider_kind: RemoteEnvironmentProviderKind::Other("unavailable".into()),
        endpoint_ref: "unavailable".into(),
        workspace_roots: Vec::new(),
        status: RemoteEnvironmentStatus::Unhealthy,
        health: ServiceHealth::Unavailable {
            reason: reason.clone(),
        },
        connection_summary: macaca_proto::BoundedSummary {
            text: reason,
            truncated: false,
            original_byte_len: None,
            artifact_ref: None,
        },
        created_at: now,
        updated_at: now,
    }
}
