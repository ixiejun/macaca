//! Workspace memory tools: read (`memory_search`, `memory_get`) and delete (`memory_forget`) with
//! **tombstone registry** integration so compiled knowledge digests cannot resurrect deleted rows.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_host_composition::memory::SharedTombstoneRegistry;
use macaca_host_composition::tools::{Tool, ToolCommand};
use macaca_proto::{
    ApplicationId, MacacaError, MacacaResult, MemoryForgetCommand, MemoryGetCommand, MemoryId,
    MemoryRecallCommand, MemoryScope, TraceContext,
};
use macaca_sdk::SystemMemoryClient;
use uuid::Uuid;

/// Build a trace context for model-visible memory tool calls.
///
/// Tools do not receive the host request trace directly, so the adapter creates
/// a fresh trace id and attaches the agent/session links known at registration
/// time.  Service providers can then audit each memory operation without
/// exposing trace plumbing to prompts.
fn tool_trace(session_id: Option<&str>, agent_name: Option<&str>) -> TraceContext {
    let mut trace = TraceContext::new(Uuid::new_v4().to_string());
    trace.session_id = session_id.map(str::to_string);
    trace.agent = agent_name.map(str::to_string);
    trace
}

/// Service-backed read-only workspace memory search tool.
///
/// This is the primary protocol service implementation.  It uses `SystemMemoryClient`
/// so Web can swap memory backends through ServiceRuntime without changing the
/// model-visible tool contract.
pub(crate) struct ServiceWorkspaceMemorySearchTool {
    pub(crate) client: Arc<dyn SystemMemoryClient>,
    pub(crate) scope: MemoryScope,
    pub(crate) default_limit: u32,
    pub(crate) session_id: Option<String>,
    pub(crate) agent_name: String,
}

#[async_trait]
impl Tool for ServiceWorkspaceMemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "Search long-term workspace memory read-only by text query (bounded)."
    }

    fn tool_schema(&self) -> serde_json::Value {
        memory_search_schema()
    }

    async fn invoke(&self, command: ToolCommand) -> MacacaResult<serde_json::Value> {
        let input = command.input;
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MacacaError::Agent("memory_search requires 'query'".into()))?;
        let limit = bounded_limit(&input, self.default_limit);
        let command = MemoryRecallCommand::new(
            self.scope.clone(),
            tool_trace(self.session_id.as_deref(), Some(&self.agent_name)),
            query.to_string(),
            limit,
        )?;
        let result = self.client.recall(command).await?;
        Ok(serde_json::json!({ "entries": simplify_entries(&result.entries) }))
    }
}

/// Service-backed read-only workspace memory lookup tool.
pub(crate) struct ServiceWorkspaceMemoryGetTool {
    pub(crate) client: Arc<dyn SystemMemoryClient>,
    pub(crate) scope: MemoryScope,
    pub(crate) session_id: Option<String>,
    pub(crate) agent_name: String,
}

#[async_trait]
impl Tool for ServiceWorkspaceMemoryGetTool {
    fn name(&self) -> &str {
        "memory_get"
    }

    fn description(&self) -> &str {
        "Retrieve a memory entry read-only by id (UUID)."
    }

    fn tool_schema(&self) -> serde_json::Value {
        memory_get_schema()
    }

    async fn invoke(&self, command: ToolCommand) -> MacacaResult<serde_json::Value> {
        let input = command.input;
        let mid = parse_memory_id(&input, "memory_get")?;
        let command = MemoryGetCommand {
            scope: self.scope.clone(),
            trace: tool_trace(self.session_id.as_deref(), Some(&self.agent_name)),
            id: mid,
            policy: Default::default(),
        };
        let result = self.client.get(command).await?;
        let Some(entry) = result.entry else {
            return Ok(serde_json::json!({ "found": false }));
        };
        Ok(simplify_entry_with_found(&entry))
    }
}

/// Service-backed workspace memory delete tool with tombstone propagation.
pub(crate) struct ServiceWorkspaceMemoryForgetTool {
    pub(crate) client: Arc<dyn SystemMemoryClient>,
    pub(crate) scope: MemoryScope,
    pub(crate) tombstones: Arc<SharedTombstoneRegistry>,
    pub(crate) session_id: Option<String>,
    pub(crate) agent_name: String,
}

#[async_trait]
impl Tool for ServiceWorkspaceMemoryForgetTool {
    fn name(&self) -> &str {
        "memory_forget"
    }

    fn description(&self) -> &str {
        "Delete a workspace memory row by UUID and mark it tombstoned for digest/evidence suppression."
    }

    fn tool_schema(&self) -> serde_json::Value {
        memory_forget_schema()
    }

    async fn invoke(&self, command: ToolCommand) -> MacacaResult<serde_json::Value> {
        let input = command.input;
        let mid = parse_memory_id(&input, "memory_forget")?;
        self.tombstones.record(mid).await;
        self.client
            .forget(MemoryForgetCommand {
                scope: self.scope.clone(),
                trace: tool_trace(self.session_id.as_deref(), Some(&self.agent_name)),
                id: mid,
                policy: Default::default(),
            })
            .await?;
        Ok(serde_json::json!({ "deleted": true, "id": mid.0.to_string() }))
    }
}

fn bounded_limit(input: &serde_json::Value, default_limit: u32) -> usize {
    input
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(default_limit as usize)
        .max(1)
        .min(32)
}

fn parse_memory_id(input: &serde_json::Value, tool_name: &str) -> MacacaResult<MemoryId> {
    let id_str = input
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| MacacaError::Agent(format!("{tool_name} requires 'id'")))?;
    Uuid::parse_str(id_str)
        .map(MemoryId)
        .map_err(|e| MacacaError::Agent(format!("invalid memory id: {e}")))
}

fn simplify_entries(entries: &[macaca_proto::MemoryEntry]) -> Vec<serde_json::Value> {
    entries
        .iter()
        .map(|entry| {
            serde_json::json!({
                "id": entry.id.0.to_string(),
                "content": entry.content,
                "layer": format!("{:?}", entry.layer),
                "created_at": entry.created_at,
            })
        })
        .collect()
}

fn simplify_entry_with_found(entry: &macaca_proto::MemoryEntry) -> serde_json::Value {
    serde_json::json!({
        "found": true,
        "id": entry.id.0.to_string(),
        "content": entry.content,
        "layer": format!("{:?}", entry.layer),
        "created_at": entry.created_at,
    })
}

fn memory_search_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Search query text" },
            "limit": { "type": "integer", "description": "Max entries to return" }
        },
        "required": ["query"]
    })
}

fn memory_get_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "description": "Memory UUID" }
        },
        "required": ["id"]
    })
}

fn memory_forget_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "description": "Memory UUID to delete" }
        },
        "required": ["id"]
    })
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
