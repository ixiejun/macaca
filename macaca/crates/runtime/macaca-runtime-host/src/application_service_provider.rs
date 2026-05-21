//! Runtime-host adapter for the Route C Application Service.
//!
//! This module uses Adapter/Bridge: `ServiceRuntime` receives provider-neutral
//! commands while this provider delegates application semantics to `macaca-app`.
//! The host owns lifecycle dispatch, trace-required service execution, and
//! structured unavailable behavior; `macaca-app` remains the owner of manifests,
//! runtime assembly, ApplicationHost, ABI metadata, and lifecycle semantics.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_app::{
    app_manifest_to_heartbeat_agent_views, app_manifest_to_metadata_view,
    app_manifest_to_service_app_view, application_service_descriptor, expand_service_capabilities,
    AppLoader, AppRegistry, AppRuntime, AppStatus, ApplicationHost, DiscoveredApp,
    InMemoryDomainPackCatalog, UnavailableApplicationHostBackend,
};
use macaca_kernel::{Kernel, SystemService};
use macaca_proto::{
    ApplicationAgentDelegateCommand, ApplicationAgentDelegateResult, ApplicationDiscoverCommand,
    ApplicationDiscoverResult, ApplicationGenUiSurfaceCommand,
    ApplicationHeartbeatAgentsQueryCommand, ApplicationHeartbeatAgentsResult,
    ApplicationHostDispatchResult, ApplicationHostDispatchServiceCommand, ApplicationId,
    ApplicationLoadCommand, ApplicationMetadataQueryCommand, ApplicationMetadataResult,
    ApplicationRemoveCommand, ApplicationServiceAppView, ApplicationServiceRuntimeView,
    ApplicationServiceSessionView, ApplicationServiceSnapshot, ApplicationServiceUnavailable,
    ApplicationSessionResumeCommand, ApplicationSessionStartCommand, ApplicationSessionStopCommand,
    ApplicationSnapshotCommand, ApplicationStartCommand, ApplicationStatusCommand,
    ApplicationStatusResult, ApplicationStopCommand, CleanupPolicy, PackageRuntimeKind,
    ServiceCallResult, ServiceCommand, ServiceError, ServiceHealth, ServiceResult, TraceContext,
    UiIntent, WasmExecutionProfile, WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
    APPLICATION_AGENT_DELEGATE_COMMAND, APPLICATION_DISCOVER_COMMAND,
    APPLICATION_GENUI_SURFACE_COMMAND, APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND,
    APPLICATION_HOST_DISPATCH_COMMAND, APPLICATION_LOAD_COMMAND,
    APPLICATION_METADATA_QUERY_COMMAND, APPLICATION_REMOVE_COMMAND,
    APPLICATION_SESSION_RESUME_COMMAND, APPLICATION_SESSION_START_COMMAND,
    APPLICATION_SESSION_STOP_COMMAND, APPLICATION_SNAPSHOT_COMMAND, APPLICATION_START_COMMAND,
    APPLICATION_STATUS_COMMAND, APPLICATION_STOP_COMMAND,
};
use tokio::sync::RwLock;

use crate::wasm_runtime_provider::{
    ComponentModelWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmExecutionSession,
    WasmHostImportBridge,
};
use crate::{ApplicationGenUiSurfaceStore, InMemoryServicePolicyEngine, ServicePolicyLayer};

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
    descriptor: macaca_proto::ServiceDescriptor,
    registry: Option<Arc<RwLock<AppRegistry>>>,
    runtime: Option<Arc<AppRuntime>>,
    kernel: Option<Arc<Kernel>>,
    wasm_policy_engine: Option<Arc<InMemoryServicePolicyEngine>>,
    wasm_runtime_provider: Option<Arc<dyn WasmApplicationRuntimeProvider>>,
    orchestration_backend: Option<Arc<dyn ApplicationOrchestrationBackend>>,
    sessions: Arc<RwLock<HashMap<String, ApplicationServiceSessionView>>>,
    wasm_sessions: Arc<RwLock<HashMap<ApplicationId, Arc<dyn WasmExecutionSession>>>>,
    genui_surfaces: ApplicationGenUiSurfaceStore,
}

