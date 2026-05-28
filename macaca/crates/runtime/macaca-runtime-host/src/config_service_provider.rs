//! Runtime-host provider for `service.config`.
//!
//! The provider owns layered configuration, executable requirements, feature
//! enablement, permission-profile visibility, hot reload notifications, and
//! sanitized audit mementos.  It uses Builder-style effective-config resolution,
//! Specification-style requirement exposure, Observer events for reloads, and a
//! Null Object unavailable provider for hosts where configuration is disabled.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::config::*;
use macaca_proto::{
    BoundedSummary, CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};
use serde_json::Value;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

pub(crate) const SECRET_REDACTION: &str = "<redacted>";
pub(crate) const SECRET_KEY_FRAGMENTS: &[&str] =
    &["secret", "token", "password", "credential", "private_key"];

#[derive(Debug, Clone)]
pub(crate) struct ConfigValueRecord {
    pub key_path: String,
    pub layer: ConfigLayer,
    pub value: Value,
    pub redacted: bool,
    pub version: u64,
    pub writer_ref: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// ServiceRuntime provider for config commands.
pub struct ConfigSystemServiceProvider {
    pub(crate) descriptor: ServiceDescriptor,
    pub(crate) values: RwLock<HashMap<(ConfigLayer, String), ConfigValueRecord>>,
    pub(crate) features: RwLock<HashMap<String, FeatureFlagRecord>>,
    pub(crate) audits: RwLock<HashMap<String, ConfigAuditRecord>>,
    pub(crate) notify_tx: broadcast::Sender<ConfigChangedEvent>,
    pub(crate) unavailable_reason: Option<String>,
}

/// Register and start the built-in local `service.config` provider.
pub async fn bootstrap_local_config_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(ConfigSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "config service registering local provider"
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
        "config service local provider started"
    );
    Ok(service_id)
}

impl ConfigSystemServiceProvider {
    /// Build a local provider with generic default values.
    pub fn local() -> Self {
        Self::empty(None)
    }

    /// Build an empty mock/local provider for focused tests.
    pub fn mock() -> Self {
        Self::empty(None)
    }

    /// Build a Null Object provider for hosts where config is disabled.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::empty(Some(reason.into()))
    }

    fn empty(unavailable_reason: Option<String>) -> Self {
        let (notify_tx, _) = broadcast::channel(1024);
        Self {
            descriptor: descriptor().base,
            values: RwLock::new(HashMap::new()),
            features: RwLock::new(HashMap::new()),
            audits: RwLock::new(HashMap::new()),
            notify_tx,
            unavailable_reason,
        }
    }

    /// Subscribe to bounded config change events for shells and gateways.
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChangedEvent> {
        self.notify_tx.subscribe()
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    pub(crate) fn result(
        response: ConfigServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
        summary: Option<BoundedSummary>,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary,
            audit_ref: Some(format!("config:{}", trace.trace_id)),
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

    pub(crate) async fn unavailable_result(
        &self,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Self::result(
            ConfigServiceResponse::Features(Vec::new()),
            trace,
            WorkbenchCommandStatus::Unavailable {
                reason: self
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "config provider is not configured".into()),
            },
            None,
        )
    }
}

#[async_trait]
impl SystemService for ConfigSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            configured = self.unavailable_reason.is_none(),
            "config service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "config service command accepted"
        );
        if self.unavailable_reason.is_some() {
            warn!(
                service_id = %self.descriptor.id,
                trace_id = %trace.trace_id,
                "config service unavailable provider returned structured unavailable"
            );
            return self.unavailable_result(trace).await;
        }
        match command.name.as_str() {
            READ_COMMAND => self.read(decode(command.payload)?, trace).await,
            VALUE_WRITE_COMMAND => self.value_write(decode(command.payload)?, trace).await,
            BATCH_WRITE_COMMAND => self.batch_write(decode(command.payload)?, trace).await,
            SCHEMA_READ_COMMAND => self.schema_read(decode(command.payload)?, trace).await,
            RELOAD_COMMAND => self.reload(decode(command.payload)?, trace).await,
            REQUIREMENTS_READ_COMMAND => {
                self.requirements_read(decode(command.payload)?, trace)
                    .await
            }
            PERMISSION_PROFILE_LIST_COMMAND => {
                self.permission_profile_list(decode(command.payload)?, trace)
                    .await
            }
            FEATURE_LIST_COMMAND => self.feature_list(decode(command.payload)?, trace).await,
            FEATURE_ENABLEMENT_SET_COMMAND => {
                self.feature_enablement_set(decode(command.payload)?, trace)
                    .await
            }
            "service.snapshot" => self.snapshot(trace).await,
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "config service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.audits.write().await.clear();
        info!(service_id = %self.descriptor.id, "config service cleanup cleared audit mementos");
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

pub(crate) fn decode<T>(value: serde_json::Value) -> ServiceResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(value).map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}

pub(crate) fn audit_ref() -> String {
    format!("audit://config/{}", Uuid::new_v4())
}

pub(crate) fn event_ref() -> String {
    format!("event://config/{}", Uuid::new_v4())
}

fn runtime_error(error: crate::ServiceRuntimeError) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}
