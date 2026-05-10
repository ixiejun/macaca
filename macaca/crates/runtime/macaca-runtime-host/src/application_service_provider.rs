//! Runtime-host adapter for the Route C Application Service.
//!
//! This module uses Adapter/Bridge: `ServiceRuntime` receives provider-neutral
//! commands while this provider delegates application semantics to `macaca-app`.
//! The host owns lifecycle dispatch, trace-required service execution, and
//! structured unavailable behavior; `macaca-app` remains the owner of manifests,
//! runtime assembly, ApplicationHost, ABI metadata, and lifecycle semantics.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_app::{
    app_entry_agent_name, app_status_from_lifecycle, application_service_descriptor,
    lifecycle_from_app_status, AppLoader, AppRegistry, AppRuntime, ApplicationHost,
    ApplicationRuntimeKindSpec, DiscoveredApp, UnavailableApplicationHostBackend,
};
use macaca_kernel::{Kernel, SystemService};
use macaca_proto::{
    ApplicationDiscoverCommand, ApplicationDiscoverResult, ApplicationGenUiSurfaceCommand,
    ApplicationHostDispatchResult, ApplicationHostDispatchServiceCommand, ApplicationId,
    ApplicationLoadCommand, ApplicationRemoveCommand, ApplicationServiceAgentView,
    ApplicationServiceAppView, ApplicationServiceRuntimeView, ApplicationServiceSessionView,
    ApplicationServiceSnapshot, ApplicationServiceUnavailable, ApplicationSessionResumeCommand,
    ApplicationSessionStartCommand, ApplicationSessionStopCommand, ApplicationSnapshotCommand,
    ApplicationStartCommand, ApplicationStatusCommand, ApplicationStatusResult,
    ApplicationStopCommand, CleanupPolicy, PackageRuntimeKind, ServiceCallResult, ServiceCommand,
    ServiceError, ServiceHealth, ServiceResult, TraceContext, APPLICATION_DISCOVER_COMMAND,
    APPLICATION_GENUI_SURFACE_COMMAND, APPLICATION_HOST_DISPATCH_COMMAND, APPLICATION_LOAD_COMMAND,
    APPLICATION_REMOVE_COMMAND, APPLICATION_SESSION_RESUME_COMMAND,
    APPLICATION_SESSION_START_COMMAND, APPLICATION_SESSION_STOP_COMMAND,
    APPLICATION_SNAPSHOT_COMMAND, APPLICATION_START_COMMAND, APPLICATION_STATUS_COMMAND,
    APPLICATION_STOP_COMMAND,
};
use tokio::sync::RwLock;

/// Host-owned Application Service provider backed by existing app primitives.
pub struct ApplicationSystemServiceProvider {
    descriptor: macaca_proto::ServiceDescriptor,
    registry: Option<Arc<RwLock<AppRegistry>>>,
    runtime: Option<Arc<AppRuntime>>,
    kernel: Option<Arc<Kernel>>,
    sessions: Arc<RwLock<HashMap<String, ApplicationServiceSessionView>>>,
}

impl ApplicationSystemServiceProvider {
    /// Create a provider backed by Web/runtime-owned application state.
    pub fn new(
        registry: Arc<RwLock<AppRegistry>>,
        runtime: Arc<AppRuntime>,
        kernel: Arc<Kernel>,
    ) -> Self {
        Self {
            descriptor: application_service_descriptor(),
            registry: Some(registry),
            runtime: Some(runtime),
            kernel: Some(kernel),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: application_service_descriptor(),
            registry: None,
            runtime: None,
            kernel: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
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
            let mut view = discovered
                .as_ref()
                .map(discovered_view)
                .unwrap_or_else(|| minimal_running_view(id, name));
            view.runtime.lifecycle_state = lifecycle_from_app_status(status);
            view.runtime.compatibility_status = format!(
                "{:?}",
                app_status_from_lifecycle(&view.runtime.lifecycle_state)
            );
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
                let host = ApplicationHost::new(UnavailableApplicationHostBackend);
                let result: ApplicationHostDispatchResult = host
                    .dispatch(typed.host_command)
                    .await
                    .map_err(service_adapter_error)?;
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            APPLICATION_GENUI_SURFACE_COMMAND => {
                let typed: ApplicationGenUiSurfaceCommand = decode(command.payload)?;
                let unavailable = ApplicationServiceUnavailable::new(
                    APPLICATION_GENUI_SURFACE_COMMAND,
                    "application-provided GenUI surface is unavailable in S7 provider",
                    Some(&typed.trace),
                );
                Ok(Self::service_result(to_value(unavailable)?, typed.trace))
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
    manifest_view(&app.manifest, Some(&app.path), false)
}

fn manifest_view(
    manifest: &macaca_app::AppManifest,
    app_dir: Option<&Path>,
    running: bool,
) -> ApplicationServiceAppView {
    let runtime_kind = match manifest.layer {
        macaca_app::AppLayer::L2Wasm => Some(PackageRuntimeKind::WasmComponent),
        _ => Some(PackageRuntimeKind::Yaml),
    };
    let runtime_spec = ApplicationRuntimeKindSpec;
    let execution_available = runtime_spec.execution_available_for_runtime(runtime_kind.as_ref());
    let lifecycle_state = if running {
        macaca_proto::ApplicationLifecycleState::Started
    } else if execution_available {
        macaca_proto::ApplicationLifecycleState::Initialized
    } else {
        macaca_proto::ApplicationLifecycleState::Failed {
            reason: "runtime unavailable".into(),
        }
    };
    let diagnostics = if execution_available {
        Vec::new()
    } else {
        vec!["runtime unavailable".into()]
    };
    ApplicationServiceAppView {
        id: manifest.id,
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        description: manifest.description.clone(),
        entry_agent: app_entry_agent_name(manifest).map(str::to_string),
        agents: manifest
            .agents
            .iter()
            .map(|agent| ApplicationServiceAgentView {
                name: match agent {
                    macaca_app::model::AgentSource::Inline(inline) => inline.name.clone(),
                    macaca_app::model::AgentSource::FilePath(path) => path.clone(),
                },
                capability_names: Vec::new(),
            })
            .collect(),
        runtime: ApplicationServiceRuntimeView {
            runtime_kind,
            lifecycle_state,
            compatibility_status: if running { "Running" } else { "Loaded" }.into(),
            app_dir: app_dir.map(|path| path.display().to_string()),
            skills_dir: app_dir.map(|path| path.join("skills").display().to_string()),
        },
        diagnostics,
    }
}

fn minimal_running_view(id: ApplicationId, name: String) -> ApplicationServiceAppView {
    ApplicationServiceAppView {
        id,
        name,
        version: "unknown".into(),
        description: None,
        entry_agent: None,
        agents: Vec::new(),
        runtime: ApplicationServiceRuntimeView {
            runtime_kind: Some(PackageRuntimeKind::Yaml),
            lifecycle_state: macaca_proto::ApplicationLifecycleState::Started,
            compatibility_status: "Running".into(),
            app_dir: None,
            skills_dir: None,
        },
        diagnostics: Vec::new(),
    }
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
