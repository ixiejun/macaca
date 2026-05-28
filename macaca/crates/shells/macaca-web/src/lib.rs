//! `macaca-web` — Simple web UI for Macaca OS.
//!
//! Provides an HTTP server with a single-page web interface for interacting
//! with Macaca OS applications. Uses axum for the HTTP layer.

mod agent_context_backend;
mod agent_execution_backend;
mod agent_execution_evidence;
pub mod agent_runner;
pub mod app_ui_routes;
pub mod bootstrap;
mod capability_catalog;
pub mod chat_mediator;
pub mod chat_orchestrator;
mod context_memory_injection;
mod context_memory_tools;
mod context_message_codec;
mod context_report_events;
mod context_reporting_memory;
mod context_reporting_model;
pub mod event_persistence;
mod external_context_adapter;
pub mod framework_runner;
pub mod framework_toolkit;
pub mod genui_routes;
pub mod heartbeat_operations_routes;
pub mod hook_consumer;
pub mod loop_manager;
mod memory_runtime;
pub mod metrics;
pub mod orchestration_tools;
mod persistence_adapter;
pub mod plugin_routes;
pub mod proto_event_visitors;
pub mod route_command;
pub mod routes;
pub mod run_trace;
pub mod runtime_event_bridge;
pub mod runtime_resume;
mod scheduled_agent_task_tool;
mod service_runtime_client;
mod service_tool_adapter;
pub mod session;
mod session_memory_capture;
pub mod session_replay;
pub mod shell;
pub mod skill_mcp;
pub mod skill_operations_routes;
pub mod skill_self_evolution_audit;
mod skill_self_evolution_execution_observer;
pub mod skill_self_evolution_observer;
mod skill_usage_telemetry;
mod source_artifact;
pub mod sse;
pub mod state;
pub mod tool_routes;
pub mod trace_events;
mod wasm_orchestration_backend;
pub mod web3_status;
pub mod workspace;
mod workspace_knowledge_digest_capability;
mod workspace_memory_recall_source;

use std::time::Duration;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tracing::{error, info, warn};

use macaca_app::{AppLoader, AppRegistry, AppRuntime};
use macaca_framework::session::{
    InMemorySessionStore as FrameworkInMemorySessionStore, SessionStore as FrameworkSessionStore,
};
use macaca_kernel::{
    AgentInfo, ApplicationExecutorRegistry, KernelBuilder, KernelServiceClientCompat,
};
use macaca_llm::{LlmProvider, LlmRouter};
use macaca_persist::RedbStore;
use macaca_proto::config::{AutonomyConfig, KernelConfig, MacacaConfig};
use macaca_proto::{ApplicationStartCommand, KernelServiceId, MacacaResult, TraceContext};
use macaca_skill::{ExecutableSkillToolSet, SkillCatalog};
use macaca_tools::{DefaultToolSet, Tool};

use crate::agent_context_backend::WebAgentContextBackend;
use crate::agent_execution_backend::WebAgentExecutionBackend;
use crate::agent_runner::WebAgentRunner;
pub use crate::bootstrap::{WebRuntimeFacade, WebServerBuilder};
use crate::external_context_adapter::install_external_adapters_from_config;
use crate::orchestration_tools::build_web_tools;
use crate::persistence_adapter::RedbKernelPersistenceAdapter;
use crate::skill_self_evolution_execution_observer::SkillSelfEvolutionObservedAgentExecutionBackend;
use crate::state::{AppConfig, AppState, LoopState, PersistenceState, SessionState};
use crate::wasm_orchestration_backend::WebApplicationOrchestrationBackend;

/// Start the Macaca OS web server.
#[deprecated(note = "Use WebServerBuilder::new().port(port).serve() instead")]
pub async fn start_server(port: u16) -> MacacaResult<()> {
    WebServerBuilder::new().port(port).serve().await
}

