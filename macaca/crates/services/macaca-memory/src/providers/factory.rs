use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use std::collections::HashMap;

use crate::core::provider::{MemoryProvider, MemoryProviderDescriptor};
use crate::core::status::MemoryCapabilitySet;
use crate::core::{
    BuiltinAgentPrivateMemory, BuiltinSessionSharedMemory, MemoryFacade, MemoryStatusReport,
};
use crate::store::{EmbeddingProvider, VectorStore};

use super::config::{MemoryProviderConfig, MemoryProviderTransportKind};
use super::mcp::McpMemoryProvider;
use super::remote::RemoteMemoryProvider;
use super::slot::{MemoryProviderSlotBinding, MemoryProviderSlotKind};

/// Result of building one provider entry.
pub enum MemoryProviderFactoryResult<'a> {
    Builtin(BuiltinMemoryProviderAdapter<'a>),
    Remote(RemoteMemoryProvider),
    Mcp(McpMemoryProvider),
    Unconfigured,
}

/// Factory that materializes provider adapters from configuration.
pub trait MemoryProviderFactory {
    fn build(
        &self,
        provider_id: &str,
        binding: MemoryProviderSlotBinding,
    ) -> MacacaResult<MemoryProviderFactoryResult<'_>>;
}

/// Builtin provider factory used for the default runtime.
pub struct BuiltinMemoryProviderFactory<'a, V: VectorStore, E: EmbeddingProvider> {
    private_memory: &'a BuiltinAgentPrivateMemory<'a, V, E>,
    shared_memory: &'a BuiltinSessionSharedMemory<'a, V, E>,
    providers: HashMap<String, MemoryProviderConfig>,
}

impl<'a, V: VectorStore, E: EmbeddingProvider> BuiltinMemoryProviderFactory<'a, V, E> {
    pub fn new(
        private_memory: &'a BuiltinAgentPrivateMemory<'a, V, E>,
        shared_memory: &'a BuiltinSessionSharedMemory<'a, V, E>,
        providers: HashMap<String, MemoryProviderConfig>,
    ) -> Self {
        Self {
            private_memory,
            shared_memory,
            providers,
        }
    }
}

impl<'a, V: VectorStore, E: EmbeddingProvider> MemoryProviderFactory
    for BuiltinMemoryProviderFactory<'a, V, E>
{
    fn build(
        &self,
        provider_id: &str,
        binding: MemoryProviderSlotBinding,
    ) -> MacacaResult<MemoryProviderFactoryResult<'_>> {
        if provider_id == "builtin" {
            return self.build_builtin(binding);
        }
        let Some(config) = self.providers.get(provider_id) else {
            return Err(MacacaError::Memory(format!(
                "unknown memory provider id: {provider_id}"
            )));
        };
        match config.transport {
            MemoryProviderTransportKind::Builtin => self.build_builtin(binding),
            MemoryProviderTransportKind::Remote => {
                RemoteMemoryProvider::new(config).map(MemoryProviderFactoryResult::Remote)
            }
            MemoryProviderTransportKind::Mcp => self.build_mcp(config),
        }
    }
}

impl<'a, V: VectorStore, E: EmbeddingProvider> BuiltinMemoryProviderFactory<'a, V, E> {
    fn build_builtin(
        &self,
        binding: MemoryProviderSlotBinding,
    ) -> MacacaResult<MemoryProviderFactoryResult<'_>> {
        match binding.slot {
            MemoryProviderSlotKind::AgentPrivate => Ok(MemoryProviderFactoryResult::Builtin(
                BuiltinMemoryProviderAdapter::new(
                    "builtin".to_string(),
                    binding,
                    self.private_memory,
                ),
            )),
            MemoryProviderSlotKind::SessionShared => Ok(MemoryProviderFactoryResult::Builtin(
                BuiltinMemoryProviderAdapter::new(
                    "builtin".to_string(),
                    binding,
                    self.shared_memory,
                ),
            )),
            _ => Ok(MemoryProviderFactoryResult::Unconfigured),
        }
    }

    fn build_mcp(
        &self,
        config: &MemoryProviderConfig,
    ) -> MacacaResult<MemoryProviderFactoryResult<'_>> {
        let server = config.endpoint.as_ref().ok_or_else(|| {
            MacacaError::Memory(format!("mcp provider {} is missing endpoint", config.id))
        })?;
        let mcp_server = super::config::MemoryProviderMcpServerConfig {
            command: server.url.clone(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            timeout_ms: server.timeout_ms,
            trust_external: false,
        };
        Ok(MemoryProviderFactoryResult::Mcp(McpMemoryProvider::new(
            config, mcp_server,
        )))
    }
}

/// Lightweight adapter that exposes a builtin facade as a provider instance.
pub struct BuiltinMemoryProviderAdapter<'a> {
    provider_id: String,
    binding: MemoryProviderSlotBinding,
    facade: &'a dyn MemoryFacade,
}

impl<'a> BuiltinMemoryProviderAdapter<'a> {
    pub fn new(
        provider_id: String,
        binding: MemoryProviderSlotBinding,
        facade: &'a dyn MemoryFacade,
    ) -> Self {
        Self {
            provider_id,
            binding,
            facade,
        }
    }
}

impl<'a> MemoryProvider for BuiltinMemoryProviderAdapter<'a> {
    fn descriptor(&self) -> MemoryProviderDescriptor {
        let display_name = match self.binding.slot {
            MemoryProviderSlotKind::AgentPrivate => "builtin-agent-private",
            MemoryProviderSlotKind::SessionShared => "builtin-session-shared",
            MemoryProviderSlotKind::Embedding => "builtin-embedding",
            MemoryProviderSlotKind::VectorBackend => "builtin-vector-backend",
            MemoryProviderSlotKind::ActiveRecall => "builtin-active-recall",
            MemoryProviderSlotKind::KnowledgeCompiler => "builtin-knowledge-compiler",
        };
        MemoryProviderDescriptor::new(
            self.provider_id.clone(),
            display_name,
            MemoryCapabilitySet::basic_store_search(),
        )
    }
}

#[async_trait]
impl<'a> MemoryFacade for BuiltinMemoryProviderAdapter<'a> {
    async fn remember(
        &self,
        request: crate::core::facade::MemoryWriteRequest,
    ) -> MacacaResult<macaca_proto::MemoryId> {
        self.facade.remember(request).await
    }

    async fn search(
        &self,
        request: crate::core::facade::MemorySearchRequest,
    ) -> MacacaResult<Vec<macaca_proto::MemoryEntry>> {
        self.facade.search(request).await
    }

    async fn get(
        &self,
        request: crate::core::facade::MemoryGetRequest,
    ) -> MacacaResult<Option<macaca_proto::MemoryEntry>> {
        self.facade.get(request).await
    }

    async fn delete(&self, request: crate::core::facade::MemoryDeleteRequest) -> MacacaResult<()> {
        self.facade.delete(request).await
    }

    fn status(&self) -> MemoryStatusReport {
        self.facade.status()
    }
}
