//! Manager-backed memory runtime adapters for the fabric boundary.
//!
//! Design pattern: **Adapter** + **Decorator**.
//! - `ManagerBackedMemoryRuntime` adapts concrete `MemoryManager` implementations
//!   (test stores, configured backends, future providers) to [`MemoryRuntimeFacade`].
//! - `FabricMemoryRuntime` decorates any `Arc<dyn MemoryRuntimeFacade>` so bootstrap
//!   code can share one stable handle across service providers without shell-local types.
//!
//! Upper crates (web shell, CLI, runtime-host) must depend on these provider-neutral
//! adapters through `macaca_sdk::memory` rather than owning duplicate runtime glue.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use crate::core::{
    ActiveRecallCapability, ActiveRecallRequest, ActiveRecallResult, DefaultActiveRecallStrategy,
    MemoryCapabilitySet, MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemorySearchRequest,
    MemoryStatusReport, MemoryWriteRequest,
};
use crate::governance::KnowledgeCompileCapability;
use crate::facade::{ForgetMemory, RecallQuery, RememberText};
use crate::{
    ConfiguredMemoryManager, KnowledgeCompileRequest, KnowledgeCompileResult, KnowledgeCompiler,
    MemoryRuntimeFacade, MemoryRuntimeStatus, TestMemoryManager,
};
use macaca_proto::{MacacaResult, MemoryEntry, MemoryId};

/// Decorator over an existing [`MemoryRuntimeFacade`] implementation.
///
/// Bootstrap registers the same `Arc<FabricMemoryRuntime>` with the memory service
/// provider, skill experience routing, and context recall wiring. The decorator keeps
/// those injection sites stable while the inner facade remains swappable.
#[derive(Clone)]
pub struct FabricMemoryRuntime {
    inner: Arc<dyn MemoryRuntimeFacade>,
    /// Stable runtime label used in diagnostics; defaults to `fabric-memory-runtime`.
    runtime_label: String,
}

impl FabricMemoryRuntime {
    /// Wrap an already constructed runtime facade.
    pub fn new(inner: Arc<dyn MemoryRuntimeFacade>) -> Self {
        info!(
            service_id = "memory",
            command = "fabric_runtime_wrap",
            runtime_label = "fabric-memory-runtime",
            "Fabric memory runtime decorator constructed"
        );
        Self {
            inner,
            runtime_label: "fabric-memory-runtime".into(),
        }
    }

    /// Build a fabric runtime over a test memory manager (bootstrap fixtures / unit tests).
    pub fn from_test_memory(memory: Arc<TestMemoryManager>) -> Self {
        Self::new(Arc::new(ManagerBackedMemoryRuntime::new(
            memory,
            "builtin-test-manager",
        )))
    }

    /// Build a fabric runtime over a configured backend manager selected by
    /// [`crate::MemoryBackendFactory`].
    pub fn from_configured_memory(
        memory: Arc<ConfiguredMemoryManager>,
        provider_profile: impl Into<String>,
    ) -> Self {
        let profile = provider_profile.into();
        info!(
            service_id = "memory",
            command = "fabric_runtime_from_configured_manager",
            provider_profile = %profile,
            "Configured manager-backed memory runtime materialized"
        );
        Self::new(Arc::new(ManagerBackedMemoryRuntime::new(memory, profile)))
    }

    /// Convenience forwarder used by legacy tool adapters during service migration.
    pub async fn remember_text(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        self.inner.remember(request).await
    }

    /// Convenience forwarder used by legacy tool adapters during service migration.
    pub async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        self.inner.search(request).await
    }

    /// Convenience forwarder used by legacy tool adapters during service migration.
    pub async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.inner.get(request).await
    }

    /// Convenience forwarder used by legacy tool adapters during service migration.
    pub async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        self.inner.delete(request).await
    }

    /// Convenience forwarder used by legacy tool adapters during service migration.
    pub async fn active_recall(
        &self,
        request: ActiveRecallRequest,
    ) -> MacacaResult<ActiveRecallResult> {
        self.inner.active_recall(request).await
    }

    /// Convenience forwarder used by legacy tool adapters during service migration.
    pub async fn compile_knowledge(
        &self,
        request: KnowledgeCompileRequest,
    ) -> MacacaResult<KnowledgeCompileResult> {
        self.inner.compile_knowledge(request).await
    }

    /// Convenience forwarder used by legacy tool adapters during service migration.
    pub async fn status(&self) -> MemoryRuntimeStatus {
        self.inner.status().await
    }
}

