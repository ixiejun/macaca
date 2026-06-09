//! Web-owned adapter for the serviceized Memory runtime.
//!
//! The Web shell keeps this adapter at the composition boundary so callers use
//! `SystemMemoryClient` and runtime facades without reaching into concrete
//! memory manager implementations. Provider selection remains in bootstrap and
//! service wiring; route handlers only receive provider-neutral commands.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{MacacaResult, MemoryEntry, MemoryId};
use macaca_sdk::memory::{
    ActiveRecallCapability, ActiveRecallRequest, ActiveRecallResult, DefaultActiveRecallStrategy,
    KnowledgeCompileCapability, KnowledgeCompileRequest, KnowledgeCompileResult, KnowledgeCompiler,
    MemoryCapabilitySet, MemoryDeleteRequest, MemoryFacade, MemoryForgetCommand, MemoryGetCommand,
    MemoryGetRequest, MemoryGetResult, MemoryPrefetchCommand, MemoryPrefetchRequest,
    MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand, MemoryRememberResult,
    MemoryRuntimeFacade, MemoryRuntimeStatus, MemorySearchRequest, MemoryServiceSnapshot,
    MemoryServiceSnapshotCommand, MemoryStatusCommand, MemoryStatusReport, MemoryWriteRequest,
    RecallQuery, RememberText, TestMemoryManager,
};

/// Web-facing adapter over the canonical memory runtime facade.
#[derive(Clone)]
pub struct WebMemoryRuntime {
    inner: Arc<dyn MemoryRuntimeFacade>,
}

impl WebMemoryRuntime {
    /// Wrap a provider-neutral memory runtime facade.
    pub fn new(inner: Arc<dyn MemoryRuntimeFacade>) -> Self {
        Self { inner }
    }

    /// Build the runtime from the default workspace memory manager.
    pub fn from_workspace_memory(memory: Arc<TestMemoryManager>) -> Self {
        Self::new(Arc::new(ManagerWorkspaceMemoryRuntime::new(
            memory,
            "workspace-builtin",
        )))
    }

    /// Build the runtime from a configured memory manager selected by bootstrap.
    pub fn from_configured_memory(
        memory: Arc<macaca_sdk::memory::ConfiguredMemoryManager>,
        provider_profile: impl Into<String>,
    ) -> Self {
        Self::new(Arc::new(ManagerWorkspaceMemoryRuntime::new(
            memory,
            provider_profile,
        )))
    }

    /// Store text content through the selected Memory runtime.
    pub async fn remember_text(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        self.inner.remember(request).await
    }

    /// Search memory entries through the selected Memory runtime.
    pub async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        self.inner.search(request).await
    }

    /// Load one memory entry through the selected Memory runtime.
    pub async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.inner.get(request).await
    }

    /// Delete one memory entry through the selected Memory runtime.
    pub async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        self.inner.delete(request).await
    }

    /// Run active recall through the selected Memory runtime.
    pub async fn active_recall(
        &self,
        request: ActiveRecallRequest,
    ) -> MacacaResult<ActiveRecallResult> {
        self.inner.active_recall(request).await
    }

    /// Compile bounded knowledge artifacts through the selected Memory runtime.
    pub async fn compile_knowledge(
        &self,
        request: KnowledgeCompileRequest,
    ) -> MacacaResult<KnowledgeCompileResult> {
        self.inner.compile_knowledge(request).await
    }

    /// Return current runtime diagnostics.
    pub async fn status(&self) -> MemoryRuntimeStatus {
        self.inner.status().await
    }
}

/// Minimal backend port implemented by concrete MemoryManager instances.
///
/// This Adapter port prevents web routes from knowing whether memory is backed
/// by the in-process test manager, a configured vector store, or a future
/// provider. The shell only translates between service DTOs and the selected
/// runtime boundary.
#[async_trait]
trait WorkspaceMemoryBackend: Send + Sync {
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId>;
    async fn recall(&self, query: RecallQuery) -> MacacaResult<macaca_sdk::memory::RecallResult>;
    async fn get_entry(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>>;
    async fn forget(&self, input: macaca_sdk::memory::ForgetMemory) -> MacacaResult<()>;
}

#[async_trait]
impl WorkspaceMemoryBackend for TestMemoryManager {
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId> {
        TestMemoryManager::remember_text(self, input).await
    }

    async fn recall(&self, query: RecallQuery) -> MacacaResult<macaca_sdk::memory::RecallResult> {
        TestMemoryManager::recall(self, query).await
    }

    async fn get_entry(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>> {
        TestMemoryManager::get_entry(self, id).await
    }

    async fn forget(&self, input: macaca_sdk::memory::ForgetMemory) -> MacacaResult<()> {
        TestMemoryManager::forget(self, input).await
    }
}

#[async_trait]
impl WorkspaceMemoryBackend for macaca_sdk::memory::ConfiguredMemoryManager {
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId> {
        macaca_sdk::memory::ConfiguredMemoryManager::remember_text(self, input).await
    }

