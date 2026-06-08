//! Test doubles for Skill experience destination routing contract tests.
//!
//! `RecordingMemoryRuntime` captures write and knowledge-compile requests so tests
//! can prove routing occurs through injected facades without constructing stores.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_memory::{
    ActiveRecallRequest, ActiveRecallResult, KnowledgeCompileRequest, KnowledgeCompileResult,
    MemoryDeleteRequest, MemoryGetRequest, MemoryRuntimeFacade, MemoryRuntimeStatus,
    MemorySearchRequest, MemoryWriteRequest,
};
use macaca_proto::{MacacaResult, MemoryEntry, MemoryId};
use tokio::sync::Mutex;

/// Records memory writes and knowledge compile calls for routing contract assertions.
#[derive(Default)]
pub(super) struct RecordingMemoryRuntime {
    pub(super) writes: Mutex<Vec<MemoryWriteRequest>>,
    pub(super) knowledge_compiles: Mutex<Vec<KnowledgeCompileRequest>>,
}

#[async_trait]
impl MemoryRuntimeFacade for RecordingMemoryRuntime {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        self.writes.lock().await.push(request);
        Ok(MemoryId::new())
    }

    async fn search(&self, _request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn get(&self, _request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        Ok(None)
    }

    async fn delete(&self, _request: MemoryDeleteRequest) -> MacacaResult<()> {
        Ok(())
    }

    async fn active_recall(
        &self,
        _request: ActiveRecallRequest,
    ) -> MacacaResult<ActiveRecallResult> {
        Ok(ActiveRecallResult {
            provider_id: "recording-memory-runtime".into(),
            candidates: Vec::new(),
            selected: Vec::new(),
            latency_ms: 0,
            diagnostics: Vec::new(),
        })
    }

    async fn compile_knowledge(
        &self,
        request: KnowledgeCompileRequest,
    ) -> MacacaResult<KnowledgeCompileResult> {
        self.knowledge_compiles.lock().await.push(request.clone());
        Ok(KnowledgeCompileResult {
            scope: request.scope,
            claims: Vec::new(),
            conflicts: Vec::new(),
            compiled_at: chrono::Utc::now(),
        })
    }

    async fn status(&self) -> MemoryRuntimeStatus {
        MemoryRuntimeStatus {
            runtime_id: "recording-memory-runtime".into(),
            provider_profile: Some("recording".into()),
            store_available: true,
            search_available: true,
            active_recall_available: true,
            knowledge_available: true,
            diagnostics: Vec::new(),
        }
    }
}
