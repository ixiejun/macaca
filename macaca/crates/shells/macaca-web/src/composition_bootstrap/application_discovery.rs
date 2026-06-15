//! Bootstrap phase 4–5: application registry, service runtime, and app auto-start.
//!
//! Discovers applications from the workspace install root and starts each through the typed
//! Application Service boundary so skill directories and heartbeat profiles are collected generically.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tracing::{error, info, warn};

use super::bootstrap_path_helpers::autonomy_runtime_config_from_web_config;
use crate::wasm_orchestration_backend::WebApplicationOrchestrationBackend;
use macaca_host_composition::app::{AppLoader, AppRegistry, AppRuntime};
use macaca_proto::{ApplicationStartCommand, KernelServiceId, MacacaResult, TraceContext};
use macaca_sdk::SharedDomainPackCatalog;

use super::bootstrap_ctx::BootstrapCtx;

/// Run the `application-discovery` bootstrap slice.
pub(crate) async fn run(ctx: &mut BootstrapCtx) -> MacacaResult<()> {
    let config = ctx.config.clone().expect("bootstrap: config");
    let kernel = Arc::clone(ctx.kernel.as_ref().expect("bootstrap: kernel"));
    let llm = Arc::clone(ctx.llm.as_ref().expect("bootstrap: llm"));
    let llm_router = Arc::clone(ctx.llm_router.as_ref().expect("bootstrap: llm_router"));

    // 4. Initialize app registry and discover apps from the configured
    // workspace install root. This keeps app installation/discovery under one
    // industrial-grade control point: `{workspace.root_dir}/apps`.
    let app_scan_dir = AppRegistry::workspace_apps_dir(&config.workspace.root_dir);
    let mut registry = AppRegistry::with_dirs(vec![app_scan_dir.clone()]);
    let discovered = registry.discover_apps()?;
    info!(
        count = discovered.len(),
        app_scan_dir = %app_scan_dir.display(),
        "Apps discovered from workspace application directory"
    );

    // 5. Compose the installed domain-pack catalog before constructing runtime
    // and Application Service so pack expansion uses the same host-owned view.
    let domain_pack_catalog: SharedDomainPackCatalog =
        super::domain_pack_wiring::build_installed_domain_pack_catalog();

    // Start the runtime and load ALL discovered apps.
    let runtime = Arc::new(AppRuntime::with_domain_pack_catalog(Arc::clone(
        &domain_pack_catalog,
    )));
    let registry = Arc::new(tokio::sync::RwLock::new(registry));
    let service_runtime = Arc::new(
        macaca_host_composition::service_runtime::ServiceRuntime::new(
            macaca_host_composition::service_runtime::ServiceRuntimeConfig::default(),
        ),
    );
    let autonomy_runtime = macaca_host_composition::autonomy_runtime::bootstrap_autonomy_services(
        Arc::clone(&service_runtime),
        "web-startup-autonomy",
        autonomy_runtime_config_from_web_config(&config.autonomy),
    )
    .await?;
    info!(
        provider_mode = %autonomy_runtime.provider_mode,
        services = autonomy_runtime.started_services.len(),
        supervisor_present = autonomy_runtime.supervisor.is_some(),
        "Autonomy runtime bootstrapped through runtime-host"
    );
    let application_orchestration_registry_ref = Arc::new(tokio::sync::RwLock::new(None));
    let app_workspaces = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
    let orchestration_backend = Arc::new(WebApplicationOrchestrationBackend::new(
        Arc::clone(&application_orchestration_registry_ref),
        Arc::clone(&service_runtime),
        Arc::clone(&app_workspaces),
    ));
    let mut app_dirs = HashMap::new();
    let mut skills_dirs = Vec::new();
    let mut started_apps: Vec<(macaca_proto::ApplicationId, String, Vec<String>)> = Vec::new();
    // Compose one shared audit bundle so replay commands and WASM host-import
    // routing can observe the same service-call evidence chain.
    let service_audit_bundle =
        macaca_host_composition::service_runtime::ServiceAuditRuntimeBundle::in_memory();
    // Host runtime wiring point for future production WASM runtime enablement.
    // Keeping this bridge bound to the shared sink guarantees audit continuity
    // once L2 WASM execution path is enabled in this host.
    let wasm_host_import_bridge = service_audit_bundle.wasm_host_import_bridge(
        Arc::clone(&service_runtime),
        macaca_host_composition::application_bootstrap::wasm_runtime_provider::WasmHostImportBridgeConfig::default(),
    );
    service_runtime
        .register_provider(
            &macaca_host_composition::service_runtime::StaticServiceProviderFactory::new(
                macaca_host_composition::service_runtime::ServiceProviderInstance::new(
                    macaca_host_composition::app::application_service_descriptor(),
                    Arc::new(
                        macaca_host_composition::application_bootstrap::ApplicationSystemServiceProvider::new(
                            Arc::clone(&registry),
                            Arc::clone(&runtime),
                            Arc::clone(&domain_pack_catalog),
                            Arc::clone(&kernel),
                            wasm_host_import_bridge.policy_engine(),
                            wasm_host_import_bridge.clone(),
                            Some(Arc::clone(&orchestration_backend)
                                as Arc<
                                    dyn macaca_host_composition::application_bootstrap::ApplicationOrchestrationBackend,
                                >),
                        ),
                    ),
                ),
            ),
            macaca_host_composition::service_runtime::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_proto::APPLICATION_SERVICE_ID),
            TraceContext::new("web-startup-application-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_host_composition::service_runtime::StaticServiceProviderFactory::new(
                service_audit_bundle.audit_service_provider_instance(),
            ),
            macaca_host_composition::service_runtime::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(
                macaca_host_composition::service_runtime::SERVICE_CALL_AUDIT_SERVICE_ID,
            ),
            TraceContext::new("web-startup-service-call-audit-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_host_composition::service_runtime::StaticServiceProviderFactory::new(
                macaca_host_composition::service_runtime::ServiceProviderInstance::new(
                    macaca_host_composition::application_bootstrap::plugin_control_service_descriptor(),
                    Arc::new(
                        macaca_host_composition::application_bootstrap::PluginControlSystemServiceProvider::in_memory(),
                    ),
                ),
            ),
            macaca_host_composition::service_runtime::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &macaca_proto::plugin_control_service_id(),
            TraceContext::new("web-startup-plugin-control-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_host_composition::service_runtime::StaticServiceProviderFactory::new(
                macaca_host_composition::service_runtime::ServiceProviderInstance::new(
                    macaca_host_composition::application_bootstrap::plugin_capability_service_descriptor(),
                    Arc::new(
                        macaca_host_composition::application_bootstrap::PluginCapabilitySystemServiceProvider::in_memory(
                        ),
                    ),
                ),
            ),
            macaca_host_composition::service_runtime::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &macaca_proto::plugin_capability_registry_service_id(),
            TraceContext::new("web-startup-plugin-capability-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_host_composition::service_runtime::StaticServiceProviderFactory::new(
                macaca_host_composition::service_runtime::ServiceProviderInstance::new(
                    macaca_host_composition::application_bootstrap::plugin_hook_service_descriptor(),
                    Arc::new(
                        macaca_host_composition::application_bootstrap::PluginHookSystemServiceProvider::in_memory(),
                    ),
                ),
            ),
            macaca_host_composition::service_runtime::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &macaca_proto::plugin_hook_bus_service_id(),
            TraceContext::new("web-startup-plugin-hook-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    let generic_service_client: Arc<dyn macaca_sdk::SystemServiceClient> = Arc::new(
        macaca_host_composition::HostRuntimeSystemServiceClient::new(
            Arc::clone(&service_runtime),
            "macaca.web",
        ),
    );
    let application_client: Arc<dyn macaca_sdk::SystemApplicationClient> = Arc::new(
        macaca_sdk::ServiceBackedApplicationClient::new(Arc::clone(&generic_service_client)),
    );

    // Auto-start all discovered apps through the service boundary.  The
    // Application Service provider owns the runtime implementation internally,
    // while Web observes startup through typed, traceable service commands and
    // sanitized result views before loading app-local skills from the returned
    // runtime metadata.
    for app in &discovered {
        let manifest_path = app.manifest_path.clone();
        if manifest_path.exists() {
            let trace = TraceContext::new(format!("web-startup-application-{}", app.id));
            let command = ApplicationStartCommand {
                trace: trace.clone(),
                manifest_path: Some(manifest_path.display().to_string()),
                manifest: None,
                app_dir: Some(app.path.display().to_string()),
                policy: Default::default(),
            };
            match application_client.start(command).await {
                Ok(view) => {
                    let agent_count = kernel.agent_count().await;
                    if let Some(app_dir) = view.runtime.app_dir.as_deref() {
                        app_dirs.insert(view.id, PathBuf::from(app_dir));
                    } else {
                        app_dirs.insert(view.id, app.path.clone());
                    }
                    if let Some(skills_dir) = view.runtime.skills_dir.as_deref() {
                        skills_dirs.push(PathBuf::from(skills_dir));
                    } else {
                        skills_dirs.push(app.path.join("skills"));
                    }
                    let app_agent_names: Vec<String> =
                        view.agents.iter().map(|agent| agent.name.clone()).collect();
                    started_apps.push((view.id, view.name.clone(), app_agent_names));
                    if let Some(supervisor) = autonomy_runtime.supervisor.as_ref() {
                        match AppLoader::load_manifest(&manifest_path) {
                            Ok(manifest) => {
                                let profile_trace = TraceContext::new(format!(
                                    "web-startup-application-heartbeat-profile-{}",
                                    view.id
                                ));
                                match supervisor.register_application_heartbeat_profile(
                                    &manifest,
                                    profile_trace,
                                ) {
                                    Ok(registered) => {
                                        info!(
                                            app_id = %view.id,
                                            registered,
                                            "Application heartbeat profile registration evaluated"
                                        );
                                    }
                                    Err(error) => {
                                        error!(
                                            app_id = %view.id,
                                            error = %error,
                                            "Application heartbeat profile registration failed"
                                        );
                                    }
                                }
                            }
                            Err(error) => {
                                error!(
                                    app_id = %view.id,
                                    error = %error,
                                    "Application manifest reload failed during heartbeat profile registration"
                                );
                            }
                        }
                    }
                    info!(
                        service_id = "application",
                        command = "start",
                        application_id = %view.id,
                        agent_count = agent_count,
                        trace_id = %trace.trace_id,
                        "Application started through Application Service"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        service_id = "application",
                        command = "start",
                        reason_code = "start_failed_continue_bootstrap",
                        application_id = %app.id,
                        error = %e,
                        trace_id = %trace.trace_id,
                        "Application Service failed to start application; continuing Web startup"
                    );
                }
            }
        }
    }

    ctx.runtime = Some(runtime);
    ctx.domain_pack_catalog = Some(domain_pack_catalog);
    ctx.registry = Some(registry);
    ctx.discovered = Some(discovered);
    ctx.service_runtime = Some(service_runtime);
    ctx.autonomy_runtime = Some(autonomy_runtime);
    ctx.application_orchestration_registry_ref = Some(application_orchestration_registry_ref);
    ctx.app_workspaces = Some(app_workspaces);
    ctx.orchestration_backend = Some(orchestration_backend);
    ctx.app_dirs = Some(app_dirs);
    ctx.skills_dirs = Some(skills_dirs);
    ctx.started_apps = Some(started_apps);
    ctx.service_audit_bundle = Some(service_audit_bundle);
    ctx.wasm_host_import_bridge = Some(wasm_host_import_bridge);
    ctx.generic_service_client = Some(generic_service_client);
    ctx.application_client = Some(application_client);
    Ok(())
}
