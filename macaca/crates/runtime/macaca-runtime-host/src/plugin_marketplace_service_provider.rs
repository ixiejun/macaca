//! Runtime-host provider for `service.plugin_marketplace`.
//!
//! The provider applies Adapter and Facade patterns around the existing Plugin
//! Control Service.  Marketplace policy, entitlement/license/metering checks,
//! rollback mementos, capability summaries, and audit records live here, while
//! low-level plugin repository and runtime registration stay in
//! `PluginControlService`.  The provider never reads raw package bytes or
//! executes plugin code; it only coordinates typed manifests and sanitized
//! records through service-runtime boundaries.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::plugin_marketplace::*;
use macaca_proto::{
    CleanupPolicy, KernelServiceId, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, TraceContext,
};
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    plugin_capability::PluginCapabilityService, plugin_control::PluginControlService,
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// ServiceRuntime provider for marketplace lifecycle commands.
pub struct PluginMarketplaceSystemServiceProvider {
    pub(crate) descriptor: ServiceDescriptor,
    pub(crate) control: Arc<PluginControlService>,
    pub(crate) marketplaces: RwLock<HashMap<String, MarketplaceDescriptor>>,
    pub(crate) audits: RwLock<Vec<PluginMarketplaceAuditRecord>>,
    pub(crate) rollbacks: RwLock<Vec<PluginRollbackRecord>>,
    pub(crate) unavailable_reason: Option<String>,
}

/// Register and start the built-in local plugin marketplace provider.
pub async fn bootstrap_local_plugin_marketplace_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(PluginMarketplaceSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "plugin marketplace registering local provider"
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
        "plugin marketplace local provider started"
    );
    Ok(service_id)
}

impl PluginMarketplaceSystemServiceProvider {
    /// Build a local provider with in-memory marketplace and plugin state.
    pub fn local() -> Self {
        let capability_service = Arc::new(PluginCapabilityService::in_memory());
        let control = Arc::new(
            PluginControlService::builder()
                .capability_service(capability_service)
                .build(),
        );
        Self::new(control)
    }

    /// Build a deterministic provider for focused tests.
    pub fn mock() -> Self {
        Self::local()
    }

    /// Build a Null Object provider for hosts where marketplace is disabled.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::empty(
            Arc::new(PluginControlService::in_memory()),
            Some(reason.into()),
        )
    }

    /// Build a provider over an injected Plugin Control facade.
    pub fn new(control: Arc<PluginControlService>) -> Self {
        Self::empty(control, None)
    }

    fn empty(control: Arc<PluginControlService>, unavailable_reason: Option<String>) -> Self {
        let mut marketplaces = HashMap::new();
        marketplaces.insert("local".into(), default_marketplace());
        Self {
            descriptor: descriptor().base,
            control,
            marketplaces: RwLock::new(marketplaces),
            audits: RwLock::new(Vec::new()),
            rollbacks: RwLock::new(Vec::new()),
            unavailable_reason,
        }
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }
}

#[async_trait]
impl SystemService for PluginMarketplaceSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "plugin marketplace service provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "plugin marketplace command accepted"
        );
        let result = self
            .dispatch(command.name.as_str(), command.payload, trace.clone())
            .await;
        Ok(ServiceCallResult {
            output: serde_json::to_value(result)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "plugin marketplace service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "plugin marketplace service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if let Some(reason) = &self.unavailable_reason {
            return Ok(ServiceHealth::Unavailable {
                reason: reason.clone(),
            });
        }
        Ok(ServiceHealth::Healthy)
    }
}

fn default_marketplace() -> MarketplaceDescriptor {
    MarketplaceDescriptor {
        marketplace_id: "local".into(),
        kind: MarketplaceSourceKind::Local,
        enabled: true,
        source_policy: MarketplaceSourcePolicy::local_default(),
        metadata: BTreeMap::new(),
    }
}

fn runtime_error(error: impl std::fmt::Display) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}
