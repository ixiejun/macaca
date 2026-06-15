//! Bootstrap phase 9a: service bus provider registration and SDK client facades.
//!
//! Registers LLM, driver, skill, MCP, memory, context, and workbench-family providers on the
//! shared `ServiceRuntime`, then materializes typed `System*Client` handles for `AppState`.

use std::sync::Arc;
use std::time::Duration;

use tracing::{info, warn};

use crate::external_context_adapter::install_external_adapters_from_config;
use macaca_host_composition::framework::runtime_context::{
    AgentSessionStore as FrameworkAgentSessionStore,
    InMemoryAgentSessionStore as FrameworkInMemoryAgentSessionStore,
};
use macaca_host_composition::mcp_runtime::McpRuntimeFacade;
use macaca_proto::{KernelServiceId, MacacaResult, TraceContext, LLM_SERVICE_ID};

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
    let domain_pack_provider_registrations =
        super::domain_pack_wiring::installed_domain_pack_provider_registrations(Arc::clone(&llm));

    // 9a. Initialize audit logger. Alert delivery is registered as a system
    // service below so presentation code never owns concrete transport logic.
    let kernel_persistence = Arc::new(macaca_host_composition::RedbKernelPersistenceAdapter::new(
        Arc::clone(&session_store_impl),
    ));
    let audit_logger = Arc::new(macaca_host_composition::kernel::AuditLogger::new(
        kernel_persistence,
    ));
    let session_store = session_store_shared;
    info!("AuditLogger initialized");

    let default_model = llm_router.default_model_reference();
    let framework_session_store: Arc<dyn FrameworkAgentSessionStore> =
        Arc::new(FrameworkInMemoryAgentSessionStore::new());
    let mcp_runtime = Arc::new(McpRuntimeFacade::load_default().await);

    let (memory_runtime, workspace_memory_tombstones) = if config.context.recall.expose_memory_tools
    {
        let mem_dir = configured_memory_base_path(&data_dir, &config.memory.file_store_path);
        std::fs::create_dir_all(&mem_dir).ok();
        let factory = macaca_host_composition::memory::MemoryBackendFactory::new(
            macaca_host_composition::memory::MemoryBackendConfig::new(mem_dir.clone())
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
        let tomb = Arc::new(macaca_host_composition::memory::SharedTombstoneRegistry::new());
        info!(
            service_id = "memory",
            memory_base_path = %mem_dir.display(),
            vector_backend = %profile.vector_backend,
            vector_collection_configured = profile.vector_collection.is_some(),
            embedding_configured = true,
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
            macaca_host_composition::memory::FabricMemoryRuntime::from_configured_memory(
                configured_manager,
                provider_profile,
            ),
        );
        (Some(runtime), Some(Arc::clone(&tomb)))
    } else {
        warn!("Workspace memory runtime disabled by context recall configuration");
        (None, None)
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
            &macaca_host_composition::service_runtime::StaticServiceProviderFactory::new(
                macaca_host_composition::service_runtime::ServiceProviderInstance::new(
                    macaca_host_composition::service_bootstrap::alert_service_descriptor(),
                    Arc::new(macaca_host_composition::service_bootstrap::AlertSystemServiceProvider::tracing()),
                ),
            ),
            macaca_host_composition::service_runtime::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_host_composition::service_runtime::StaticServiceProviderFactory::new(
                macaca_host_composition::service_runtime::ServiceProviderInstance::new(
                    macaca_host_composition::llm::llm_service_descriptor(),
                    Arc::new({
                        let default_reference = llm_router.default_model_reference();
                        let profile = if default_reference.trim().is_empty() {
                            macaca_host_composition::service_bootstrap::LlmProviderProfile::generic(llm.name())
                        } else {
                            macaca_host_composition::service_bootstrap::LlmProviderProfile::generic(llm.name())
                                .with_default_model(default_reference)
                        };
                        macaca_host_composition::service_bootstrap::LlmSystemServiceProvider::with_catalog(
                            Arc::clone(&llm),
                            profile,
                            macaca_host_composition::service_bootstrap::LlmProviderCatalogProfile::from_config(
                                &config.llm,
                            ),
                        )
                    }),
                ),
            ),
            macaca_host_composition::service_runtime::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    let materialized_skill_roots =
        materialized_skill_recovery_roots(&config.workspace.root_dir, &skills_dirs);
    let governance_event_journal_path =
        skill_governance_event_journal_path(&config.workspace.root_dir);
    let skill_memory_runtime = memory_runtime.as_ref().map(|runtime| {
        Arc::clone(runtime) as Arc<dyn macaca_host_composition::memory::MemoryRuntimeFacade>
    });
    macaca_host_composition::service_bootstrap::bootstrap_local_skill_service_provider(
        &service_runtime,
        materialized_skill_roots,
        governance_event_journal_path,
        skill_memory_runtime,
    )
    .await
    .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .register_provider(
            &macaca_host_composition::service_runtime::StaticServiceProviderFactory::new(
                macaca_host_composition::service_runtime::ServiceProviderInstance::new(
                    macaca_host_composition::service_bootstrap::mcp_service_descriptor(),
                    Arc::new(
                        macaca_host_composition::service_bootstrap::McpSystemServiceProvider::new(
                            Arc::clone(&mcp_runtime),
                        ),
                    ),
                ),
            ),
            macaca_host_composition::service_runtime::ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    if let Some(runtime) = memory_runtime.as_ref() {
        let memory_service: Arc<dyn macaca_host_composition::kernel::SystemService> = Arc::new(
            macaca_host_composition::service_bootstrap::MemorySystemServiceProvider::new(
                Arc::clone(runtime) as Arc<dyn macaca_host_composition::memory::MemoryFacade>,
            ),
        );
        service_runtime
            .register_provider(
                &macaca_host_composition::service_runtime::StaticServiceProviderFactory::new(
                    macaca_host_composition::service_runtime::ServiceProviderInstance::new(
                        macaca_host_composition::memory::memory_service_descriptor(),
                        memory_service,
                    ),
                ),
                macaca_host_composition::service_runtime::ServiceProviderFactoryContext::new(),
            )
            .await
            .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
        service_runtime
            .start(
                &KernelServiceId::new(macaca_host_composition::memory::MEMORY_SERVICE_ID),
                TraceContext::new("web-startup-memory-service"),
            )
            .await
            .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    }
    let configured_external_adapters = install_external_adapters_from_config(&config.context)?;
    let external_adapter_installations = configured_external_adapters.installations.clone();
    let external_adapter_runtime_registry = Arc::new(
        crate::context_runtime_facade::ExternalAdapterRuntimeRegistry::new()
            .with_installations(external_adapter_installations),
    );
    let context_engine_registry = Arc::new(configured_external_adapters.registry);
    let context_service_capabilities =
        macaca_host_composition::context::ContextServiceRuntimeCapabilities {
            memory_recall: if memory_runtime.is_some() {
                crate::context_reporting_memory::build_context_service_recall_capability(
                    &config.context,
                    Arc::clone(&memory_client),
                    workspace_memory_tombstones.as_ref(),
                )
            } else {
                None
            },
            knowledge_digest: None,
        };
    service_runtime
        .register_provider(
            &macaca_host_composition::service_runtime::StaticServiceProviderFactory::new(
                macaca_host_composition::service_runtime::ServiceProviderInstance::new(
                    macaca_proto::ServiceDescriptor::new(
                        KernelServiceId::new(macaca_host_composition::context::CONTEXT_SERVICE_ID),
                        macaca_proto::ServiceType::new("context"),
                        macaca_proto::TraceSchemaRef::new("trace.system_service.context.v1"),
                    ),
                    Arc::new(
                        macaca_host_composition::service_bootstrap::ContextSystemServiceProvider::with_capabilities(
                            (*context_engine_registry).clone(),
                            context_service_capabilities,
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
            &KernelServiceId::new(macaca_host_composition::context::CONTEXT_SERVICE_ID),
            TraceContext::new("web-startup-context-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_sdk::ALERT_SERVICE_ID),
            TraceContext::new("web-startup-alert-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(LLM_SERVICE_ID),
            TraceContext::new("web-startup-llm-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_sdk::DRIVER_SERVICE_ID),
            TraceContext::new("web-startup-driver-service"),
        )
        .await
        .map_err(|err| macaca_proto::MacacaError::Config(err.to_string()))?;
    service_runtime
        .start(
            &KernelServiceId::new(macaca_host_composition::runtime_host::SKILL_SERVICE_ID),
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
    let workbench_services = macaca_host_composition::bootstrap_workbench_family_services(
        Arc::clone(&service_runtime),
        "web-startup",
    )
    .await?;
    info!(
        services = workbench_services.requested_services,
        "Workbench-family services bootstrapped through host composition"
    );
    // Domain-pack providers are optional extensions registered by composition
    // roots through package crates (for example `macaca-domain-pack-finance`).
    // The base web shell registers none by default so absent packs surface
    // structured unavailable results instead of OS-owned business logic.
    let domain_pack_services =
        macaca_host_composition::application_bootstrap::bootstrap_domain_pack_services(
            Arc::clone(&service_runtime),
            domain_pack_provider_registrations,
            "web-startup-domain-pack",
        )
        .await?;
    info!(
        services = domain_pack_services.started_services.len(),
        "Domain-pack bootstrap completed through generic runtime-host boundary"
    );
    // S12 thin-shell completion moves optional service lifecycle ownership into
    // `macaca-runtime-host`. Web provides repositories and the event log only;
    // provider registration/start semantics now cross a typed host bootstrap
    // boundary instead of living in presentation startup.
    let optional_services = macaca_host_composition::bootstrap_host_optional_services(
        Arc::clone(&service_runtime),
        Arc::clone(&entitlement_store),
        Some(Arc::clone(&event_log)),
        Arc::clone(&payment_store),
        macaca_proto::local_simulated_terms("1", "UNIT"),
        "web-startup",
    )
    .await?;
    info!(
        services = optional_services.started_services,
        "Optional services bootstrapped through host composition"
    );
    ctx.audit_logger = Some(audit_logger);
    ctx.session_store = Some(session_store);
    ctx.default_model = Some(default_model);
    ctx.framework_session_store = Some(framework_session_store);
    ctx.mcp_runtime = Some(mcp_runtime);
    ctx.memory_runtime = memory_runtime;
    ctx.workspace_memory_tombstones = workspace_memory_tombstones;
    ctx.memory_client = Some(memory_client);
    ctx.external_adapter_runtime_registry = Some(external_adapter_runtime_registry);
    ctx.context_engine_registry = Some(context_engine_registry);
    super::service_client_facades::materialize_service_clients(ctx)?;
    Ok(())
}