    async fn recall(&self, query: RecallQuery) -> MacacaResult<macaca_sdk::memory::RecallResult> {
        macaca_sdk::memory::ConfiguredMemoryManager::recall(self, query).await
    }

    async fn get_entry(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>> {
        macaca_sdk::memory::ConfiguredMemoryManager::get_entry(self, id).await
    }

    async fn forget(&self, input: macaca_sdk::memory::ForgetMemory) -> MacacaResult<()> {
        macaca_sdk::memory::ConfiguredMemoryManager::forget(self, input).await
    }
}

/// Runtime facade over a MemoryManager selected by the Memory backend factory.
struct ManagerWorkspaceMemoryRuntime {
    memory: Arc<dyn WorkspaceMemoryBackend>,
    knowledge: KnowledgeCompiler,
    provider_profile: String,
}

impl ManagerWorkspaceMemoryRuntime {
    fn new<M>(memory: Arc<M>, provider_profile: impl Into<String>) -> Self
    where
        M: WorkspaceMemoryBackend + 'static,
    {
        Self {
            memory,
            knowledge: KnowledgeCompiler,
            provider_profile: provider_profile.into(),
        }
    }
}

#[async_trait]
impl MemoryRuntimeFacade for ManagerWorkspaceMemoryRuntime {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        let mut input = RememberText::new(request.content)
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
            .forget(macaca_sdk::memory::ForgetMemory { id: request.id })
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
            provider_profile: Some(self.provider_profile.clone()),
            store_available: true,
            search_available: true,
            active_recall_available: true,
            knowledge_available: true,
            diagnostics: Vec::new(),
        }
    }
}

#[async_trait]
impl MemoryFacade for ManagerWorkspaceMemoryRuntime {
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
            .forget(macaca_sdk::memory::ForgetMemory { id: request.id })
            .await
    }

    fn status(&self) -> MemoryStatusReport {
        MemoryStatusReport::healthy(
            self.provider_profile.clone(),
            MemoryCapabilitySet {
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

    fn status(&self) -> MemoryStatusReport {
        // `MemoryFacade::status` is intentionally synchronous. Returning the
        // stable advertised capability set avoids blocking inside async runtime
        // diagnostics while still exposing the selected service boundary.
        MemoryStatusReport::healthy(
            "web-memory-runtime",
            MemoryCapabilitySet {
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
impl macaca_sdk::SystemMemoryClient for WebMemoryRuntime {
    async fn remember(&self, command: MemoryRememberCommand) -> MacacaResult<MemoryRememberResult> {
        let id = MemoryRuntimeFacade::remember(
            self,
            MemoryWriteRequest::new(command.scope, command.content)
                .layer(command.layer)
                .metadata(command.metadata),
        )
        .await?;
        Ok(MemoryRememberResult {
            id,
            stored_at: Utc::now(),
        })
    }

    async fn recall(&self, command: MemoryRecallCommand) -> MacacaResult<MemoryRecallResult> {
        let entries = MemoryRuntimeFacade::search(
            self,
            MemorySearchRequest::new(command.scope, command.query, command.limit),
        )
        .await?;
        Ok(MemoryRecallResult::new(entries))
    }

    async fn prefetch(&self, command: MemoryPrefetchCommand) -> MacacaResult<MemoryRecallResult> {
        let entries = MemoryFacade::prefetch(
            self,
            MemoryPrefetchRequest {
                scope: command.scope,
                query: command.query,
                limit: command.limit,
            },
        )
        .await?;
        Ok(MemoryRecallResult::new(entries))
    }

    async fn get(&self, command: MemoryGetCommand) -> MacacaResult<MemoryGetResult> {
        let entry = MemoryRuntimeFacade::get(
            self,
            MemoryGetRequest {
                scope: command.scope,
                id: command.id,
            },
        )
        .await?;
        Ok(MemoryGetResult::new(entry))
    }

    async fn forget(&self, command: MemoryForgetCommand) -> MacacaResult<()> {
        MemoryRuntimeFacade::delete(
            self,
            MemoryDeleteRequest {
                scope: command.scope,
                id: command.id,
            },
        )
        .await
    }

    async fn status(&self, _command: MemoryStatusCommand) -> MacacaResult<MemoryStatusReport> {
        Ok(MemoryStatusReport::healthy(
            "web-memory-runtime",
            MemoryCapabilitySet {
                store: true,
                search: true,
                prompt: true,
                lifecycle: true,
                flush: false,
                artifact: false,
                governance: false,
                knowledge: true,
            },
        ))
    }

    async fn snapshot(
        &self,
        _command: MemoryServiceSnapshotCommand,
    ) -> MacacaResult<MemoryServiceSnapshot> {
        Ok(MemoryServiceSnapshot::new(
            "web-memory-runtime",
            true,
            MemoryCapabilitySet {
                store: true,
                search: true,
                prompt: true,
                lifecycle: true,
                flush: false,
                artifact: false,
                governance: false,
                knowledge: true,
            },
            None,
        ))
    }
}
