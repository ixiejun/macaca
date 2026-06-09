//! Bootstrap phase 9a: service bus provider registration and SDK client facades.
//!
//! Registers LLM, driver, skill, MCP, memory, context, and workbench-family providers on the
//! shared `ServiceRuntime`, then materializes typed `System*Client` handles for `AppState`.

use std::sync::Arc;
use std::time::Duration;

use tracing::{info, warn};

use crate::external_context_adapter::install_external_adapters_from_config;
use crate::persistence_adapter::RedbKernelPersistenceAdapter;
use macaca_proto::{KernelServiceId, MacacaResult, TraceContext};
use macaca_sdk::framework::runtime_context::{
    AgentSessionStore as FrameworkAgentSessionStore,
    InMemoryAgentSessionStore as FrameworkInMemoryAgentSessionStore,
};

use super::bootstrap_path_helpers::{
    configured_memory_base_path, materialized_skill_recovery_roots,
    skill_governance_event_journal_path,
};

use super::bootstrap_ctx::BootstrapCtx;

/// Run the `service-runtime-wiring` bootstrap slice.
pub(crate) async fn run(ctx: &mut BootstrapCtx) -> MacacaResult<()> {
    let config = ctx.config.clone().expect("bootstrap: config");
    let llm = Arc::clone(ctx.llm.as_ref().expect("bootstrap: llm"));
    let llm_router = Arc::clone(ctx.llm_router.as_ref().expect("bootstrap: llm_router"));
    let service_runtime = Arc::clone(
        ctx.service_runtime
            .as_ref()
            .expect("bootstrap: service_runtime"),
    );
    let skills_dirs = ctx.skills_dirs.clone().expect("bootstrap: skills_dirs");
    let data_dir = ctx.data_dir.clone().expect("bootstrap: data_dir");
    let session_store_impl = Arc::clone(
        ctx.session_store_impl
            .as_ref()
            .expect("bootstrap: session_store_impl"),
    );
    let session_store_shared = Arc::clone(
        ctx.session_store_shared
            .as_ref()
            .expect("bootstrap: session_store_shared"),
    );
    let generic_service_client = Arc::clone(
        ctx.generic_service_client
            .as_ref()
            .expect("bootstrap: generic_service_client"),
    );
    let event_log = Arc::clone(ctx.event_log.as_ref().expect("bootstrap: event_log"));
    let entitlement_store = Arc::clone(
        ctx.entitlement_store
            .as_ref()
            .expect("bootstrap: entitlement_store"),
    );
    let payment_store = Arc::clone(
        ctx.payment_store
            .as_ref()
            .expect("bootstrap: payment_store"),
    );
    let entitlement_facade = Arc::clone(
        ctx.entitlement_facade
            .as_ref()
            .expect("bootstrap: entitlement_facade"),
    );
    let driver_runtime = Arc::clone(
        ctx.driver_runtime
            .as_ref()
            .expect("bootstrap: driver_runtime"),
    );
    let domain_pack_provider_registrations =
        super::domain_pack_wiring::installed_domain_pack_provider_registrations(Arc::clone(&llm));

    // 9a. Initialize audit logger and alert manager.
    let kernel_persistence = Arc::new(RedbKernelPersistenceAdapter::new(Arc::clone(
        &session_store_impl,
    )));
    let audit_logger = Arc::new(macaca_sdk::kernel::audit::AuditLogger::new(
        kernel_persistence,
    ));
    let session_store = session_store_shared;
    let alert_config = macaca_sdk::kernel::alert::AlertConfig::default();
    let alert_manager = Arc::new(macaca_sdk::kernel::alert::AlertManager::new(alert_config));
    info!("AuditLogger and AlertManager initialized");

    let default_model = llm_router.default_model_reference();
    let framework_session_store: Arc<dyn FrameworkAgentSessionStore> =
        Arc::new(FrameworkInMemoryAgentSessionStore::new());
    let mcp_runtime = Arc::new(macaca_sdk::runtime_host::McpRuntimeFacade::load_default().await);

    let (memory_runtime, workspace_memory, workspace_memory_tombstones) =
        if config.context.recall.expose_memory_tools {
            let mem_dir = configured_memory_base_path(&data_dir, &config.memory.file_store_path);
            std::fs::create_dir_all(&mem_dir).ok();
            let factory = macaca_sdk::memory::MemoryBackendFactory::new(
                macaca_sdk::memory::MemoryBackendConfig::new(mem_dir.clone())
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
            let tomb = Arc::new(macaca_sdk::memory::SharedTombstoneRegistry::new());
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
                None::<Arc<macaca_sdk::memory::TestMemoryManager>>,
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
            &macaca_sdk::runtime_host::StaticServiceProviderFactory::new(
                macaca_sdk::runtime_host::ServiceProviderInstance::new(
                    macaca_sdk::llm::llm_service_descriptor(),
                    Arc::new({
                        let default_reference = llm_router.default_model_reference();
                        let profile = if default_reference.trim().is_empty() {
                            macaca_sdk::runtime_host::LlmProviderProfile::generic(llm.name())
                        } else {
                            macaca_sdk::runtime_host::LlmProviderProfile::generic(llm.name())
                                .with_default_model(default_reference)
                        };
                        macaca_sdk::runtime_host::LlmSystemServiceProvider::with_catalog(
                            Arc::clone(&llm),
                            profile,
                            macaca_sdk::runtime_host::LlmProviderCatalogProfile::from_config(
                                &config.llm,
                            ),
                        )
                    }),
                ),
            ),
            macaca_sdk::runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_sdk::runtime_host::StaticServiceProviderFactory::new(
                macaca_sdk::runtime_host::ServiceProviderInstance::new(
                    macaca_sdk::driver::driver_service_descriptor(),
                    Arc::new(macaca_sdk::runtime_host::DriverSystemServiceProvider::new(
                        Arc::clone(&driver_runtime),
                    )),
                ),
            ),
            macaca_sdk::runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    let materialized_skill_roots =
        materialized_skill_recovery_roots(&config.workspace.root_dir, &skills_dirs);
    let governance_event_journal_path =
        skill_governance_event_journal_path(&config.workspace.root_dir);
    let skill_service_provider = if let Some(runtime) = memory_runtime.as_ref() {
        macaca_sdk::runtime_host::SkillSystemServiceProvider::new()
            .with_materialized_skill_roots(materialized_skill_roots.clone())
            .with_governance_event_journal_path(governance_event_journal_path.clone())
            .with_memory_runtime(
                Arc::clone(runtime) as Arc<dyn macaca_sdk::memory::MemoryRuntimeFacade>
            )
    } else {
        macaca_sdk::runtime_host::SkillSystemServiceProvider::new()
            .with_materialized_skill_roots(materialized_skill_roots.clone())
            .with_governance_event_journal_path(governance_event_journal_path.clone())
    };
    service_runtime
        .register_provider(
            &macaca_sdk::runtime_host::StaticServiceProviderFactory::new(
                macaca_sdk::runtime_host::ServiceProviderInstance::new(
                    macaca_sdk::skill::skill_service_descriptor(),
                    Arc::new(skill_service_provider),
                ),
            ),
            macaca_sdk::runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_sdk::runtime_host::StaticServiceProviderFactory::new(
                macaca_sdk::runtime_host::ServiceProviderInstance::new(
                    macaca_sdk::runtime_host::mcp_service_descriptor(),
                    Arc::new(macaca_sdk::runtime_host::McpSystemServiceProvider::new(
                        Arc::clone(&mcp_runtime),
                    )),
                ),
            ),
            macaca_sdk::runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    if let Some(runtime) = memory_runtime.as_ref() {
        let memory_service: Arc<dyn macaca_sdk::kernel::SystemService> =
            Arc::new(macaca_sdk::runtime_host::MemorySystemServiceProvider::new(
                Arc::clone(runtime) as Arc<dyn macaca_sdk::memory::MemoryFacade>,
            ));
        service_runtime
            .register_provider(
                &macaca_sdk::runtime_host::StaticServiceProviderFactory::new(
                    macaca_sdk::runtime_host::ServiceProviderInstance::new(
                        macaca_sdk::memory::memory_service_descriptor(),
                        memory_service,
                    ),
                ),
                macaca_sdk::runtime_host::ServiceProviderFactoryContext::new(),
            )
            .await
            .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
        service_runtime
            .start(
                &KernelServiceId::new(macaca_sdk::memory::MEMORY_SERVICE_ID),
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
    let context_service_capabilities = macaca_sdk::context::ContextServiceRuntimeCapabilities {
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
            &macaca_sdk::runtime_host::StaticServiceProviderFactory::new(
                macaca_sdk::runtime_host::ServiceProviderInstance::new(
                    macaca_proto::ServiceDescriptor::new(
                        KernelServiceId::new(macaca_sdk::context::CONTEXT_SERVICE_ID),
                        macaca_proto::ServiceType::new("context"),
                        macaca_proto::TraceSchemaRef::new("trace.system_service.context.v1"),
                    ),
                    Arc::new(
                        macaca_sdk::runtime_host::ContextSystemServiceProvider::with_capabilities(
                            (*context_engine_registry).clone(),
                            context_service_capabilities,
                        ),
                    ),
                ),
            ),
            macaca_sdk::runtime_host::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_sdk::context::CONTEXT_SERVICE_ID),
            TraceContext::new("web-startup-context-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_sdk::llm::LLM_SERVICE_ID),
            TraceContext::new("web-startup-llm-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_sdk::driver::DRIVER_SERVICE_ID),
            TraceContext::new("web-startup-driver-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_sdk::skill::SKILL_SERVICE_ID),
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
    // Web is the host composition root, but runtime-host remains the owning
    // factory for concrete providers.  Register all workbench-family services
    // here so application WASM host imports and shell diagnostics route through
    // `ServiceRuntime` instead of encountering unknown service ids.
    macaca_sdk::runtime_host::bootstrap_local_app_protocol_service(
        Arc::clone(&service_runtime),
        "web-startup-app-protocol-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_process_service(
        Arc::clone(&service_runtime),
        "web-startup-process-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_sandbox_service(
        Arc::clone(&service_runtime),
        "web-startup-sandbox-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_diagnostics_service(
        Arc::clone(&service_runtime),
        "web-startup-diagnostics-service",
    )
    .await?;
    // Realtime is optional.  Registering an unavailable Null Object provider
    // preserves traceable, auditable behavior without pretending a transport is
    // configured or leaking optional-module absence as an unknown service.
    macaca_sdk::runtime_host::bootstrap_unavailable_realtime_service(
        Arc::clone(&service_runtime),
        "web-startup-realtime-service",
        "realtime provider is not configured",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_remote_environment_service(
        Arc::clone(&service_runtime),
        "web-startup-remote-environment-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_file_service(
        Arc::clone(&service_runtime),
        "web-startup-file-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_approval_service(
        Arc::clone(&service_runtime),
        "web-startup-approval-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_hook_service(
        Arc::clone(&service_runtime),
        "web-startup-hook-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_config_service(
        Arc::clone(&service_runtime),
        "web-startup-config-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_plugin_marketplace_service(
        Arc::clone(&service_runtime),
        "web-startup-plugin-marketplace-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_code_intelligence_service(
        Arc::clone(&service_runtime),
        "web-startup-code-intelligence-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_git_service(
        Arc::clone(&service_runtime),
        "web-startup-git-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_local_review_service(
        Arc::clone(&service_runtime),
        "web-startup-review-service",
    )
    .await?;
    macaca_sdk::runtime_host::bootstrap_tool_planning_service(
        Arc::clone(&service_runtime),
        Arc::new(macaca_sdk::runtime_host::industrial_tool_planning_service()?),
        "web-startup-tool-service",
    )
    .await?;
    // Domain-pack providers are optional extensions registered by composition
    // roots through package crates (for example `macaca-domain-pack-finance`).
    // The base web shell registers none by default so absent packs surface
    // structured unavailable results instead of OS-owned business logic.
    let domain_pack_services = macaca_sdk::runtime_host::bootstrap_domain_pack_services(
        Arc::clone(&service_runtime),
        domain_pack_provider_registrations,
        "web-startup-domain-pack",
    )
    .await?;
    info!(
        services = domain_pack_services.started_services.len(),
        "Domain-pack bootstrap completed through generic runtime-host boundary"
    );
    // S12 thin-shell completion moves S9-S11 service lifecycle ownership into
    // `macaca-runtime-host`.  Web still provides the existing local stores and
    // facade handles, but provider registration/start semantics now cross a
    // typed host bootstrap boundary instead of living in presentation startup.
    let route_c_optional_services = macaca_sdk::runtime_host::bootstrap_route_c_optional_services(
        Arc::clone(&service_runtime),
        macaca_sdk::runtime_host::RouteCOptionalServicesBootstrapInputs::new(
            Arc::clone(&entitlement_store),
            Arc::clone(&entitlement_facade),
            Arc::clone(&payment_store),
            macaca_proto::local_simulated_terms("1", "UNIT"),
        ),
        "web-startup",
    )
    .await?;
    info!(
        services = route_c_optional_services.started_services.len(),
        "Route C optional services bootstrapped through runtime-host"
    );
    ctx.audit_logger = Some(audit_logger);
    ctx.session_store = Some(session_store);
    ctx.alert_manager = Some(alert_manager);
    ctx.default_model = Some(default_model);
    ctx.framework_session_store = Some(framework_session_store);
    ctx.mcp_runtime = Some(mcp_runtime);
    ctx.memory_runtime = memory_runtime;
    ctx.workspace_memory = workspace_memory;
    ctx.workspace_memory_tombstones = workspace_memory_tombstones;
    ctx.memory_client = Some(memory_client);
    ctx.external_adapter_runtime_registry = Some(external_adapter_runtime_registry);
    ctx.context_engine_registry = Some(context_engine_registry);
    super::service_client_facades::materialize_service_clients(ctx)?;
    Ok(())
}
