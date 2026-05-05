//! Read-only workspace memory recall tools (`memory_search`, `memory_get`).

use std::sync::Arc;

use async_trait::async_trait;
use macaca_memory::{RecallQuery, TestMemoryManager};
use macaca_proto::{MacacaError, MacacaResult, MemoryId};
use macaca_tools::Tool;
use uuid::Uuid;

pub(crate) struct WorkspaceMemorySearchTool {
    pub(crate) memory: Arc<TestMemoryManager>,
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
            .memory
            .recall(RecallQuery::new(query.to_string(), limit))
            .await?;
        let simplified: Vec<_> = results
            .entries
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
    pub(crate) memory: Arc<TestMemoryManager>,
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
        let Some(entry) = self.memory.get_entry(&mid).await? else {
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