/// Minimal backend port implemented by concrete memory manager instances.
///
/// The adapter intentionally exposes only the manager operations required by the
/// runtime facade. Provider selection stays inside the memory backend factory so
/// shells never branch on Milvus, file stores, or other vendor-specific types.
#[async_trait]
trait WorkspaceMemoryBackend: Send + Sync {
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId>;
    async fn recall(&self, query: RecallQuery) -> MacacaResult<crate::facade::RecallResult>;
    async fn get_entry(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>>;
    async fn forget(&self, input: ForgetMemory) -> MacacaResult<()>;
}

#[async_trait]
impl WorkspaceMemoryBackend for TestMemoryManager {
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId> {
        TestMemoryManager::remember_text(self, input).await
    }

    async fn recall(&self, query: RecallQuery) -> MacacaResult<crate::facade::RecallResult> {
        TestMemoryManager::recall(self, query).await
    }

    async fn get_entry(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>> {
        TestMemoryManager::get_entry(self, id).await
    }

    async fn forget(&self, input: ForgetMemory) -> MacacaResult<()> {
        TestMemoryManager::forget(self, input).await
    }
}

#[async_trait]
impl WorkspaceMemoryBackend for ConfiguredMemoryManager {
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId> {
        ConfiguredMemoryManager::remember_text(self, input).await
    }

    async fn recall(&self, query: RecallQuery) -> MacacaResult<crate::facade::RecallResult> {
        ConfiguredMemoryManager::recall(self, query).await
    }

    async fn get_entry(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>> {
        ConfiguredMemoryManager::get_entry(self, id).await
    }

    async fn forget(&self, input: ForgetMemory) -> MacacaResult<()> {
        ConfiguredMemoryManager::forget(self, input).await
    }
}

/// Runtime facade over a manager selected by the memory backend factory.
struct ManagerBackedMemoryRuntime {
    memory: Arc<dyn WorkspaceMemoryBackend>,
    knowledge: KnowledgeCompiler,
    provider_profile: String,
}

impl ManagerBackedMemoryRuntime {
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

    fn capability_set(&self) -> MemoryCapabilitySet {
        MemoryCapabilitySet {
            store: true,
            search: true,
            prompt: true,
            lifecycle: true,
            flush: false,
            artifact: false,
            governance: false,
            knowledge: true,
        }
    }
}

#[async_trait]
impl MemoryRuntimeFacade for ManagerBackedMemoryRuntime {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        let mut input = RememberText::new(request.content)
            .layer(request.layer)
            .metadata(request.metadata);
        if let Some(agent_id) = request.scope.agent_id_value() {
            input = input.agent_id(agent_id);
        }
        let id = self.memory.remember_text(input).await?;
        info!(
            service_id = "memory",
            command = "remember",
            memory_id = ?id,
            provider_profile = %self.provider_profile,
            "Manager-backed memory runtime stored entry"
        );
        Ok(id)
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        let entries = self
            .memory
            .recall(RecallQuery::new(request.query, request.limit))
            .await?
            .entries;
        info!(
            service_id = "memory",
            command = "search",
            result_count = entries.len(),
            provider_profile = %self.provider_profile,
            "Manager-backed memory runtime search completed"
        );
        Ok(entries)
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.memory.get_entry(&request.id).await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        self.memory.forget(ForgetMemory { id: request.id }).await
    }

    async fn active_recall(
        &self,
        request: ActiveRecallRequest,
    ) -> MacacaResult<ActiveRecallResult> {
        let strategy =
            DefaultActiveRecallStrategy::new(self.provider_profile.as_str(), self);
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
            runtime_id: "manager-backed-memory-runtime".into(),
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
impl MemoryFacade for ManagerBackedMemoryRuntime {
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
        self.memory.forget(ForgetMemory { id: request.id }).await
    }

    fn status(&self) -> MemoryStatusReport {
        MemoryStatusReport::healthy(self.provider_profile.clone(), self.capability_set())
    }
}

#[async_trait]
impl MemoryRuntimeFacade for FabricMemoryRuntime {
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
impl MemoryFacade for FabricMemoryRuntime {
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
        MemoryStatusReport::healthy(
            self.runtime_label.clone(),
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

/// Backward-compatible alias retained for migration notes and archived specs.
#[deprecated(
    note = "Renamed to FabricMemoryRuntime; web shell ownership removed in unified-call-path P3.1.4"
)]
pub type WebMemoryRuntime = FabricMemoryRuntime;
