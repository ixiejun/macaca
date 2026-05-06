//! Workspace memory tools: read (`memory_search`, `memory_get`) and delete (`memory_forget`) with
//! **tombstone registry** integration so compiled knowledge digests cannot resurrect deleted rows.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_memory::{
    MemoryDeleteRequest, MemoryGetRequest, MemoryScope, MemorySearchRequest,
    SharedTombstoneRegistry,
};
use macaca_proto::{ApplicationId, MacacaError, MacacaResult, MemoryId};
use macaca_tools::Tool;
use uuid::Uuid;

use crate::memory_runtime::WebMemoryRuntime;

pub(crate) struct WorkspaceMemorySearchTool {
    pub(crate) runtime: Arc<WebMemoryRuntime>,
    pub(crate) scope: MemoryScope,
    pub(crate) default_limit: u32,
}

#[async_trait]
impl Tool for WorkspaceMemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "Search long-term workspace memory read-only by text query (bounded)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query text" },
                "limit": { "type": "integer", "description": "Max entries to return" }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> MacacaResult<serde_json::Value> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MacacaError::Agent("memory_search requires 'query'".into()))?;
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(self.default_limit as usize)
            .max(1)
            .min(32);
        let results = self
            .runtime
            .search(MemorySearchRequest::new(
                self.scope.clone(),
                query.to_string(),
                limit,
            ))
            .await?;
        let simplified: Vec<_> = results
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id.0.to_string(),
                    "content": e.content,
                    "layer": format!("{:?}", e.layer),
                    "created_at": e.created_at,
                })
            })
            .collect();
        Ok(serde_json::json!({ "entries": simplified }))
    }
}

pub(crate) struct WorkspaceMemoryGetTool {
    pub(crate) runtime: Arc<WebMemoryRuntime>,
    pub(crate) scope: MemoryScope,
}

#[async_trait]
impl Tool for WorkspaceMemoryGetTool {
    fn name(&self) -> &str {
        "memory_get"
    }

    fn description(&self) -> &str {
        "Retrieve a memory entry read-only by id (UUID)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Memory UUID" }
            },
            "required": ["id"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> MacacaResult<serde_json::Value> {
        let id_str = input
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MacacaError::Agent("memory_get requires 'id'".into()))?;
        let uuid = Uuid::parse_str(id_str)
            .map_err(|e| MacacaError::Agent(format!("invalid memory id: {e}")))?;
        let mid = MemoryId(uuid);
        let Some(entry) = self
            .runtime
            .get(MemoryGetRequest {
                scope: self.scope.clone(),
                id: mid,
            })
            .await?
        else {
            return Ok(serde_json::json!({ "found": false }));
        };
        Ok(serde_json::json!({
            "found": true,
            "id": entry.id.0.to_string(),
            "content": entry.content,
            "layer": format!("{:?}", entry.layer),
            "created_at": entry.created_at,
        }))
    }
}

/// Deletes workspace memory row by id **and** records a tombstone for digest/evidence filtering.
pub(crate) struct WorkspaceMemoryForgetTool {
    pub(crate) runtime: Arc<WebMemoryRuntime>,
    pub(crate) scope: MemoryScope,
    pub(crate) tombstones: Arc<SharedTombstoneRegistry>,
}

#[async_trait]
impl Tool for WorkspaceMemoryForgetTool {
    fn name(&self) -> &str {
        "memory_forget"
    }

    fn description(&self) -> &str {
        "Delete a workspace memory row by UUID and mark it tombstoned for digest/evidence suppression."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Memory UUID to delete" }
            },
            "required": ["id"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> MacacaResult<serde_json::Value> {
        let id_str = input
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MacacaError::Agent("memory_forget requires 'id'".into()))?;
        let uuid = Uuid::parse_str(id_str)
            .map_err(|e| MacacaError::Agent(format!("invalid memory id: {e}")))?;
        let mid = MemoryId(uuid);
        self.tombstones.record(mid).await;
        self.runtime
            .delete(MemoryDeleteRequest {
                scope: self.scope.clone(),
                id: mid,
            })
            .await?;
        Ok(serde_json::json!({ "deleted": true, "id": id_str }))
    }
}

/// Build the shared-memory scope used by explicit web memory tools.
///
/// Tools execute without receiving a session id argument from the model, so the adapter binds them
/// to the current application and a stable workspace collaboration namespace at registration time.
/// This keeps the tool path scoped and runtime-backed without introducing prompt-visible scope
/// parameters or application-specific logic.
#[must_use]
pub(crate) fn workspace_tool_scope(application_id: ApplicationId) -> MemoryScope {
    MemoryScope::project_shared(application_id, "workspace")
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_memory::{
        ActiveRecallRequest, ActiveRecallResult, KnowledgeCompileCapability,
        KnowledgeCompileRequest, KnowledgeCompileResult, MemoryRuntimeFacade, MemoryRuntimeStatus,
    };
    use macaca_proto::{MemoryEntry, MemoryLayer};

    struct FakeRuntime {
        entry: MemoryEntry,
    }

    #[async_trait]
    impl MemoryRuntimeFacade for FakeRuntime {
        async fn remember(
            &self,
            _request: macaca_memory::MemoryWriteRequest,
        ) -> MacacaResult<MemoryId> {
            Ok(self.entry.id)
        }

        async fn search(&self, _request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
            Ok(vec![self.entry.clone()])
        }

        async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
            Ok((request.id == self.entry.id).then(|| self.entry.clone()))
        }

        async fn delete(&self, _request: MemoryDeleteRequest) -> MacacaResult<()> {
            Ok(())
        }

        async fn active_recall(
            &self,
            _request: ActiveRecallRequest,
        ) -> MacacaResult<ActiveRecallResult> {
            Ok(ActiveRecallResult {
                provider_id: "fake".into(),
                candidates: Vec::new(),
                selected: vec![self.entry.clone()],
                latency_ms: 0,
                diagnostics: Vec::new(),
            })
        }

        async fn compile_knowledge(
            &self,
            request: KnowledgeCompileRequest,
        ) -> MacacaResult<KnowledgeCompileResult> {
            Ok(macaca_memory::KnowledgeCompiler::default().compile(request))
        }

        async fn status(&self) -> MemoryRuntimeStatus {
            MemoryRuntimeStatus::default()
        }
    }

    fn runtime_with_entry(content: &str) -> Arc<WebMemoryRuntime> {
        Arc::new(WebMemoryRuntime::new(Arc::new(FakeRuntime {
            entry: MemoryEntry {
                id: MemoryId::new(),
                layer: MemoryLayer::Vector,
                content: content.into(),
                metadata: serde_json::Value::Null,
                agent_id: None,
                created_at: Utc::now(),
                expires_at: None,
            },
        })))
    }

    #[tokio::test]
    async fn web_memory_runtime_search_tool_uses_runtime_facade() {
        let tool = WorkspaceMemorySearchTool {
            runtime: runtime_with_entry("runtime-backed search row"),
            scope: workspace_tool_scope(ApplicationId::new()),
            default_limit: 4,
        };

        let output = tool
            .execute(serde_json::json!({ "query": "runtime-backed" }))
            .await
            .unwrap();

        assert_eq!(
            output["entries"][0]["content"],
            serde_json::Value::String("runtime-backed search row".into())
        );
    }
}
