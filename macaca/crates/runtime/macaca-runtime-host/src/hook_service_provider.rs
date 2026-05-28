//! Runtime-host provider for `service.hook`.
//!
//! The provider owns managed lifecycle hook selection, Chain of Responsibility
//! execution, bounded model-visible mutations, audit mementos, and event
//! emission.  Concrete execution hosts are Adapter seams: the local provider can
//! run descriptor-backed local/mock hooks now, while script, WASM, remote, and
//! plugin adapters report structured unavailable until their providers are
//! installed through approved composition roots.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::hook::*;
use macaca_proto::{
    BoundedSummary, CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

pub(crate) const LIST_LIMIT_DEFAULT: usize = 100;
pub(crate) const LIST_LIMIT_MAX: usize = 500;
pub(crate) const INLINE_FEEDBACK_MAX_BYTES: usize = 4096;

/// Register and start the built-in local `service.hook` provider.
///
/// Runtime-host remains the Abstract Factory composition root.  SDK, kernel,
/// shells, and applications receive only the typed service boundary and never
/// construct executable hook adapters directly.
pub async fn bootstrap_local_hook_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(HookSystemServiceProvider::local(Vec::new()));
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "hook service registering local provider"
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
        "hook service local provider started"
    );
    Ok(service_id)
}

/// ServiceRuntime provider for managed lifecycle hook commands.
pub struct HookSystemServiceProvider {
    pub(crate) descriptor: ServiceDescriptor,
    pub(crate) hooks: RwLock<Vec<HookDescriptor>>,
    pub(crate) audits: RwLock<HashMap<String, HookAuditRecord>>,
    pub(crate) notify_tx: broadcast::Sender<HookEvent>,
    pub(crate) unavailable_reason: Option<String>,
}

impl HookSystemServiceProvider {
    /// Build a local/mock provider from injected descriptors.
    pub fn local(hooks: Vec<HookDescriptor>) -> Self {
        let (notify_tx, _) = broadcast::channel(1024);
        Self {
            descriptor: descriptor().base,
            hooks: RwLock::new(hooks),
            audits: RwLock::new(HashMap::new()),
            notify_tx,
            unavailable_reason: None,
        }
    }

    /// Build a Null Object provider for hosts where hook execution is disabled.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let (notify_tx, _) = broadcast::channel(16);
        Self {
            descriptor: descriptor().base,
            hooks: RwLock::new(Vec::new()),
            audits: RwLock::new(HashMap::new()),
            notify_tx,
            unavailable_reason: Some(reason.into()),
        }
    }

    /// Subscribe to bounded hook lifecycle events for shells and gateways.
    pub fn subscribe(&self) -> broadcast::Receiver<HookEvent> {
        self.notify_tx.subscribe()
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    pub(crate) fn result(
        response: HookServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
        summary: Option<BoundedSummary>,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary,
            audit_ref: Some(format!("hook:{}", trace.trace_id)),
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
            HookServiceResponse::Catalog {
                hooks: Vec::new(),
                truncated: false,
            },
            trace,
            WorkbenchCommandStatus::Unavailable {
                reason: self
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "hook provider is not configured".into()),
            },
            None,
        )
    }
}

#[async_trait]
impl SystemService for HookSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            configured = self.unavailable_reason.is_none(),
            "hook service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "hook service command accepted"
        );
        if self.unavailable_reason.is_some() {
            warn!(
                service_id = %self.descriptor.id,
                trace_id = %trace.trace_id,
                "hook service unavailable provider returned structured unavailable"
            );
            return self.unavailable_result(trace).await;
        }
        match command.name.as_str() {
            CATALOG_LIST_COMMAND => self.catalog_list(decode(command.payload)?, trace).await,
            POLICY_RESOLVE_COMMAND => self.policy_resolve(decode(command.payload)?, trace).await,
            PRE_TOOL_RUN_COMMAND => self.pre_tool_run(decode(command.payload)?, trace).await,
            POST_TOOL_RUN_COMMAND => self.post_tool_run(decode(command.payload)?, trace).await,
            SESSION_RUN_COMMAND => self.lifecycle_run(decode(command.payload)?, trace).await,
            TURN_RUN_COMMAND => self.lifecycle_run(decode(command.payload)?, trace).await,
            RESULT_AUDIT_COMMAND => self.result_audit(decode(command.payload)?, trace).await,
            "service.snapshot" => {
                let count = self.hooks.read().await.len();
                Self::result(
                    HookServiceResponse::Catalog {
                        hooks: Vec::new(),
                        truncated: count > 0,
                    },
                    trace,
                    WorkbenchCommandStatus::Completed,
                    Some(BoundedSummary {
                        text: format!("hook descriptors tracked: {count}"),
                        truncated: false,
                        original_byte_len: None,
                        artifact_ref: None,
                    }),
                )
            }
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "hook service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.audits.write().await.clear();
        info!(service_id = %self.descriptor.id, "hook service cleanup cleared audit mementos");
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

pub(crate) fn sanitize_summary(summary: Option<BoundedSummary>) -> Option<BoundedSummary> {
    summary.map(|mut value| {
        let byte_len = value.text.len();
        if byte_len > INLINE_FEEDBACK_MAX_BYTES {
            value.text.truncate(INLINE_FEEDBACK_MAX_BYTES);
            value.truncated = true;
            value.original_byte_len = Some(byte_len as u64);
        }
        value
    })
}

pub(crate) fn event_ref() -> String {
    format!("event://hook/{}", Uuid::new_v4())
}

pub(crate) fn audit_ref() -> String {
    format!("audit://hook/{}", Uuid::new_v4())
}

fn runtime_error(error: crate::ServiceRuntimeError) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}