impl ApplicationSystemServiceProvider {
    /// Create a provider backed by Web/runtime-owned application state.
    pub fn new(
        registry: Arc<RwLock<AppRegistry>>,
        runtime: Arc<AppRuntime>,
        kernel: Arc<Kernel>,
        wasm_policy_engine: Arc<InMemoryServicePolicyEngine>,
        wasm_host_import_bridge: WasmHostImportBridge,
        orchestration_backend: Option<Arc<dyn ApplicationOrchestrationBackend>>,
    ) -> Self {
        let genui_surfaces = ApplicationGenUiSurfaceStore::default();
        let wasm_host_import_bridge =
            wasm_host_import_bridge.with_genui_surface_store(genui_surfaces.clone());
        let wasm_runtime_provider = Arc::new(
            ComponentModelWasmRuntimeProvider::default()
                .with_host_import_bridge(Arc::new(wasm_host_import_bridge)),
        );
        Self {
            descriptor: application_service_descriptor(),
            registry: Some(registry),
            runtime: Some(runtime),
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

    fn registry(&self) -> ServiceResult<Arc<RwLock<AppRegistry>>> {
        self.registry.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("application registry is not configured".into())
        })
    }

    fn runtime(&self) -> ServiceResult<Arc<AppRuntime>> {
        self.runtime.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("application runtime is not configured".into())
        })
    }

    fn kernel(&self) -> ServiceResult<Arc<Kernel>> {
        self.kernel.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                "application kernel compatibility handle is not configured".into(),
            )
        })
    }

    /// Resolve one app-scoped WASM execution session, creating it lazily.
    ///
    /// The session key is `application_id` and the setup is completely generic:
    /// - ability id comes from sanitized metadata projection (`ability.runtime.wasm` fallback),
    /// - artifact path uses the discovered app install root (`component.wasm`),
    /// - runtime profile is provider default (`wasm_component`).
    ///
    /// This keeps host dispatch app-agnostic and prevents any application-
    /// specific business logic from leaking into the infrastructure layer.
    async fn ensure_wasm_session(
        &self,
        app_id: ApplicationId,
        trace: TraceContext,
    ) -> ServiceResult<Option<Arc<dyn WasmExecutionSession>>> {
        let Some(provider) = self.wasm_runtime_provider.as_ref() else {
            return Ok(None);
        };
        if let Some(existing) = self.wasm_sessions.read().await.get(&app_id).cloned() {
            return Ok(Some(existing));
        }

        let registry = self.registry()?;
        let discovered = {
            let guard = registry.read().await;
            guard.get_app(&app_id).cloned()
        };
        let Some(discovered) = discovered else {
            return Ok(None);
        };
        if discovered.manifest.layer != macaca_app::AppLayer::L2Wasm {
            return Ok(None);
        }

        // The manifest v1 YAML adapter synthesizes one runtime ability with a
        // stable id (`ability.runtime.wasm`) for L2 WASM apps. We use that
        // neutral contract id directly so session creation stays deterministic
        // and does not depend on optional metadata-view projection arguments.
        let ability_id = "ability.runtime.wasm".to_string();
        let artifact_path = discovered.path.join("component.wasm");
        let request = WasmRuntimeSessionRequest::new(
            trace.clone(),
            app_id.to_string(),
            ability_id.clone(),
            WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
            WasmExecutionProfile::default_wasm_component(),
        )
        .map_err(service_adapter_error)?;
        let session = Arc::from(
            provider
                .create_session(request)
                .await
                .map_err(service_adapter_error)?,
        );
        tracing::info!(
            trace_id = %trace.trace_id,
            app_id = %app_id,
            ability_id = %ability_id,
            artifact = %artifact_path.display(),
            "Created app-scoped WASM execution session for application host dispatch"
        );
        self.wasm_sessions
            .write()
            .await
            .insert(app_id, Arc::clone(&session));
        Ok(Some(session))
    }

    /// Install app-scoped WASM service-call allowlist from manifest contract.
    ///
    /// This is a generic mapping layer:
    /// - input: app `service_contract` declaration (+ domain packs),
    /// - output: policy engine app override (`allow_services`) consumed by host
    ///   import `service.call` router.
    ///
    /// The mapping is app-agnostic and deny-by-default for undeclared services.
    async fn sync_wasm_service_policy_for_app(&self, app_id: &ApplicationId) -> ServiceResult<()> {
        let Some(engine) = self.wasm_policy_engine.as_ref() else {
            return Ok(());
        };
        let Some(registry) = self.registry.as_ref() else {
            return Ok(());
        };
        let discovered = {
            let guard = registry.read().await;
            guard.get_app(app_id).cloned()
        };
        let Some(discovered) = discovered else {
            tracing::warn!(app_id = %app_id, "Skipping WASM policy sync because app is not discovered");
            return Ok(());
        };
        let catalog = InMemoryDomainPackCatalog::with_builtin_defaults();
        let expanded =
            expand_service_capabilities(discovered.manifest.service_contract.as_ref(), &catalog);
        engine.set_app_override(
            app_id.to_string(),
            ServicePolicyLayer {
                allow_services: BTreeSet::from_iter(expanded.services.iter().cloned()),
                deny_services: BTreeSet::new(),
                timeout_ms: None,
                max_retries: None,
            },
        );
        tracing::info!(
            app_id = %app_id,
            service_count = expanded.services.len(),
            capabilities_hash = %expanded.capabilities_hash,
            unresolved_pack_count = expanded.unresolved_packs.len(),
            "Synchronized app-scoped WASM service policy from manifest contract"
        );
        Ok(())
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn service_result(output: serde_json::Value, trace: TraceContext) -> ServiceCallResult {
        ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        }
    }

    /// Store the latest validated GenUI intent for one app/session/surface.
    ///
    /// This is a host-owned Repository/Memento boundary.  WASM guests and
    /// future application runtimes emit declarative UI intent data, while
    /// Application Service owns the replayable lookup surface consumed by Web,
    /// Desktop, CLI inspectors, or audit tooling.  Keeping this store here
    /// prevents presentation shells from inventing application-specific state
    /// maps and keeps render queries traceable through the same service facade
    /// used for lifecycle and host dispatch commands.
    #[cfg(test)]
    async fn store_genui_surface(&self, intent: UiIntent) -> ServiceResult<()> {
        tracing::info!(
            app_id = %intent.app_id,
            session_id = %intent.session_id,
            surface_id = %intent.surface_id,
            trace_id = intent.trace.as_ref().map(|trace| trace.trace_id.as_str()).unwrap_or("none"),
            root_component = %intent.tree.root.id,
            "application service stored GenUI session surface"
        );
        self.genui_surfaces.store(intent).await
    }

    /// Query the latest GenUI intent for one app/session/surface.
    ///
    /// A missing value is a first-class `None`, not an error.  That preserves
    /// the existing chat shell fallback for applications that have not emitted
    /// a declarative surface yet.
    async fn get_genui_surface(
        &self,
        command: &ApplicationGenUiSurfaceCommand,
    ) -> ServiceResult<Option<UiIntent>> {
        let app_id = command.scope.application_id.ok_or_else(|| {
            ServiceError::AdapterFailure("application.genui.surface requires application_id".into())
        })?;
        let session_id = command
            .scope
            .session_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                ServiceError::AdapterFailure("application.genui.surface requires session_id".into())
            })?;
        let surface_id = command
            .surface_id
            .as_deref()
            .filter(|value| !value.trim().is_empty());
        let surface = self
            .genui_surfaces
            .get(&app_id.to_string(), session_id, surface_id)
            .await?;
        tracing::info!(
            trace_id = %command.trace.trace_id,
            app_id = %app_id,
            session_id,
            surface_id = surface_id.unwrap_or("default"),
            found = surface.is_some(),
            "application service queried GenUI session surface"
        );
        Ok(surface)
    }

    async fn discovered_views(
        registry: &Arc<RwLock<AppRegistry>>,
    ) -> ServiceResult<Vec<ApplicationServiceAppView>> {
        let registry = registry.read().await;
        Ok(registry
            .list_apps()
            .into_iter()
            .map(discovered_view)
            .collect())
    }

    async fn running_views(
        runtime: &Arc<AppRuntime>,
        registry: Option<&Arc<RwLock<AppRegistry>>>,
    ) -> ServiceResult<Vec<ApplicationServiceAppView>> {
        let apps = runtime.list_apps().await;
        let mut views = Vec::new();
        for (id, name, status) in apps {
            let discovered = if let Some(registry) = registry {
                let guard = registry.read().await;
                guard.get_app(&id).cloned()
            } else {
                None
            };
            let view = discovered
                .as_ref()
                .map(|app| app_manifest_to_service_app_view(&app.manifest, Some(&app.path), status))
                .unwrap_or_else(|| minimal_running_view(id, name, status));
            views.push(view);
        }
        Ok(views)
    }
}

