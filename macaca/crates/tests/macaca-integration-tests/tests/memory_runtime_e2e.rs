//! End-to-end memory runtime test for the completed memory fabric runtime.
//!
//! This test intentionally uses only in-process builtin components. It verifies that upper-layer
//! callers can depend on the canonical [`macaca_memory::MemoryRuntimeFacade`] while the concrete
//! backing store remains replaceable. No external Milvus, MCP server, remote provider, or LLM is
//! required.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_memory::{
    ActiveRecallBudget, ActiveRecallRequest, BuiltinSessionSharedMemory, ComposedMemoryRuntime,
    KnowledgeCompileRequest, KnowledgeCompiler, MemoryBackendConfig, MemoryBackendFactory,
    MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemoryRuntimeFacade, MemoryScope,
    MemorySearchRequest, MemoryStatusReport, MemoryWriteRequest, TestMemoryManager,
};
use macaca_proto::{ApplicationId, MacacaResult, MemoryEntry, MemoryId, MemoryLayer};
use tempfile::tempdir;

/// Adapter that exposes the real in-process [`TestMemoryManager`] through the scoped fabric
/// contract expected by [`MemoryRuntimeFacade`].
///
/// The production web runtime uses the same Adapter pattern: concrete managers are allowed
/// as builtin backing stores, but callers interact with the runtime facade.
struct SharedManagerFacade {
    manager: Arc<TestMemoryManager>,
}

#[async_trait]
impl MemoryFacade for SharedManagerFacade {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        let mut input = macaca_memory::RememberText::new(request.content)
            .layer(request.layer)
            .metadata(request.metadata);
        if let Some(agent_id) = request.scope.agent_id_value() {
            input = input.agent_id(agent_id);
        }
        self.manager.remember_text(input).await
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        Ok(self
            .manager
            .recall(macaca_memory::RecallQuery::new(
                request.query,
                request.limit,
            ))
            .await?
            .entries)
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        self.manager.get_entry(&request.id).await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        self.manager
            .forget(macaca_memory::ForgetMemory { id: request.id })
            .await
    }

    fn status(&self) -> MemoryStatusReport {
        macaca_memory::MemoryStatusReport::healthy(
            "e2e-shared-manager",
            macaca_memory::MemoryCapabilitySet::basic_store_search(),
        )
    }
}

#[tokio::test]
async fn memory_runtime_facade_e2e_covers_recall_digest_and_delete() {
    let temp = tempdir().unwrap();
    let factory = MemoryBackendFactory::new(MemoryBackendConfig::new(temp.path().to_path_buf()));
    let manager = Arc::new(factory.test_manager());
    let facade = Arc::new(SharedManagerFacade {
        manager: Arc::clone(&manager),
    });
    let active_recall = Arc::new(macaca_memory::DefaultActiveRecallStrategy::new(
        "e2e-active-recall",
        facade.as_ref(),
    ));
    let runtime = ComposedMemoryRuntime::new(
        Arc::clone(&facade),
        active_recall,
        Arc::new(KnowledgeCompiler::default()),
    )
    .with_runtime_id("e2e-memory-runtime")
    .with_provider_profile(Some("builtin-e2e".into()));

    let app_id = ApplicationId::new();
    let scope = MemoryScope::session_shared(app_id, "session-e2e");
    let memory_id = runtime
        .remember(
            MemoryWriteRequest::new(
                scope.clone(),
                "memory runtime e2e keeps deployment runbook evidence",
            )
            .layer(MemoryLayer::Vector),
        )
        .await
        .unwrap();

    let search_rows = runtime
        .search(MemorySearchRequest::new(
            scope.clone(),
            "deployment runbook",
            8,
        ))
        .await
        .unwrap();
    assert!(search_rows.iter().any(|entry| entry.id == memory_id));

    let active = runtime
        .active_recall(ActiveRecallRequest {
            scope: scope.clone(),
            query: "deployment runbook".into(),
            budget: ActiveRecallBudget {
                max_hits: 4,
                max_chars: 4_000,
                max_tokens: 1_000,
                timeout_ms: 1_000,
            },
        })
        .await
        .unwrap();
    assert!(
        active.selected.iter().any(|entry| entry.id == memory_id),
        "active recall should select the remembered row through the runtime facade"
    );

    let compiled = runtime
        .compile_knowledge(KnowledgeCompileRequest {
            scope: scope.clone(),
            candidates: search_rows
                .iter()
                .map(|entry| {
                    macaca_memory::MemoryCandidate::new(
                        entry.id.0.to_string(),
                        scope.clone(),
                        macaca_memory::CandidateSource::AgentSummary,
                        entry.content.clone(),
                    )
                    .confidence(0.95)
                    .recurrence_count(2)
                })
                .collect(),
            existing_claims: Vec::new(),
        })
        .await
        .unwrap();
    assert!(
        compiled
            .claims
            .iter()
            .flat_map(|claim| claim.evidence.iter())
            .any(|evidence| evidence.source_id == memory_id.0.to_string()),
        "knowledge compilation should preserve exact evidence ids"
    );

    runtime
        .delete(MemoryDeleteRequest {
            scope: scope.clone(),
            id: memory_id,
        })
        .await
        .unwrap();
    let deleted = runtime
        .get(MemoryGetRequest {
            scope,
            id: memory_id,
        })
        .await
        .unwrap();
    assert!(deleted.is_none());

    let status = runtime.status().await;
    assert_eq!(status.runtime_id, "e2e-memory-runtime");
    assert!(status.store_available);
    assert!(status.search_available);
    assert!(status.active_recall_available);
    assert!(status.knowledge_available);

    // Also instantiate the builtin adapter to prove the e2e uses the same concrete store type
    // supported by the memory fabric core, without relying on vendor-specific backends.
    let _builtin_shared_adapter = BuiltinSessionSharedMemory::new(manager.as_ref());
}
