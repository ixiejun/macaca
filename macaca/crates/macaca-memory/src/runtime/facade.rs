use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryEntry, MemoryId};

use crate::core::active_recall::{ActiveRecallCapability, ActiveRecallRequest, ActiveRecallResult};
use crate::core::facade::{
    MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemorySearchRequest, MemoryWriteRequest,
};
use crate::governance::{
    KnowledgeCompileCapability, KnowledgeCompileRequest, KnowledgeCompileResult,
};

use super::status::MemoryRuntimeStatus;

/// Canonical runtime boundary for upper crates.
#[async_trait]
pub trait MemoryRuntimeFacade: Send + Sync {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId>;
    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>>;
    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>>;
    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()>;
    async fn active_recall(&self, request: ActiveRecallRequest)
        -> MacacaResult<ActiveRecallResult>;
    async fn compile_knowledge(
        &self,
        request: KnowledgeCompileRequest,
    ) -> MacacaResult<KnowledgeCompileResult>;
    async fn status(&self) -> MemoryRuntimeStatus;
}

/// Composed runtime that keeps the memory fabric behind one facade.
pub struct ComposedMemoryRuntime<F, A, K> {
    facade: Arc<F>,
    active_recall: Arc<A>,
    knowledge: Arc<K>,
    runtime_id: String,
    provider_profile: Option<String>,
}

impl<F, A, K> ComposedMemoryRuntime<F, A, K> {
    pub fn new(facade: Arc<F>, active_recall: Arc<A>, knowledge: Arc<K>) -> Self {
        Self {
            facade,
            active_recall,
            knowledge,
            runtime_id: "memory-runtime".into(),
            provider_profile: None,
        }
    }

    pub fn with_runtime_id(mut self, runtime_id: impl Into<String>) -> Self {
        self.runtime_id = runtime_id.into();
        self
    }

    pub fn with_provider_profile(mut self, provider_profile: Option<String>) -> Self {
        self.provider_profile = provider_profile;
        self
    }
}

#[async_trait]
impl<F, A, K> MemoryRuntimeFacade for ComposedMemoryRuntime<F, A, K>
where
    F: MemoryFacade + Send + Sync,
    A: ActiveRecallCapability + Send + Sync,
    K: KnowledgeCompileCapability + Send + Sync,
{
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        self.facade.remember(request).await
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        self.facade.search(request).await
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.facade.get(request).await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        self.facade.delete(request).await
    }

    async fn active_recall(
        &self,
        request: ActiveRecallRequest,
    ) -> MacacaResult<ActiveRecallResult> {
        self.active_recall.prefetch(request).await
    }

    async fn compile_knowledge(
        &self,
        request: KnowledgeCompileRequest,
    ) -> MacacaResult<KnowledgeCompileResult> {
        Ok(self.knowledge.compile(request))
    }

    async fn status(&self) -> MemoryRuntimeStatus {
        let status = self.facade.status();
        MemoryRuntimeStatus {
            runtime_id: self.runtime_id.clone(),
            provider_profile: self.provider_profile.clone().or(Some(status.provider_id)),
            store_available: status.capabilities.store,
            search_available: status.capabilities.search,
            active_recall_available: true,
            knowledge_available: true,
            diagnostics: status.message.into_iter().collect(),
        }
    }
}
