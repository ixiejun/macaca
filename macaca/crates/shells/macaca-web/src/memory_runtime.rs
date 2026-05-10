use std::sync::Arc;

use async_trait::async_trait;
use macaca_memory::{
    ActiveRecallCapability, ActiveRecallRequest, ActiveRecallResult, DefaultActiveRecallStrategy,
    KnowledgeCompileCapability, KnowledgeCompileRequest, KnowledgeCompileResult, KnowledgeCompiler,
    MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemoryRuntimeFacade, MemoryRuntimeStatus,
    MemorySearchRequest, MemoryWriteRequest, RecallQuery, TestMemoryManager,
};
use macaca_proto::{MacacaResult, MemoryEntry, MemoryId};

/// Web-facing adapter over the canonical memory runtime facade.
#[derive(Clone)]
pub struct WebMemoryRuntime {
    inner: Arc<dyn MemoryRuntimeFacade>,
}

impl WebMemoryRuntime {
    pub fn new(inner: Arc<dyn MemoryRuntimeFacade>) -> Self {
        Self { inner }
    }

    pub fn from_workspace_memory(memory: Arc<TestMemoryManager>) -> Self {
        Self::new(Arc::new(LegacyWorkspaceMemoryRuntime::new(memory)))
    }

    pub async fn remember_text(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        self.inner.remember(request).await
    }

    pub async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        self.inner.search(request).await
    }

    pub async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.inner.get(request).await
    }

    pub async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        self.inner.delete(request).await
    }

    pub async fn active_recall(
        &self,
        request: ActiveRecallRequest,
    ) -> MacacaResult<ActiveRecallResult> {
        self.inner.active_recall(request).await
    }

    pub async fn compile_knowledge(
        &self,
        request: KnowledgeCompileRequest,
    ) -> MacacaResult<KnowledgeCompileResult> {
        self.inner.compile_knowledge(request).await
    }

    pub async fn status(&self) -> MemoryRuntimeStatus {
        self.inner.status().await
    }
}

/// Runtime facade over the legacy workspace `TestMemoryManager`.
struct LegacyWorkspaceMemoryRuntime {
    memory: Arc<TestMemoryManager>,
    knowledge: KnowledgeCompiler,
}

impl LegacyWorkspaceMemoryRuntime {
    fn new(memory: Arc<TestMemoryManager>) -> Self {
        Self {
            memory,
            knowledge: KnowledgeCompiler,
        }
    }
}

#[async_trait]
impl MemoryRuntimeFacade for LegacyWorkspaceMemoryRuntime {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        let mut input = macaca_memory::RememberText::new(request.content)
            .layer(request.layer)
            .metadata(request.metadata);
        if let Some(agent_id) = request.scope.agent_id_value() {
            input = input.agent_id(agent_id);
        }
        self.memory.remember_text(input).await
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        Ok(self
            .memory
            .recall(RecallQuery::new(request.query, request.limit))
            .await?
            .entries)
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.memory.get_entry(&request.id).await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        self.memory
            .forget(macaca_memory::ForgetMemory { id: request.id })
            .await
    }

    async fn active_recall(
        &self,
        request: ActiveRecallRequest,
    ) -> MacacaResult<ActiveRecallResult> {
        let strategy = DefaultActiveRecallStrategy::new("web-memory-runtime", self);
        strategy.prefetch(request).await
    }

    async fn compile_knowledge(
        &self,
        request: KnowledgeCompileRequest,
    ) -> MacacaResult<KnowledgeCompileResult> {
        Ok(self.knowledge.compile(request))
    }

    async fn status(&self) -> MemoryRuntimeStatus {
        MemoryRuntimeStatus {
            runtime_id: "web-memory-runtime".into(),
            provider_profile: Some("legacy-workspace-builtin".into()),
            store_available: true,
            search_available: true,
            active_recall_available: true,
            knowledge_available: true,
            diagnostics: Vec::new(),
        }
    }
}

#[async_trait]
impl MemoryFacade for LegacyWorkspaceMemoryRuntime {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        MemoryRuntimeFacade::remember(self, request).await
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        MemoryRuntimeFacade::search(self, request).await
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.memory.get_entry(&request.id).await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        self.memory
            .forget(macaca_memory::ForgetMemory { id: request.id })
            .await
    }

    fn status(&self) -> macaca_memory::MemoryStatusReport {
        macaca_memory::MemoryStatusReport::healthy(
            "web-memory-runtime",
            macaca_memory::MemoryCapabilitySet {
                store: true,
                search: true,
                prompt: true,
                lifecycle: true,
                flush: false,
                artifact: false,
                governance: false,
                knowledge: true,
            },
        )
    }
}

#[async_trait]
impl MemoryRuntimeFacade for WebMemoryRuntime {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        self.inner.remember(request).await
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        self.inner.search(request).await
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.inner.get(request).await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        self.inner.delete(request).await
    }

    async fn active_recall(
        &self,
        request: ActiveRecallRequest,
    ) -> MacacaResult<ActiveRecallResult> {
        self.inner.active_recall(request).await
    }

    async fn compile_knowledge(
        &self,
        request: KnowledgeCompileRequest,
    ) -> MacacaResult<KnowledgeCompileResult> {
        self.inner.compile_knowledge(request).await
    }

    async fn status(&self) -> MemoryRuntimeStatus {
        self.inner.status().await
    }
}

#[async_trait]
impl MemoryFacade for WebMemoryRuntime {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        MemoryRuntimeFacade::remember(self, request).await
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        MemoryRuntimeFacade::search(self, request).await
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        MemoryRuntimeFacade::get(self, request).await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        MemoryRuntimeFacade::delete(self, request).await
    }

    fn status(&self) -> macaca_memory::MemoryStatusReport {
        // `MemoryFacade::status` is intentionally synchronous because callers
        // use it while producing lightweight service snapshots.  The wrapped
        // `MemoryRuntimeFacade` exposes richer asynchronous diagnostics, so the
        // web adapter returns the stable capability contract advertised by this
        // runtime instead of blocking inside an async status call.
        macaca_memory::MemoryStatusReport::healthy(
            "web-memory-runtime",
            macaca_memory::MemoryCapabilitySet {
                store: true,
                search: true,
                prompt: true,
                lifecycle: true,
                flush: false,
                artifact: false,
                governance: false,
                knowledge: true,
            },
        )
    }
}