#[async_trait]
impl SystemService for ApplicationSystemServiceProvider {
    fn descriptor(&self) -> macaca_proto::ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = %self.descriptor.id,
            registry_configured = self.registry.is_some(),
            runtime_configured = self.runtime.is_some(),
            "application service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "application service command accepted"
        );
        match command.name.as_str() {
            APPLICATION_DISCOVER_COMMAND => {
                let typed: ApplicationDiscoverCommand = decode(command.payload)?;
                let registry = self.registry()?;
                let mut guard = registry.write().await;
                let discovered = guard.discover_apps().map_err(service_adapter_error)?;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = discovered.len(),
                    "application service discovery completed"
                );
                let views: ApplicationDiscoverResult =
                    discovered.iter().map(discovered_view).collect();
                Ok(Self::service_result(to_value(views)?, typed.trace))
            }
            APPLICATION_START_COMMAND => {
                let typed: ApplicationStartCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                let kernel = self.kernel()?;
                let manifest_path = typed.manifest_path.as_deref().ok_or_else(|| {
                    ServiceError::AdapterFailure(
                        "application.start requires manifest_path in S7".into(),
                    )
                })?;
                let path = Path::new(manifest_path);
                #[allow(deprecated)]
                let app_id = runtime
                    .start_app_from_file(path, &kernel)
                    .await
                    .map_err(service_adapter_error)?;
                let app_dir = path.parent().unwrap_or_else(|| Path::new("."));
                let manifest = AppLoader::load_manifest(path).map_err(service_adapter_error)?;
                let view = manifest_view(&manifest, Some(app_dir), true);
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    app_id = %app_id,
                    app_name = %view.name,
                    "application service start completed"
                );
                Ok(Self::service_result(to_value(view)?, typed.trace))
            }
            APPLICATION_STATUS_COMMAND => {
                let typed: ApplicationStatusCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                let result: ApplicationStatusResult =
                    Self::running_views(&runtime, self.registry.as_ref()).await?;
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            APPLICATION_METADATA_QUERY_COMMAND => {
                let typed: ApplicationMetadataQueryCommand = decode(command.payload)?;
                let registry = self.registry()?;
                let app_id = typed.scope.application_id.ok_or_else(|| {
                    ServiceError::AdapterFailure(
                        "application.metadata.query requires application_id".into(),
                    )
                })?;
                let discovered = {
                    let guard = registry.read().await;
                    guard.get_app(&app_id).cloned()
                }
                .ok_or_else(|| {
                    ServiceError::AdapterFailure(format!("application {app_id} not found"))
                })?;
                let status = running_status_for(self.runtime.as_ref(), &app_id).await;
                let view: ApplicationMetadataResult = app_manifest_to_metadata_view(
                    &discovered.manifest,
                    Some(&discovered.path),
                    status,
                    typed.include_abilities,
                    typed.include_policy,
                    typed.include_overlay,
                    typed.include_digest,
                );
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    app_id = %app_id,
                    ability_count = view.abilities.len(),
                    "application service metadata query completed"
                );
                Ok(Self::service_result(to_value(view)?, typed.trace))
            }
            APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND => {
                let typed: ApplicationHeartbeatAgentsQueryCommand = decode(command.payload)?;
                let registry = self.registry()?;
                let app_id = typed.scope.application_id.ok_or_else(|| {
                    ServiceError::AdapterFailure(
                        "application.heartbeat.agents.query requires application_id".into(),
                    )
                })?;
                let discovered = {
                    let guard = registry.read().await;
                    guard.get_app(&app_id).cloned()
                }
                .ok_or_else(|| {
                    ServiceError::AdapterFailure(format!("application {app_id} not found"))
                })?;
                let views: ApplicationHeartbeatAgentsResult =
                    app_manifest_to_heartbeat_agent_views(&discovered.manifest);
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    app_id = %app_id,
                    declaration_count = views.len(),
                    enabled_count = views.iter().filter(|view| view.enabled).count(),
                    invalid_count = views.iter().filter(|view| !view.diagnostics.is_empty()).count(),
                    "application service heartbeat agent query completed"
                );
                Ok(Self::service_result(to_value(views)?, typed.trace))
            }
            APPLICATION_SNAPSHOT_COMMAND => {
                let typed: ApplicationSnapshotCommand = decode(command.payload)?;
                let discovered = if typed.include_discovered {
                    if let Some(registry) = &self.registry {
                        Self::discovered_views(registry).await?
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
                let running = if typed.include_running {
                    if let Some(runtime) = &self.runtime {
                        Self::running_views(runtime, self.registry.as_ref()).await?
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
                let sessions = self.sessions.read().await.values().cloned().collect();
                let snapshot = ApplicationServiceSnapshot::healthy(discovered, running, sessions);
                tracing::info!(trace_id = %typed.trace.trace_id, "application service snapshot emitted");
                Ok(Self::service_result(to_value(snapshot)?, typed.trace))
            }
            APPLICATION_SESSION_START_COMMAND => {
                let typed: ApplicationSessionStartCommand = decode(command.payload)?;
                let view = session_view(&typed.scope, "running")?;
                self.sync_wasm_service_policy_for_app(&view.application_id)
                    .await?;
                self.sessions
                    .write()
                    .await
                    .insert(view.session_id.clone(), view.clone());
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    app_id = %view.application_id,
                    session_id = %view.session_id,
                    "application service session started"
                );
                Ok(Self::service_result(to_value(view)?, typed.trace))
            }
            APPLICATION_SESSION_RESUME_COMMAND => {
                let typed: ApplicationSessionResumeCommand = decode(command.payload)?;
                let view = session_view(&typed.scope, "resumed")?;
                self.sessions
                    .write()
                    .await
                    .insert(view.session_id.clone(), view.clone());
                Ok(Self::service_result(to_value(view)?, typed.trace))
            }
            APPLICATION_SESSION_STOP_COMMAND => {
                let typed: ApplicationSessionStopCommand = decode(command.payload)?;
                let view = session_view(&typed.scope, "stopped")?;
                self.sessions.write().await.remove(&view.session_id);
                Ok(Self::service_result(to_value(view)?, typed.trace))
            }
            APPLICATION_HOST_DISPATCH_COMMAND => {
                let typed: ApplicationHostDispatchServiceCommand = decode(command.payload)?;
                let app_id = typed.scope.application_id.ok_or_else(|| {
                    ServiceError::AdapterFailure(
                        "application.host.dispatch requires application_id".into(),
                    )
                })?;
                let result: ApplicationHostDispatchResult = if let Some(session) = self
                    .ensure_wasm_session(app_id, typed.trace.clone())
                    .await?
                {
                    session
                        .dispatch(typed.host_command)
                        .await
                        .map_err(service_adapter_error)?
                } else {
                    let host = ApplicationHost::new(UnavailableApplicationHostBackend);
                    host.dispatch(typed.host_command)
                        .await
                        .map_err(service_adapter_error)?
                };
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            APPLICATION_AGENT_DELEGATE_COMMAND => {
                let typed: ApplicationAgentDelegateCommand = decode(command.payload)?;
                let result_trace = typed.trace.clone();
                let app_id = typed.scope.application_id.ok_or_else(|| {
                    ServiceError::AdapterFailure(
                        "application.agent.delegate requires application_id".into(),
                    )
                })?;
                let session_id = typed
                    .scope
                    .session_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        ServiceError::AdapterFailure(
                            "application.agent.delegate requires session_id".into(),
                        )
                    })?;
                if typed.target_agent.trim().is_empty() {
                    return Err(ServiceError::AdapterFailure(
                        "application.agent.delegate requires target_agent".into(),
                    ));
                }
                if typed.prompt.trim().is_empty() {
                    return Err(ServiceError::AdapterFailure(
                        "application.agent.delegate requires prompt".into(),
                    ));
                }
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    app_id = %app_id,
                    session_id,
                    target_agent = %typed.target_agent,
                    "application service accepted app-scoped agent delegation"
                );
                let result = if let Some(backend) = &self.orchestration_backend {
                    backend.delegate_agent(typed).await?
                } else {
                    ApplicationAgentDelegateResult {
                        application_id: app_id,
                        session_id: session_id.to_string(),
                        target_agent: typed.target_agent,
                        task_id: None,
                        success: false,
                        output: serde_json::json!({
                            "reason": "application orchestration backend is not configured"
                        }),
                        status: "unavailable".into(),
                        metadata: BTreeMap::from([(
                            "reason_code".into(),
                            "orchestration_backend_unavailable".into(),
                        )]),
                    }
                };
                Ok(Self::service_result(to_value(result)?, result_trace))
            }
            APPLICATION_GENUI_SURFACE_COMMAND => {
                let typed: ApplicationGenUiSurfaceCommand = decode(command.payload)?;
                let surface = self.get_genui_surface(&typed).await?;
                Ok(Self::service_result(to_value(surface)?, typed.trace))
            }
            APPLICATION_LOAD_COMMAND => {
                let typed: ApplicationLoadCommand = decode(command.payload)?;
                let unavailable = ApplicationServiceUnavailable::new(
                    APPLICATION_LOAD_COMMAND,
                    "application load/admission is metadata-only in this S7 provider path",
                    Some(&typed.trace),
                );
                Ok(Self::service_result(to_value(unavailable)?, typed.trace))
            }
            APPLICATION_STOP_COMMAND => {
                let typed: ApplicationStopCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                let kernel = self.kernel()?;
                let app_id = typed.scope.application_id.ok_or_else(|| {
                    ServiceError::AdapterFailure("application.stop requires application_id".into())
                })?;
                runtime
                    .stop_app(&app_id, &kernel)
                    .await
                    .map_err(service_adapter_error)?;
                Ok(Self::service_result(
                    serde_json::json!({"application_id": app_id, "status": "stopped"}),
                    typed.trace,
                ))
            }
            APPLICATION_REMOVE_COMMAND => {
                let typed: ApplicationRemoveCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                let app_id = typed.scope.application_id.ok_or_else(|| {
                    ServiceError::AdapterFailure(
                        "application.remove requires application_id".into(),
                    )
                })?;
                runtime
                    .remove_app(&app_id)
                    .await
                    .map_err(service_adapter_error)?;
                Ok(Self::service_result(
                    serde_json::json!({"application_id": app_id, "status": "removed"}),
                    typed.trace,
                ))
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Application service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "application service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.sessions.write().await.clear();
        self.wasm_sessions.write().await.clear();
        self.genui_surfaces.clear().await;
        tracing::info!(service_id = %self.descriptor.id, "application service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.registry.is_some() && self.runtime.is_some() && self.kernel.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: "application provider dependencies are not fully configured".into(),
            })
        }
    }
}

