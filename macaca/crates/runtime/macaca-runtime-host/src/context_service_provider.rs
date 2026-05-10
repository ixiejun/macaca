//! Runtime-host adapter for the Route C Context Service.
//!
//! The provider wraps a context engine registry and service commands while the
//! domain crate continues to own composition and engine behavior.  This keeps
//! runtime-host focused on lifecycle, dispatch, trace logging, and sanitized
//! snapshots.

use std::collections::BTreeMap;

use async_trait::async_trait;
use macaca_context::{
    assemble_context_providers, ContextAssembleCommand, ContextAssembleServiceResult,
    ContextEngineRegistry, ContextFacade, ContextFacadeAssemblyPolicy,
    ContextServiceRuntimeCapabilities, ContextServiceSnapshot, ProviderAssemblyEnvironment,
    ProviderFactoryInput, CONTEXT_ASSEMBLE_COMMAND, CONTEXT_ENGINE_INVENTORY_COMMAND,
    CONTEXT_PROVIDER_INVENTORY_COMMAND, CONTEXT_SNAPSHOT_COMMAND,
};
use macaca_kernel::SystemService;
use macaca_proto::{
    CleanupPolicy, KernelServiceId, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceContext, TraceSchemaRef,
};

/// Host-owned Context service provider backed by an engine registry.
pub struct ContextSystemServiceProvider {
    descriptor: ServiceDescriptor,
    registry: ContextEngineRegistry,
    capabilities: ContextServiceRuntimeCapabilities,
}

impl ContextSystemServiceProvider {
    /// Create a context service adapter from a provider-neutral engine registry.
    pub fn new(registry: ContextEngineRegistry) -> Self {
        Self::with_capabilities(registry, ContextServiceRuntimeCapabilities::default())
    }

    /// Create a context service adapter with runtime-only Strategy handles.
    ///
    /// Capabilities are injected at host boot, not embedded in service commands.
    /// This keeps the Context Service replaceable while still allowing the
    /// default implementation to run vector recall and knowledge digest through
    /// whatever memory service the host selected.
    pub fn with_capabilities(
        registry: ContextEngineRegistry,
        capabilities: ContextServiceRuntimeCapabilities,
    ) -> Self {
        let descriptor = ServiceDescriptor::new(
            KernelServiceId::new(macaca_context::CONTEXT_SERVICE_ID),
            ServiceType::new("context"),
            TraceSchemaRef::new("trace.system_service.context.v1"),
        );
        Self {
            descriptor,
            registry,
            capabilities,
        }
    }

    fn result(output: serde_json::Value, trace: TraceContext) -> ServiceCallResult {
        ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        }
    }
}

#[async_trait]
impl SystemService for ContextSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "context service provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "context service command accepted"
        );

        match command.name.as_str() {
            CONTEXT_ASSEMBLE_COMMAND => {
                let typed: ContextAssembleCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                let selection = typed.selection.clone();
                let trace = typed.trace.clone();
                let facade = ContextFacade::builtins_with_engine_overlay(selection, &self.registry);
                let (providers, policy) = if let Some(snapshot) = typed.assembly.as_ref() {
                    let env = ProviderAssemblyEnvironment {
                        agent_profile_root: snapshot.agent_profile_root.clone(),
                        agent_profile: snapshot.context_config.agent_profile.clone(),
                        skill_capability_catalog: Some(std::sync::Arc::new(
                            snapshot.skill_capability_catalog.clone(),
                        )),
                        mcp_capability_catalog: Some(std::sync::Arc::new(
                            snapshot.mcp_capability_catalog.clone(),
                        )),
                        runtime_tool_capability_catalog: Some(std::sync::Arc::new(
                            snapshot.runtime_tool_capability_catalog.clone(),
                        )),
                        ready_mcp_server_ids: Some(std::sync::Arc::new(
                            snapshot.ready_mcp_server_ids.clone(),
                        )),
                        memory_recall_capability: self.capabilities.memory_recall.clone(),
                        active_vector_memory: snapshot.context_config.active_vector_memory.clone(),
                        knowledge_digest_capability: self.capabilities.knowledge_digest.clone(),
                        knowledge_digest: snapshot.context_config.knowledge_digest.clone(),
                    };
                    let factory_input = ProviderFactoryInput {
                        agent_name: typed.scope.agent_name.clone(),
                        session_id: Some(typed.scope.session_id.clone()),
                        params: snapshot.factory_params.clone(),
                    };
                    let (providers, notes) = assemble_context_providers(
                        &snapshot.context_config,
                        &env,
                        &factory_input,
                        None,
                    )
                    .await
                    .map_err(|err| ServiceError::AdapterFailure(err.to_string()))?;
                    let policy = ContextFacadeAssemblyPolicy::from_context_config_parts(
                        snapshot.context_config.governance.clone(),
                        snapshot.context_config.trust_governance.clone(),
                        snapshot.context_config.knowledge_digest.clone(),
                    );
                    tracing::info!(
                        trace_id = %trace.trace_id,
                        provider_count = providers.len(),
                        catalog_note_count = notes.len(),
                        "context service rebuilt provider chain from assembly snapshot"
                    );
                    (providers, policy)
                } else {
                    tracing::warn!(
                        trace_id = %trace.trace_id,
                        "context service assemble command has no assembly snapshot; using legacy empty provider chain"
                    );
                    (
                        Vec::new(),
                        macaca_context::ContextFacadeAssemblyPolicy::default(),
                    )
                };
                let assembled = facade
                    .assemble_model_context(typed.into_engine_input(), &providers, policy)
                    .await
                    .map_err(|err| ServiceError::AdapterFailure(err.to_string()))?;
                let result = ContextAssembleServiceResult::new(assembled);
                tracing::info!(trace_id = %trace.trace_id, "context service assemble completed");
                Ok(Self::result(
                    serde_json::to_value(result)
                        .map_err(|err| ServiceError::AdapterFailure(err.to_string()))?,
                    trace,
                ))
            }
            CONTEXT_ENGINE_INVENTORY_COMMAND | CONTEXT_PROVIDER_INVENTORY_COMMAND => {
                tracing::info!(trace_id = %trace.trace_id, "context service inventory emitted");
                Ok(Self::result(
                    serde_json::json!({
                        "engines": self.registry.list_engine_infos(),
                        "providers": []
                    }),
                    trace,
                ))
            }
            CONTEXT_SNAPSHOT_COMMAND => {
                let snapshot = ContextServiceSnapshot::healthy(self.registry.list_engine_infos());
                tracing::info!(trace_id = %trace.trace_id, "context service snapshot emitted");
                Ok(Self::result(
                    serde_json::to_value(snapshot)
                        .map_err(|err| ServiceError::AdapterFailure(err.to_string()))?,
                    trace,
                ))
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Context service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "context service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "context service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}
