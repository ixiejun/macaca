//! Runtime-host adapter for the Application Service (Facade module tree).
//!
//! This provider uses Adapter/Bridge: `ServiceRuntime` receives provider-neutral
//! commands while this module delegates application semantics to `macaca-app`.
//! The host owns lifecycle dispatch, trace-required service execution, and
//! structured unavailable behavior; `macaca-app` remains the owner of manifests,
//! runtime assembly, ApplicationHost, ABI metadata, and lifecycle semantics.
//!
//! **Pattern:** Facade + Command Router — `system_service` owns lifecycle hooks,
//! `command_dispatch` routes typed commands, and focused handler modules keep
//! each Application syscall surface under the 500-line constitution.
//!
//! Module layout:
//! - `support.rs` — DTO shaping, decode/encode helpers, view projection
//! - `wasm_session.rs` — lazy WASM session factory + policy sync Bridge
//! - `genui_surface.rs` — GenUI intent Repository/Memento boundary
//! - `lifecycle_queries.rs` — discovered/running catalog projections
//! - `system_service.rs` — `SystemService` lifecycle syscall surface
//! - `command_dispatch.rs` — Command Router match table
//! - `lifecycle_commands.rs` — discover/start/stop/remove/load/status/snapshot/metadata/heartbeat
//! - `session_commands.rs` — session start/resume/stop
//! - `host_commands.rs` — host dispatch, agent delegate, GenUI surface query

mod command_dispatch;
mod genui_surface;
mod host_commands;
mod lifecycle_commands;
mod lifecycle_queries;
mod session_commands;
mod support;
mod system_service;
mod wasm_session;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_app::{
    application_service_descriptor, empty_domain_pack_catalog, AppRegistry, AppRuntime,
    SharedDomainPackCatalog,
};
use macaca_kernel::Kernel;
use macaca_proto::{
    ApplicationAgentDelegateCommand, ApplicationAgentDelegateResult, ApplicationId,
    ApplicationServiceSessionView, CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceError,
    ServiceResult, TraceContext,
};
use tokio::sync::RwLock;

use crate::wasm_runtime_provider::{WasmApplicationRuntimeProvider, WasmExecutionSession};
use crate::{ApplicationGenUiSurfaceStore, InMemoryServicePolicyEngine};

/// Runtime-host owned bridge for application-scoped orchestration.
///
/// This trait is the stable Adapter boundary between Application Service and
/// shell/runtime composition roots.  Application Service owns the provider-
/// neutral command contract, while Web or future hosts own the concrete
/// executor registry and loop lifecycle.  Keeping this interface in
/// `macaca-runtime-host` prevents the service provider from importing Web state
/// or concrete shell types while still allowing WASM guests to request
/// app-scoped agent work through ServiceRuntime.
#[async_trait]
pub trait ApplicationOrchestrationBackend: Send + Sync {
    /// Delegate work to an app-scoped agent and return a bounded replayable
    /// result.  Implementations must validate app/session/agent scope before
    /// starting any worker-side effect.
    async fn delegate_agent(
        &self,
        command: ApplicationAgentDelegateCommand,
    ) -> ServiceResult<ApplicationAgentDelegateResult>;
}

/// Host-owned Application Service provider backed by existing app primitives.
pub struct ApplicationSystemServiceProvider {
    pub(crate) descriptor: macaca_proto::ServiceDescriptor,
    pub(crate) registry: Option<Arc<RwLock<AppRegistry>>>,
    pub(crate) runtime: Option<Arc<AppRuntime>>,
    /// Host-installed catalog shared with `AppRuntime` for pack expansion.
    pub(crate) domain_pack_catalog: SharedDomainPackCatalog,
    pub(crate) kernel: Option<Arc<Kernel>>,
    pub(crate) wasm_policy_engine: Option<Arc<InMemoryServicePolicyEngine>>,
    pub(crate) wasm_runtime_provider: Option<Arc<dyn WasmApplicationRuntimeProvider>>,
    pub(crate) orchestration_backend: Option<Arc<dyn ApplicationOrchestrationBackend>>,
    pub(crate) sessions: Arc<RwLock<HashMap<String, ApplicationServiceSessionView>>>,
    pub(crate) wasm_sessions: Arc<RwLock<HashMap<ApplicationId, Arc<dyn WasmExecutionSession>>>>,
    pub(crate) genui_surfaces: ApplicationGenUiSurfaceStore,
}

impl ApplicationSystemServiceProvider {
    /// Create a provider backed by Web/runtime-owned application state.
    pub fn new(
        registry: Arc<RwLock<AppRegistry>>,
        runtime: Arc<AppRuntime>,
        domain_pack_catalog: SharedDomainPackCatalog,
        kernel: Arc<Kernel>,
        wasm_policy_engine: Arc<InMemoryServicePolicyEngine>,
        wasm_host_import_bridge: crate::wasm_runtime_provider::WasmHostImportBridge,
        orchestration_backend: Option<Arc<dyn ApplicationOrchestrationBackend>>,
    ) -> Self {
        let genui_surfaces = ApplicationGenUiSurfaceStore::default();
        let wasm_host_import_bridge =
            wasm_host_import_bridge.with_genui_surface_store(genui_surfaces.clone());
        let wasm_runtime_provider = Arc::new(
            crate::wasm_runtime_provider::ComponentModelWasmRuntimeProvider::default()
                .with_host_import_bridge(Arc::new(wasm_host_import_bridge)),
        );
        tracing::info!(
            service_id = %application_service_descriptor().id,
            "ApplicationSystemServiceProvider wired with composition-root domain-pack catalog"
        );
        Self {
            descriptor: application_service_descriptor(),
            registry: Some(registry),
            runtime: Some(runtime),
            domain_pack_catalog,
            kernel: Some(kernel),
            wasm_policy_engine: Some(wasm_policy_engine),
            wasm_runtime_provider: Some(wasm_runtime_provider),
            orchestration_backend,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            wasm_sessions: Arc::new(RwLock::new(HashMap::new())),
            genui_surfaces,
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: application_service_descriptor(),
            registry: None,
            runtime: None,
            domain_pack_catalog: empty_domain_pack_catalog(),
            kernel: None,
            wasm_policy_engine: None,
            wasm_runtime_provider: None,
            orchestration_backend: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            wasm_sessions: Arc::new(RwLock::new(HashMap::new())),
            genui_surfaces: ApplicationGenUiSurfaceStore::default(),
        }
    }

    /// Attach an orchestration backend to an existing provider.
    ///
    /// Tests and alternate host composition roots use this builder-style hook
    /// to prove that Application Service stays generic: the service provider
    /// owns command validation and result shaping, while the backend owns the
    /// concrete executor or remote orchestration transport.
    pub fn with_orchestration_backend(
        mut self,
        backend: Arc<dyn ApplicationOrchestrationBackend>,
    ) -> Self {
        self.orchestration_backend = Some(backend);
        self
    }

    pub(crate) fn registry(&self) -> ServiceResult<Arc<RwLock<AppRegistry>>> {
        self.registry.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("application registry is not configured".into())
        })
    }

    pub(crate) fn runtime(&self) -> ServiceResult<Arc<AppRuntime>> {
        self.runtime.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("application runtime is not configured".into())
        })
    }

    pub(crate) fn kernel(&self) -> ServiceResult<Arc<Kernel>> {
        self.kernel.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("application kernel handle is not configured".into())
        })
    }

    pub(crate) fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    pub(crate) fn service_result(
        output: serde_json::Value,
        trace: TraceContext,
    ) -> ServiceCallResult {
        ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        }
    }
}