fn discovered_view(app: &DiscoveredApp) -> ApplicationServiceAppView {
    app_manifest_to_service_app_view(&app.manifest, Some(&app.path), AppStatus::Loaded)
}

fn manifest_view(
    manifest: &macaca_app::AppManifest,
    app_dir: Option<&Path>,
    running: bool,
) -> ApplicationServiceAppView {
    let status = if running {
        AppStatus::Running
    } else {
        AppStatus::Loaded
    };
    app_manifest_to_service_app_view(manifest, app_dir, status)
}

fn minimal_running_view(
    id: ApplicationId,
    name: String,
    status: AppStatus,
) -> ApplicationServiceAppView {
    ApplicationServiceAppView {
        id,
        name,
        version: "unknown".into(),
        description: None,
        entry_agent: None,
        agents: Vec::new(),
        runtime: ApplicationServiceRuntimeView {
            runtime_kind: Some(PackageRuntimeKind::Yaml),
            lifecycle_state: macaca_app::lifecycle_from_app_status(status),
            compatibility_status: format!("{status:?}"),
            app_dir: None,
            skills_dir: None,
        },
        ui: None,
        diagnostics: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use macaca_kernel::SystemService;
    use macaca_proto::{
        ApplicationAgentDelegateCommand, ApplicationAgentDelegateResult,
        ApplicationGenUiSurfaceCommand, ApplicationServiceScope, ServiceCommand,
        ServiceCommandName, TraceContext, UiComponent, UiComponentKind, UiComponentTree, UiIntent,
        UiRenderSurface, APPLICATION_AGENT_DELEGATE_COMMAND, APPLICATION_GENUI_SURFACE_COMMAND,
    };
    use serde_json::json;

    use super::*;

    struct FakeOrchestrationBackend;

    #[async_trait]
    impl ApplicationOrchestrationBackend for FakeOrchestrationBackend {
        async fn delegate_agent(
            &self,
            command: ApplicationAgentDelegateCommand,
        ) -> ServiceResult<ApplicationAgentDelegateResult> {
            Ok(ApplicationAgentDelegateResult {
                application_id: command.scope.application_id.unwrap(),
                session_id: command.scope.session_id.unwrap(),
                target_agent: command.target_agent,
                task_id: Some("task-from-fake-backend".into()),
                success: true,
                output: json!({"status": "queued"}),
                status: "queued".into(),
                metadata: BTreeMap::from([("reason_code".into(), "delegate_queued".into())]),
            })
        }
    }

    fn card_intent(app_id: ApplicationId, session_id: &str, surface_id: &str) -> UiIntent {
        let trace = TraceContext::new("test-genui-render");
        UiIntent {
            app_id: app_id.to_string(),
            session_id: session_id.to_string(),
            surface_id: UiRenderSurface::new(surface_id),
            tree: UiComponentTree {
                root: UiComponent::new(
                    "session-card",
                    UiComponentKind::Card,
                    json!({
                        "title": "Session Surface",
                        "body": "Schema-defined session card"
                    }),
                ),
                trace_markers: Vec::new(),
                metadata: Default::default(),
            },
            permission_prompts: Vec::new(),
            approval_prompts: Vec::new(),
            trace: Some(trace),
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn genui_surface_query_returns_stored_session_surface() {
        let provider = ApplicationSystemServiceProvider::unavailable();
        let app_id = ApplicationId::new();
        let session_id = "session-genui-test";
        let surface_id = "session-surface";
        let intent = card_intent(app_id, session_id, surface_id);

        provider
            .store_genui_surface(intent.clone())
            .await
            .expect("test intent should store");

        let command = ApplicationGenUiSurfaceCommand {
            trace: TraceContext::new("test-genui-query"),
            scope: ApplicationServiceScope {
                application_id: Some(app_id),
                application_name: None,
                session_id: Some(session_id.to_string()),
                agent_name: None,
            },
            surface_id: Some(surface_id.to_string()),
        };
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(APPLICATION_GENUI_SURFACE_COMMAND),
                serde_json::to_value(command).unwrap(),
                TraceContext::new("test-genui-query"),
            ))
            .await
            .unwrap();
        assert_eq!(result.trace.trace_id, "test-genui-query");
        let decoded: Option<UiIntent> = serde_json::from_value(result.output).unwrap();

        assert_eq!(decoded, Some(intent));
    }

    #[tokio::test]
    async fn genui_surface_query_without_surface_id_returns_latest_session_surface() {
        let provider = ApplicationSystemServiceProvider::unavailable();
        let app_id = ApplicationId::new();
        let session_id = "session-genui-default-test";
        let intent = card_intent(app_id, session_id, "non-default-surface");

        provider
            .store_genui_surface(intent.clone())
            .await
            .expect("test intent should store");

        let command = ApplicationGenUiSurfaceCommand {
            trace: TraceContext::new("test-genui-default-query"),
            scope: ApplicationServiceScope {
                application_id: Some(app_id),
                application_name: None,
                session_id: Some(session_id.to_string()),
                agent_name: None,
            },
            surface_id: None,
        };
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(APPLICATION_GENUI_SURFACE_COMMAND),
                serde_json::to_value(command).unwrap(),
                TraceContext::new("test-genui-default-query"),
            ))
            .await
            .unwrap();
        let decoded: Option<UiIntent> = serde_json::from_value(result.output).unwrap();

        assert_eq!(decoded, Some(intent));
    }

    #[tokio::test]
    async fn agent_delegate_without_backend_returns_structured_unavailable() {
        let provider = ApplicationSystemServiceProvider::unavailable();
        let app_id = ApplicationId::new();
        let command = ApplicationAgentDelegateCommand {
            trace: TraceContext::new("test-agent-delegate-unavailable"),
            scope: ApplicationServiceScope {
                application_id: Some(app_id),
                application_name: None,
                session_id: Some("session-agent-unavailable".into()),
                agent_name: Some("wasm-guest".into()),
            },
            target_agent: "analyst".into(),
            prompt: "Analyze BTC".into(),
            context: json!({"symbol": "BTC"}),
            metadata: BTreeMap::new(),
        };

        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(APPLICATION_AGENT_DELEGATE_COMMAND),
                serde_json::to_value(command).unwrap(),
                TraceContext::new("test-agent-delegate-unavailable"),
            ))
            .await
            .unwrap();
        let decoded: ApplicationAgentDelegateResult =
            serde_json::from_value(result.output).unwrap();

        assert!(!decoded.success);
        assert_eq!(decoded.status, "unavailable");
        assert_eq!(
            decoded.metadata.get("reason_code").map(String::as_str),
            Some("orchestration_backend_unavailable")
        );
    }

    #[tokio::test]
    async fn agent_delegate_uses_injected_backend() {
        let provider = ApplicationSystemServiceProvider::unavailable()
            .with_orchestration_backend(Arc::new(FakeOrchestrationBackend));
        let app_id = ApplicationId::new();
        let command = ApplicationAgentDelegateCommand {
            trace: TraceContext::new("test-agent-delegate-backend"),
            scope: ApplicationServiceScope {
                application_id: Some(app_id),
                application_name: None,
                session_id: Some("session-agent-backend".into()),
                agent_name: Some("wasm-guest".into()),
            },
            target_agent: "analyst".into(),
            prompt: "Analyze BTC".into(),
            context: json!({"symbol": "BTC"}),
            metadata: BTreeMap::new(),
        };

        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(APPLICATION_AGENT_DELEGATE_COMMAND),
                serde_json::to_value(command).unwrap(),
                TraceContext::new("test-agent-delegate-backend"),
            ))
            .await
            .unwrap();
        let decoded: ApplicationAgentDelegateResult =
            serde_json::from_value(result.output).unwrap();

        assert!(decoded.success);
        assert_eq!(decoded.task_id.as_deref(), Some("task-from-fake-backend"));
        assert_eq!(decoded.status, "queued");
    }
}

async fn running_status_for(
    runtime: Option<&Arc<AppRuntime>>,
    app_id: &ApplicationId,
) -> AppStatus {
    if let Some(runtime) = runtime {
        for (id, _, status) in runtime.list_apps().await {
            if id == *app_id {
                return status;
            }
        }
    }
    AppStatus::Loaded
}

fn session_view(
    scope: &macaca_proto::ApplicationServiceScope,
    status: &str,
) -> ServiceResult<ApplicationServiceSessionView> {
    let application_id = scope.application_id.ok_or_else(|| {
        ServiceError::AdapterFailure("application session command requires application_id".into())
    })?;
    let session_id = scope.session_id.clone().ok_or_else(|| {
        ServiceError::AdapterFailure("application session command requires session_id".into())
    })?;
    Ok(ApplicationServiceSessionView::new(
        application_id,
        session_id,
        scope.agent_name.clone(),
        status,
    ))
}

fn decode<T: serde::de::DeserializeOwned>(payload: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(payload).map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}

fn to_value<T: serde::Serialize>(value: T) -> ServiceResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}

fn service_adapter_error(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