pub(crate) async fn serve_web_server(port: u16) -> MacacaResult<()> {
    // 1. Load configuration from config/default.toml
    let config = MacacaConfig::load_default();
    info!(default_provider = %config.llm.default_provider, "Configuration loaded");

    // 1b. Publish [mcp.env] entries into the current process environment so
    //     every stdio MCP child process (which inherits parent env by default)
    //     automatically receives secrets such as FIGMA_API_KEY.
    let mcp_env_outcomes =
        macaca_runtime_host::RuntimeEnvBuilder::apply_process_env(&config.mcp.env);
    if !mcp_env_outcomes.is_empty() {
        info!(
            entries = mcp_env_outcomes.len(),
            "Applied [mcp.env] to process environment for MCP child inheritance"
        );
    }

    // 2. Create LLM router/provider registry from configuration.
    let llm_router = Arc::new(LlmRouter::from_config(&config.llm)?);
    let llm: Arc<dyn LlmProvider> = llm_router.clone();

    info!(provider = llm.name(), "LLM provider initialized");

    // 3. Create kernel.
    let kernel_config = KernelConfig {
        max_agents: 64,
        heartbeat_interval_ms: 5000,
        agent_timeout_ms: 60000,
    };
    let kernel = Arc::new(
        KernelBuilder::from_service_clients(
            kernel_config,
            KernelServiceClientCompat::from_agent_provider_boxed_tools(
                Arc::clone(&llm),
                Box::new(DefaultToolSet::new()),
            ),
        )
        .build(),
    );

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

    // 5. Start the runtime and load ALL discovered apps.
    let runtime = Arc::new(AppRuntime::new());
    let registry = Arc::new(tokio::sync::RwLock::new(registry));
    let service_runtime = Arc::new(macaca_runtime_host::ServiceRuntime::new(
        macaca_runtime_host::ServiceRuntimeConfig::default(),
    ));
    let autonomy_runtime = macaca_runtime_host::bootstrap_autonomy_services(
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
    let orchestration_backend = Arc::new(WebApplicationOrchestrationBackend::new(
        Arc::clone(&application_orchestration_registry_ref),
        Arc::clone(&service_runtime),
    ));
    let mut app_dirs = HashMap::new();
    let mut skills_dirs = Vec::new();
    let mut started_apps: Vec<(macaca_proto::ApplicationId, String, Vec<String>)> = Vec::new();
    // Compose one shared audit bundle so replay commands and WASM host-import
    // routing can observe the same service-call evidence chain.
    let service_audit_bundle = macaca_runtime_host::ServiceAuditRuntimeBundle::in_memory();
    // Host runtime wiring point for future production WASM runtime enablement.
    // Keeping this bridge bound to the shared sink guarantees audit continuity
    // once L2 WASM execution path is enabled in this host.
    let wasm_host_import_bridge = service_audit_bundle.wasm_host_import_bridge(
        Arc::clone(&service_runtime),
        macaca_runtime_host::wasm_runtime_provider::WasmHostImportBridgeConfig::default(),
    );
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_app::application_service_descriptor(),
                    Arc::new(macaca_runtime_host::ApplicationSystemServiceProvider::new(
                        Arc::clone(&registry),
                        Arc::clone(&runtime),
                        Arc::clone(&kernel),
                        wasm_host_import_bridge.policy_engine(),
                        wasm_host_import_bridge.clone(),
                        Some(orchestration_backend),
                    )),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
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
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                service_audit_bundle.audit_service_provider_instance(),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_runtime_host::SERVICE_CALL_AUDIT_SERVICE_ID),
            TraceContext::new("web-startup-service-call-audit-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_runtime_host::plugin_control_service_descriptor(),
                    Arc::new(macaca_runtime_host::PluginControlSystemServiceProvider::in_memory()),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
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
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_runtime_host::plugin_capability_service_descriptor(),
                    Arc::new(
                        macaca_runtime_host::PluginCapabilitySystemServiceProvider::in_memory(),
                    ),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
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
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_runtime_host::plugin_hook_service_descriptor(),
                    Arc::new(macaca_runtime_host::PluginHookSystemServiceProvider::in_memory()),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
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
        crate::service_runtime_client::WebRuntimeSystemServiceClient::new(
            Arc::clone(&service_runtime),
            "macaca.web",
        ),
    );
    let application_client: Arc<dyn macaca_sdk::SystemApplicationClient> = Arc::new(
        macaca_sdk::ServiceBackedApplicationClient::new(Arc::clone(&generic_service_client)),
    );

    // Auto-start all discovered apps through the service boundary.  The
    // Application Service provider still delegates to the legacy runtime
    // implementation internally, but Web now observes startup through typed,
    // traceable service commands and sanitized result views before loading
    // app-local skills from the returned runtime metadata.
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
                        app_id = %view.id,
                        app_name = %view.name,
                        agents = agent_count,
                        trace_id = %trace.trace_id,
                        "App started through Application Service"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        app_name = %app.name,
                        error = %e,
                        trace_id = %trace.trace_id,
                        "Application Service failed to start app; continuing Web startup"
                    );
                }
            }
        }
    }

    // 6. Load skill catalog (SKILL.md knowledge skills).
    let mut catalog = SkillCatalog::new();
    for dir in &skills_dirs {
        if dir.exists() {
            match catalog.load_from_directory(&dir).await {
                Ok(n) => info!(count = n, "Knowledge skills loaded into catalog"),
                Err(e) => tracing::warn!("Failed to load knowledge skills: {e}"),
            }
        }
    }

    // 7. Build composite toolset: built-in tools + executable skill tools.
    let mut all_tools: Vec<Box<dyn Tool>> = vec![
        Box::new(macaca_tools::FileReadTool),
        Box::new(macaca_tools::FileWriteTool),
        Box::new(macaca_tools::ShellTool::default()),
    ];

    // Load executable skill tools from all app skills directories.
    for dir in &skills_dirs {
        if dir.exists() {
            let mut skill_tools = ExecutableSkillToolSet::new();
            match skill_tools.load_from_directory(dir).await {
                Ok(n) => {
                    let skill_tools = skill_tools.into_tools();
                    info!(count = n, "Executable skill tools loaded");
                    all_tools.extend(skill_tools);
                }
                Err(e) => tracing::warn!("Failed to load executable skills: {e}"),
            }
        }
    }

    // Load external driver plugins from configured directory.
    // Driver tools are NOT added to the static CompositeToolSet; instead they
    // are aggregated dynamically from `DriverRegistry` in `build_toolkit` so
    // that `/api/drivers/reload` picks up new driver tools at runtime.
    let drivers_dir =
        std::env::var("MACACA_DRIVERS_DIR").unwrap_or_else(|_| config.drivers.directory.clone());
    let driver_registry = Arc::new(macaca_driver::DriverRegistry::new());
    let driver_runtime = Arc::new(macaca_driver::DriverRuntime::new(
        drivers_dir.clone(),
        Arc::clone(&driver_registry),
    ));
    if config.drivers.auto_load {
        let report = driver_runtime.load_all().await;
        for entry in &report.entries {
            match entry.status {
                macaca_driver::DriverLoadStatus::Loaded => {
                    info!(
                        name = %entry.name,
                        tools = entry.tool_count.unwrap_or_default(),
                        "External driver loaded"
                    );
                }
                macaca_driver::DriverLoadStatus::Failed => {
                    error!(
                        name = %entry.name,
                        error = %entry.error.as_deref().unwrap_or("unknown error"),
                        "Failed to load external driver"
                    );
                }
            }
        }
    }

    // 8. Initialize orchestration tools.
    let tool_assembly = build_web_tools(Arc::clone(&kernel), all_tools);
    let tools = tool_assembly.tools;
    let executor_registry_ref = tool_assembly.executor_registry_ref;
    let delegate_session_id = tool_assembly.delegate_session_id;

    // 9. Initialize persistent session store.
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("macaca");
    std::fs::create_dir_all(&data_dir).ok();
    let session_db_path = data_dir.join("sessions.db");
    let session_store_impl = Arc::new(RedbStore::open(&session_db_path)?);
    let session_store_shared: Arc<dyn macaca_persist::PersistBackend> = session_store_impl.clone();
    let todo_store = Arc::new(macaca_task::TodoStore::new(Arc::clone(
        &session_store_shared,
    )));
    let event_log = Arc::new(macaca_persist::EventLog::new(Arc::clone(
        &session_store_impl,
    )));
    macaca_runtime_host::bootstrap_interaction_service(
        Arc::clone(&service_runtime),
        Arc::clone(&session_store_shared),
        Some(Arc::clone(&event_log)),
        "web-startup-interaction-service",
    )
    .await?;
    let entitlement_store: Arc<dyn macaca_persist::EntitlementStore> =
        Arc::new(macaca_persist::InMemoryEntitlementStore::new());
    let payment_store: Arc<dyn macaca_persist::PaymentStore> =
        Arc::new(macaca_persist::InMemoryPaymentStore::new());
    let entitlement_facade = Arc::new(
        macaca_runtime_host::EntitlementRuntimeFacade::with_event_log(
            Arc::clone(&entitlement_store),
            Arc::clone(&event_log),
        ),
    );
    let run_tracer = Arc::new(crate::run_trace::RunTracer::new(Arc::clone(&event_log)));
    info!(path = %session_db_path.display(), "Session store initialized");

    // 9a. Initialize audit logger and alert manager.
    let kernel_persistence = Arc::new(RedbKernelPersistenceAdapter::new(Arc::clone(
        &session_store_impl,
    )));
    let audit_logger = Arc::new(macaca_kernel::audit::AuditLogger::new(kernel_persistence));
    let session_store = session_store_shared;
    let alert_config = macaca_kernel::alert::AlertConfig::default();
    let alert_manager = Arc::new(macaca_kernel::alert::AlertManager::new(alert_config));
    info!("AuditLogger and AlertManager initialized");

    let default_model = llm_router.default_model_reference();
    let framework_session_store: Arc<dyn FrameworkSessionStore> =
        Arc::new(FrameworkInMemorySessionStore::new());
    let mcp_runtime = Arc::new(macaca_runtime_host::McpRuntimeFacade::load_default().await);

    let (memory_runtime, workspace_memory, workspace_memory_tombstones) =
        if config.context.recall.expose_memory_tools {
            let mem_dir = configured_memory_base_path(&data_dir, &config.memory.file_store_path);
            std::fs::create_dir_all(&mem_dir).ok();
            let factory = macaca_memory::MemoryBackendFactory::new(
                macaca_memory::MemoryBackendConfig::new(mem_dir.clone())
                    .session_ttl(Duration::from_secs(
                        config.memory.session_ttl_seconds.max(1),
                    ))
                    .enable_vector(config.context.active_vector_memory.enabled)
                    .vector_backend(
                        config.memory.vector.backend.clone(),
                        config.memory.vector.milvus_url.clone(),
                        config.memory.vector.collection_name.clone(),
                    )
                    .embedding_provider(
                        config.memory.embedding.provider.clone(),
                        config.memory.embedding.model.clone(),
                        config.memory.embedding.api_key.clone(),
                        config.memory.embedding.base_url.clone(),
                        config.memory.embedding.dimensions,
                    ),
            );
            let profile = factory
                .configured_profile()
                .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
            let tomb = Arc::new(macaca_memory::SharedTombstoneRegistry::new());
            info!(
                memory_base_path = %mem_dir.display(),
                vector_backend = %profile.vector_backend,
                vector_collection = ?profile.vector_collection,
                embedding_provider = %profile.embedding_provider,
                embedding_model = %profile.embedding_model,
                embedding_dimensions = profile.embedding_dimensions,
                "Configured workspace memory runtime"
            );
            let configured_manager = Arc::new(
                factory
                    .configured_manager()
                    .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?,
            );
            let provider_profile = format!(
                "{}:{}:{}",
                profile.vector_backend,
                profile
                    .vector_collection
                    .clone()
                    .unwrap_or_else(|| "vector-disabled".into()),
                profile.embedding_provider
            );
            let runtime = Arc::new(
                crate::memory_runtime::WebMemoryRuntime::from_configured_memory(
                    configured_manager,
                    provider_profile,
                ),
            );
            (
                Some(runtime),
                None::<Arc<macaca_memory::TestMemoryManager>>,
                Some(Arc::clone(&tomb)),
            )
        } else {
            warn!("Workspace memory runtime disabled by context recall configuration");
            (None, None, None)
        };
    let memory_client: Arc<dyn macaca_sdk::SystemMemoryClient> = if memory_runtime.is_some() {
        Arc::new(macaca_sdk::ServiceBackedMemoryClient::new(Arc::clone(
            &generic_service_client,
        )))
    } else {
        Arc::new(macaca_sdk::UnavailableSystemMemoryClient)
    };
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_llm::llm_service_descriptor(),
                    Arc::new(macaca_runtime_host::LlmSystemServiceProvider::new(
                        Arc::clone(&llm),
                    )),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_driver::driver_service_descriptor(),
                    Arc::new(macaca_runtime_host::DriverSystemServiceProvider::new(
                        Arc::clone(&driver_runtime),
                    )),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    let materialized_skill_roots =
        materialized_skill_recovery_roots(&config.workspace.root_dir, &skills_dirs);
    let governance_event_journal_path =
        skill_governance_event_journal_path(&config.workspace.root_dir);
    let skill_service_provider = if let Some(runtime) = memory_runtime.as_ref() {
        macaca_runtime_host::SkillSystemServiceProvider::new()
            .with_materialized_skill_roots(materialized_skill_roots.clone())
            .with_governance_event_journal_path(governance_event_journal_path.clone())
            .with_memory_runtime(Arc::clone(runtime) as Arc<dyn macaca_memory::MemoryRuntimeFacade>)
    } else {
        macaca_runtime_host::SkillSystemServiceProvider::new()
            .with_materialized_skill_roots(materialized_skill_roots.clone())
            .with_governance_event_journal_path(governance_event_journal_path.clone())
    };
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_skill::skill_service_descriptor(),
                    Arc::new(skill_service_provider),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_runtime_host::mcp_service_descriptor(),
                    Arc::new(macaca_runtime_host::McpSystemServiceProvider::new(
                        Arc::clone(&mcp_runtime),
                    )),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    if let Some(runtime) = memory_runtime.as_ref() {
        let memory_service: Arc<dyn macaca_kernel::SystemService> =
            Arc::new(macaca_runtime_host::MemorySystemServiceProvider::new(
                Arc::clone(runtime) as Arc<dyn macaca_memory::MemoryFacade>,
            ));
        service_runtime
            .register_provider(
                &macaca_runtime_host::StaticServiceProviderFactory::new(
                    macaca_runtime_host::ServiceProviderInstance::new(
                        macaca_memory::memory_service_descriptor(),
                        memory_service,
                    ),
                ),
                macaca_runtime_host::ServiceProviderFactoryContext::new(),
            )
            .await
            .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
        service_runtime
            .start(
                &KernelServiceId::new(macaca_memory::MEMORY_SERVICE_ID),
                TraceContext::new("web-startup-memory-service"),
            )
            .await
            .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    }
    let configured_external_adapters = install_external_adapters_from_config(&config.context)?;
    let external_adapter_installations = configured_external_adapters.installations.clone();
    let external_adapter_runtime_registry = Arc::new(
        crate::state::ExternalAdapterRuntimeRegistry::new()
            .with_installations(external_adapter_installations),
    );
    let context_engine_registry = Arc::new(configured_external_adapters.registry);
    let context_service_capabilities = macaca_context::ContextServiceRuntimeCapabilities {
        memory_recall: if let Some(runtime) = memory_runtime.as_ref() {
            crate::context_reporting_memory::build_context_service_recall_capability(
                &config.context,
                Arc::clone(runtime) as Arc<dyn macaca_sdk::SystemMemoryClient>,
                workspace_memory_tombstones.as_ref(),
            )
        } else {
            None
        },
        knowledge_digest: None,
    };
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_proto::ServiceDescriptor::new(
                        KernelServiceId::new(macaca_context::CONTEXT_SERVICE_ID),
                        macaca_proto::ServiceType::new("context"),
                        macaca_proto::TraceSchemaRef::new("trace.system_service.context.v1"),
                    ),
                    Arc::new(
                        macaca_runtime_host::ContextSystemServiceProvider::with_capabilities(
                            (*context_engine_registry).clone(),
                            context_service_capabilities,
                        ),
                    ),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_context::CONTEXT_SERVICE_ID),
            TraceContext::new("web-startup-context-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_llm::LLM_SERVICE_ID),
            TraceContext::new("web-startup-llm-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_driver::DRIVER_SERVICE_ID),
            TraceContext::new("web-startup-driver-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_skill::SKILL_SERVICE_ID),
            TraceContext::new("web-startup-skill-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_proto::MCP_SERVICE_ID),
            TraceContext::new("web-startup-mcp-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    macaca_runtime_host::bootstrap_local_file_service(
        Arc::clone(&service_runtime),
        "web-startup-file-service",
    )
    .await?;
    macaca_runtime_host::bootstrap_local_approval_service(
        Arc::clone(&service_runtime),
        "web-startup-approval-service",
    )
    .await?;
    macaca_runtime_host::bootstrap_tool_planning_service(
        Arc::clone(&service_runtime),
        Arc::new(macaca_runtime_host::industrial_tool_planning_service()?),
        "web-startup-tool-service",
    )
    .await?;
    // Built-in domain-pack services are registered at the generic service
    // boundary, not in any individual application path.  WASM applications
    // declare packs and service ids in their manifest; the host simply makes a
    // reusable service surface available for every contract-driven app.
    let domain_pack_services = macaca_runtime_host::bootstrap_builtin_domain_pack_services(
        Arc::clone(&service_runtime),
        Arc::clone(&llm),
        "web-startup-domain-pack",
    )
    .await?;
    info!(
        services = domain_pack_services.started_services.len(),
        "Built-in domain-pack services bootstrapped through runtime-host"
    );
    // S12 thin-shell completion moves S9-S11 service lifecycle ownership into
    // `macaca-runtime-host`.  Web still provides the existing local stores and
    // facade handles, but provider registration/start semantics now cross a
    // typed host bootstrap boundary instead of living in presentation startup.
    let route_c_optional_services = macaca_runtime_host::bootstrap_route_c_optional_services(
        Arc::clone(&service_runtime),
        macaca_runtime_host::RouteCOptionalServicesBootstrapInputs::new(
            Arc::clone(&entitlement_store),
            Arc::clone(&entitlement_facade),
            Arc::clone(&payment_store),
            macaca_kernel::local_simulated_terms("1", "UNIT"),
        ),
        "web-startup",
    )
    .await?;
    info!(
        services = route_c_optional_services.started_services.len(),
        "Route C optional services bootstrapped through runtime-host"
    );
    let llm_client: Arc<dyn macaca_sdk::SystemLlmClient> = Arc::new(
        macaca_sdk::ServiceBackedLlmClient::new(Arc::clone(&generic_service_client)),
    );
    let context_client: Arc<dyn macaca_sdk::SystemContextClient> = Arc::new(
        macaca_sdk::ServiceBackedContextClient::new(Arc::clone(&generic_service_client)),
    );
    let driver_client: Arc<dyn macaca_sdk::SystemDriverClient> = Arc::new(
        macaca_sdk::ServiceBackedDriverClient::new(Arc::clone(&generic_service_client)),
    );
    let skill_client: Arc<dyn macaca_sdk::SystemSkillClient> = Arc::new(
        macaca_sdk::ServiceBackedSkillClient::new(Arc::clone(&generic_service_client)),
    );
    let mcp_client: Arc<dyn macaca_sdk::SystemMcpClient> = Arc::new(
        macaca_sdk::ServiceBackedMcpClient::new(Arc::clone(&generic_service_client)),
    );
    let tool_client: Arc<dyn macaca_sdk::SystemToolClient> = Arc::new(
        macaca_sdk::ServiceBackedToolClient::new(Arc::clone(&generic_service_client)),
    );
    let store_client: Arc<dyn macaca_sdk::SystemStoreClient> = Arc::new(
        macaca_sdk::ServiceBackedStoreClient::new(Arc::clone(&generic_service_client)),
    );
    let entitlement_client: Arc<dyn macaca_sdk::SystemEntitlementClient> = Arc::new(
        macaca_sdk::ServiceBackedEntitlementClient::new(Arc::clone(&generic_service_client)),
    );
    let payment_client: Arc<dyn macaca_sdk::SystemPaymentClient> = Arc::new(
        macaca_sdk::ServiceBackedPaymentClient::new(Arc::clone(&generic_service_client)),
    );
    let scheduler_client: Arc<dyn macaca_sdk::SystemSchedulerClient> = Arc::new(
        macaca_sdk::ServiceBackedSchedulerClient::new(Arc::clone(&generic_service_client)),
    );
    let scheduled_agent_task_client: Arc<dyn macaca_sdk::SystemScheduledAgentTaskClient> = Arc::new(
        macaca_sdk::ServiceBackedScheduledAgentTaskClient::new(Arc::clone(&generic_service_client)),
    );
    let heartbeat_client: Arc<dyn macaca_sdk::SystemHeartbeatClient> = Arc::new(
        macaca_sdk::ServiceBackedHeartbeatClient::new(Arc::clone(&generic_service_client)),
    );
    let web3_client: Arc<dyn macaca_sdk::SystemWeb3Client> = Arc::new(
        macaca_sdk::ServiceBackedWeb3Client::new(Arc::clone(&generic_service_client)),
    );
    let evm_client: Arc<dyn macaca_sdk::SystemEvmClient> = Arc::new(
        macaca_sdk::ServiceBackedEvmClient::new(Arc::clone(&generic_service_client)),
    );
    let plugin_control_client: Arc<dyn macaca_sdk::SystemPluginControlClient> = Arc::new(
        macaca_sdk::ServiceBackedPluginControlClient::new(Arc::clone(&generic_service_client)),
    );
    let plugin_capability_client: Arc<dyn macaca_sdk::SystemPluginCapabilityClient> = Arc::new(
        macaca_sdk::ServiceBackedPluginCapabilityClient::new(Arc::clone(&generic_service_client)),
    );
    let plugin_hook_client: Arc<dyn macaca_sdk::SystemPluginHookClient> = Arc::new(
        macaca_sdk::ServiceBackedPluginHookClient::new(Arc::clone(&generic_service_client)),
    );
    let system_facade = crate::shell::WebSystemFacadeBundle::new(
        Arc::clone(&generic_service_client),
        Arc::clone(&web3_client),
        Arc::clone(&evm_client),
        Arc::clone(&plugin_control_client),
        Arc::clone(&plugin_capability_client),
        Arc::clone(&plugin_hook_client),
    );

    // 10. Build shared state.
    let state = Arc::new_cyclic(|weak_state| {
        // Create the real agent runner with the actual weak state
        let runner = Arc::new(WebAgentRunner::new(weak_state.clone()));
        let executor_registry = Arc::new(ApplicationExecutorRegistry::new(
            Arc::clone(&runner) as Arc<dyn macaca_kernel::AgentRunner>
        ));

        AppState {
            kernel: kernel.clone(),
            runtime: runtime.clone(),
            registry: Arc::clone(&registry),
            application_client: Arc::clone(&application_client),
            llm_client: Arc::clone(&llm_client),
            memory_client: Arc::clone(&memory_client),
            context_client: Arc::clone(&context_client),
            driver_client: Arc::clone(&driver_client),
            skill_client: Arc::clone(&skill_client),
            mcp_client: Arc::clone(&mcp_client),
            tool_client: Arc::clone(&tool_client),
            store_client: Arc::clone(&store_client),
            entitlement_client: Arc::clone(&entitlement_client),
            payment_client: Arc::clone(&payment_client),
            scheduler_client: Arc::clone(&scheduler_client),
            scheduled_agent_task_client: Arc::clone(&scheduled_agent_task_client),
            heartbeat_client: Arc::clone(&heartbeat_client),
            web3_client: Arc::clone(&web3_client),
            evm_client: Arc::clone(&evm_client),
            plugin_control_client: Arc::clone(&plugin_control_client),
            plugin_capability_client: Arc::clone(&plugin_capability_client),
            plugin_hook_client: Arc::clone(&plugin_hook_client),
            system_facade: system_facade.clone(),
            service_runtime: Arc::clone(&service_runtime),
            autonomy_runtime: autonomy_runtime.clone(),
            llm: llm.clone(),
            llm_router: llm_router.clone(),
            tools,
            executor_registry: executor_registry.clone(),
            memory_runtime: memory_runtime.clone(),
            workspace_memory: workspace_memory.clone(),
            workspace_memory_tombstones: workspace_memory_tombstones.clone(),
            mcp_runtime: Arc::clone(&mcp_runtime),
            driver_registry: Arc::clone(&driver_registry),
            driver_runtime: Arc::clone(&driver_runtime),
            drivers_dir: drivers_dir.clone(),
            persist: PersistenceState {
                session_store,
                todo_store,
                event_log: event_log.clone(),
                audit_logger: audit_logger.clone(),
                run_tracer: Arc::clone(&run_tracer),
            },
            loops: LoopState {
                plan_loop_handles: tokio::sync::RwLock::new(HashMap::new()),
                worker_loop_handles: tokio::sync::RwLock::new(HashMap::new()),
                plan_loop_wakers: tokio::sync::RwLock::new(HashMap::new()),
                worker_loop_wakers: tokio::sync::RwLock::new(HashMap::new()),
                scheduler_handles: tokio::sync::RwLock::new(HashMap::new()),
            },
            sessions: SessionState {
                conversations: tokio::sync::RwLock::new(HashMap::new()),
                cancel_flags: tokio::sync::RwLock::new(HashMap::new()),
                active_sessions: tokio::sync::RwLock::new(HashMap::new()),
                fork_to_session: tokio::sync::RwLock::new(HashMap::new()),
                goal_to_session: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
                delegate_session_id: Arc::clone(&delegate_session_id),
                framework_session_store: Arc::clone(&framework_session_store),
            },
            config: AppConfig {
                app_dirs: tokio::sync::RwLock::new(app_dirs),
                app_workspaces: tokio::sync::RwLock::new(HashMap::new()),
                default_model,
                context: config.context.clone(),
                catalog: tokio::sync::RwLock::new(catalog),
                alert_manager: alert_manager.clone(),
            },
            context_provider_registry: Arc::new(macaca_context::ContextProviderRegistry::new()),
            context_engine_registry: Arc::clone(&context_engine_registry),
            external_adapter_runtime_registry: Arc::clone(&external_adapter_runtime_registry),
            provider_health_ledger: Arc::new(macaca_context::ProviderHealthLedger::new()),
        }
    });

    // 10a. Set the executor registry reference for the delegate tool
    {
        let mut guard = executor_registry_ref.write().await;
        *guard = Some(Arc::clone(&state.executor_registry));
    }
    {
        let mut guard = application_orchestration_registry_ref.write().await;
        *guard = Some(Arc::clone(&state.executor_registry));
    }

    // Register the unified Agent Context and Agent Execution services after
    // `AppState` exists because Web is the composition root for personas,
    // skills, tools, workspaces, and executor event broadcasts.  Application
    // runtimes, including WASM guests, call these services instead of reaching
    // into Web executor internals directly.
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_runtime_host::agent_context_service_descriptor(),
                    Arc::new(macaca_runtime_host::AgentContextSystemServiceProvider::new(
                        Arc::new(WebAgentContextBackend::new(Arc::clone(&state))),
                    )),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_proto::AGENT_CONTEXT_SERVICE_ID),
            TraceContext::new("web-startup-agent-context-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_runtime_host::execution_control_service_descriptor(),
                    Arc::new(
                        macaca_runtime_host::ExecutionControlSystemServiceProvider::new(Arc::new(
                            macaca_runtime_host::ExecutionControlRuntimeCapability::new(),
                        )),
                    ),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_proto::EXECUTION_CONTROL_SERVICE_ID),
            TraceContext::new("web-startup-execution-control-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_runtime_host::StaticServiceProviderFactory::new(
                macaca_runtime_host::ServiceProviderInstance::new(
                    macaca_runtime_host::agent_execution_service_descriptor(),
                    Arc::new(
                        macaca_runtime_host::AgentExecutionSystemServiceProvider::new(Arc::new(
                            SkillSelfEvolutionObservedAgentExecutionBackend::new(
                                Arc::new(WebAgentExecutionBackend::new(
                                    Arc::clone(&state),
                                    Arc::clone(&service_runtime),
                                )),
                                Arc::clone(&state),
                            ),
                        )),
                    ),
                ),
            ),
            macaca_runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_proto::AGENT_EXECUTION_SERVICE_ID),
            TraceContext::new("web-startup-agent-execution-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;

    // 10b. Register all started apps to the executor registry and create workspaces
    {
        let kernel_ref = Arc::clone(&kernel);
        let registry_ref = state.executor_registry.clone();
        let apps_to_register = started_apps.clone();
        let todo_store_for_recovery = Arc::clone(&state.persist.todo_store);
        let state_ref = Arc::clone(&state);

        tokio::spawn(async move {
            // Get all agents from kernel, then register each executor with
            // only the agents declared by that application.
            let all_agents = kernel_ref.list_agents().await;
            let agents_by_name: HashMap<_, _> =
                all_agents.iter().map(|m| (m.name.clone(), m)).collect();

            // Register each app to executor registry
            for (app_id, app_name, app_agent_names) in apps_to_register {
                let mut app_agents: Vec<AgentInfo> = app_agent_names
                    .iter()
                    .filter_map(|name| agents_by_name.get(name.as_str()).copied())
                    .map(|m| AgentInfo {
                        id: m.id.0.to_string(),
                        name: m.name.clone(),
                        capabilities: m.capabilities.iter().map(|c| c.name.clone()).collect(),
                        current_load: 0,
                        max_load: 4,
                        available: true,
                    })
                    .collect();
                if app_agents.is_empty() {
                    // Security boundary enforcement:
                    // Never fall back to global agents when an application has no
                    // app-scoped runtime identity. Falling back would let one app
                    // execute through another app's agents/tools, which breaks
                    // isolation and violates the generic multi-tenant OS contract.
                    //
                    // We keep the app discoverable/listed at control-plane level,
                    // but skip executor registration so request handling cannot
                    // accidentally route into unrelated global workers.
                    tracing::warn!(
                        app_id = %app_id.0,
                        app_name = %app_name,
                        "No app-scoped agents resolved; skipping executor registration to preserve app isolation"
                    );
                    continue;
                }
                let workspace_agent_names: Vec<String> =
                    app_agents.iter().map(|agent| agent.name.clone()).collect();

                // Register this app to executor registry
                let _executor = registry_ref
                    .register_application(app_id, app_name, app_agents.clone())
                    .await;
                tracing::info!(app_id = %app_id.0, "App registered to executor");

                // Recover crashed tasks: rollback InProgress/Assigned → Pending
                todo_store_for_recovery.rollback_in_progress(&app_id).await;

                // Create workspace directories for this app
                let workspace =
                    crate::workspace::AppWorkspace::new(&config.workspace.root_dir, &app_id);
                match workspace.ensure_dirs(&workspace_agent_names) {
                    Ok(()) => {
                        tracing::info!(
                            app_id = %app_id.0,
                            workspace = %workspace.root.display(),
                            "Workspace directories created"
                        );
                    }
                    Err(e) => {
                        tracing::error!(app_id = %app_id.0, error = %e, "Failed to create workspace directories");
                    }
                }
                state_ref
                    .config
                    .app_workspaces
                    .write()
                    .await
                    .insert(app_id, workspace);

                // Auto-start PlanLoop and WorkerLoops for this app so pending
                // tasks (e.g., PendingReview from before restart) are processed.
                crate::loop_manager::ensure_plan_and_worker_loops(&state_ref, &app_id, None).await;
            }
        });
    }

    // 10c. Note: executor_registry is available in state for task delegation
    // The executor_registry allows agents to delegate tasks to other agents
    // using capability-based routing or direct agent targeting.
    // Route handlers can access it via state.executor_registry
    {
        let _registry = state.executor_registry.clone();
        info!("ApplicationExecutorRegistry initialized and apps registered");
    }

    // 10d. Start hook event consumer for coordinator auto-continue
    {
        let consumer_state = Arc::clone(&state);
        tokio::spawn(async move {
            hook_consumer::start_hook_event_consumer(consumer_state).await;
        });
        info!("Hook event consumer started");
    }

    // 11. Build axum router.
    let app = WebRuntimeFacade::new(state).router();

    // 12. Start server.
    let addr = format!("0.0.0.0:{port}");
    info!(addr = %addr, "Macaca OS API server starting");
    println!("\n  Macaca OS API server: http://localhost:{port}\n");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| macaca_proto::MacacaError::Io(e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| macaca_proto::MacacaError::Io(e))?;

    Ok(())
}

/// Build provider-neutral Skill package roots for governance restart recovery.
///
/// Web remains a composition root here: it contributes filesystem roots that it
/// already owns through configuration and application startup, but it does not
/// parse Skill metadata, inspect package bodies, or decide governance state.
/// The Skill service provider performs those semantic checks behind the service
/// boundary.
fn materialized_skill_recovery_roots(
    workspace_root: &str,
    application_skill_roots: &[PathBuf],
) -> Vec<PathBuf> {
    let mut roots = application_skill_roots.to_vec();
    let workspace_root = PathBuf::from(workspace_root);
    if let Ok(children) = std::fs::read_dir(&workspace_root) {
        for child in children.flatten() {
            let path = child.path();
            if !path.is_dir() {
                continue;
            }
            if path.file_name().is_some_and(|name| name == "apps") {
                continue;
            }
            roots.push(path.join("skills"));
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

/// Return the local Skill governance event journal path for this host.
///
/// Web is acting only as an approved composition root here: it chooses a
/// workspace-scoped service storage location and passes it to the Skill service
/// provider.  It does not parse the journal, interpret governance events, or
/// branch on application-specific semantics.
fn skill_governance_event_journal_path(workspace_root: &str) -> PathBuf {
    PathBuf::from(workspace_root).join("skill-governance-events.jsonl")
}

/// Resolve the durable Memory file-store base path for local web runs.
///
/// The Memory service can write to both file/session layers and vector
/// providers. Keeping this resolver in the Web composition root lets operators
/// use a relative `memory.file_store_path` in config while still getting a
/// deterministic path under the host data directory. No application id, agent
/// name, or business workflow is hardcoded here; scope-specific isolation stays
/// inside Memory Service commands and providers.
fn configured_memory_base_path(data_dir: &std::path::Path, configured: &str) -> PathBuf {
    let configured = configured.trim();
    if configured.is_empty() {
        return data_dir.join("workspace_memory");
    }
    let path = PathBuf::from(configured);
    if path.is_absolute() {
        path
    } else {
        data_dir.join(path)
    }
}

/// Translate provider-neutral web configuration into runtime-host activation.
///
/// This Adapter keeps `macaca-proto` free of runtime-host concrete enum types
/// while still making local web startup explicit and auditable. Unknown modes
/// deliberately fall back to unavailable instead of trying to infer a provider;
/// that fail-closed behavior is part of the serviceization constitution.
fn autonomy_runtime_config_from_web_config(
    config: &AutonomyConfig,
) -> macaca_runtime_host::AutonomyRuntimeConfig {
    let provider_mode = match config.provider_mode.trim().to_ascii_lowercase().as_str() {
        "local" => macaca_runtime_host::AutonomyProviderMode::Local,
        _ => macaca_runtime_host::AutonomyProviderMode::Unavailable,
    };
    macaca_runtime_host::AutonomyRuntimeConfig {
        provider_mode,
        supervisor_enabled: config.supervisor_enabled,
        scheduler_tick_interval_ms: config.scheduler_tick_interval_ms,
        heartbeat_tick_interval_ms: config.heartbeat_tick_interval_ms,
        max_leases_per_tick: config.max_leases_per_tick,
        dispatch_timeout_ms: config.dispatch_timeout_ms,
        shutdown_grace_ms: config.shutdown_grace_ms,
        recovery_wake_enabled: config.recovery_wake_enabled,
        safe_retention_limit: config.safe_retention_limit,
    }
}
