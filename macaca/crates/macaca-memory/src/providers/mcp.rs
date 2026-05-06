use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult, MemoryEntry, MemoryId};

use crate::core::facade::{
    MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemorySearchRequest, MemoryWriteRequest,
};
use crate::core::provider::{MemoryProvider, MemoryProviderDescriptor};
use crate::core::status::MemoryCapabilitySet;
use crate::core::status::MemoryStatusReport;

use super::config::{MemoryProviderConfig, MemoryProviderMcpServerConfig};
use super::resilience::{redact_text, MemoryProviderResilience};

/// Tool names expected by the MCP memory provider.
#[derive(Debug, Clone, PartialEq, Eq)]
struct McpMemoryToolNames {
    remember: String,
    search: String,
    get: String,
    delete: String,
}

impl McpMemoryToolNames {
    fn from_config(config: &MemoryProviderConfig) -> Self {
        let mut tools = HashMap::new();
        for tool in &config.tools {
            let key = normalize_tool_key(&tool.name);
            tools.entry(key).or_insert_with(|| tool.name.clone());
        }
        Self {
            remember: tools
                .get("write")
                .or_else(|| tools.get("store"))
                .or_else(|| tools.get("remember"))
                .cloned()
                .unwrap_or_else(|| "memory_write".into()),
            search: tools
                .get("search")
                .cloned()
                .unwrap_or_else(|| "memory_search".into()),
            get: tools
                .get("get")
                .cloned()
                .unwrap_or_else(|| "memory_get".into()),
            delete: tools
                .get("delete")
                .or_else(|| tools.get("forget"))
                .cloned()
                .unwrap_or_else(|| "memory_delete".into()),
        }
    }
}

fn normalize_tool_key(name: &str) -> String {
    let lowered = name.to_ascii_lowercase();
    for prefix in ["memory_", "memory.", "memory-"] {
        if let Some(rest) = lowered.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    lowered
}

/// Minimal live MCP client seam used by tests and runtime adapters.
#[async_trait]
pub trait MemoryMcpClient: Send + Sync {
    async fn call_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> MacacaResult<serde_json::Value>;
}
/// MCP-backed memory provider adapter.
///
/// The adapter does not assume a specific MCP transport implementation beyond
/// the configured command and tool names. It keeps the provider pluggable by
/// treating the server as an external capability boundary rather than a Rust
/// dependency.
pub struct McpMemoryProvider {
    provider_id: String,
    display_name: String,
    server: MemoryProviderMcpServerConfig,
    resilience: MemoryProviderResilience,
    tools: McpMemoryToolNames,
    client: Option<Arc<dyn MemoryMcpClient>>,
}

impl McpMemoryProvider {
    /// Build an MCP provider from a memory provider config.
    pub fn new(config: &MemoryProviderConfig, server: MemoryProviderMcpServerConfig) -> Self {
        Self {
            provider_id: config.id.clone(),
            display_name: config
                .display_name
                .clone()
                .unwrap_or_else(|| config.id.clone()),
            server,
            resilience: MemoryProviderResilience::new(&config.resilience),
            tools: McpMemoryToolNames::from_config(config),
            client: None,
        }
    }

    /// Attach a live MCP client and enable operational store/search capabilities.
    #[must_use]
    pub fn with_client(mut self, client: Arc<dyn MemoryMcpClient>) -> Self {
        self.client = Some(client);
        self
    }

    fn command_line(&self) -> String {
        let mut parts = vec![self.server.command.clone()];
        parts.extend(self.server.args.clone());
        parts.join(" ")
    }

    fn redact(&self, text: &str) -> String {
        let mut markers = vec![self.server.command.clone()];
        markers.extend(self.server.args.clone());
        markers.extend(self.server.env.values().cloned());
        redact_text(text, &markers)
    }

    /// Report the configured server diagnostics string.
    pub fn diagnostics(&self) -> String {
        self.redact(&self.command_line())
    }

    fn unavailable_error(&self, operation: &str) -> MacacaError {
        MacacaError::Memory(format!(
            "mcp_transport_unwired: live MCP client unavailable for {operation}"
        ))
    }

    fn capabilities(&self) -> MemoryCapabilitySet {
        let live = self.client.is_some();
        MemoryCapabilitySet {
            store: live,
            search: live,
            prompt: live,
            lifecycle: live,
            flush: live,
            artifact: false,
            governance: live,
            knowledge: false,
        }
    }

    async fn call_required_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        operation: &str,
    ) -> MacacaResult<serde_json::Value> {
        let Some(client) = &self.client else {
            return Err(self.unavailable_error(operation));
        };
        client.call_tool(tool_name, input).await.map_err(|error| {
            MacacaError::Memory(format!(
                "mcp provider {} {operation} tool {tool_name} failed: {error}",
                self.provider_id
            ))
        })
    }
}

#[async_trait::async_trait]
impl MemoryFacade for McpMemoryProvider {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        let tool_name = self.tools.remember.clone();
        self.resilience
            .execute("mcp.write", self.command_line().len(), || async {
                let value = self
                    .call_required_tool(
                        &tool_name,
                        serde_json::json!({
                            "scope": request.scope,
                            "content": request.content,
                            "layer": request.layer,
                            "metadata": request.metadata,
                        }),
                        "write",
                    )
                    .await?;
                parse_memory_id(value)
            })
            .await
    }

    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        let tool_name = self.tools.search.clone();
        self.resilience
            .execute("mcp.search", self.command_line().len(), || async {
                let value = self
                    .call_required_tool(
                        &tool_name,
                        serde_json::json!({
                            "scope": request.scope,
                            "query": request.query,
                            "limit": request.limit,
                        }),
                        "search",
                    )
                    .await?;
                parse_memory_entries(value)
            })
            .await
    }

    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        let tool_name = self.tools.get.clone();
        self.resilience
            .execute("mcp.get", self.command_line().len(), || async {
                let value = self
                    .call_required_tool(
                        &tool_name,
                        serde_json::json!({
                            "scope": request.scope,
                            "id": request.id.0.to_string(),
                        }),
                        "get",
                    )
                    .await?;
                parse_optional_memory_entry(value)
            })
            .await
    }

    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()> {
        let tool_name = self.tools.delete.clone();
        self.resilience
            .execute("mcp.delete", self.command_line().len(), || async {
                let _ = self
                    .call_required_tool(
                        &tool_name,
                        serde_json::json!({
                            "scope": request.scope,
                            "id": request.id.0.to_string(),
                        }),
                        "delete",
                    )
                    .await?;
                Ok(())
            })
            .await
    }

    fn status(&self) -> MemoryStatusReport {
        let live = self.client.is_some();
        MemoryStatusReport {
            provider_id: self.provider_id.clone(),
            healthy: live,
            capabilities: self.capabilities(),
            message: (!live).then(|| "mcp_transport_unwired".into()),
        }
    }
}

impl MemoryProvider for McpMemoryProvider {
    fn descriptor(&self) -> MemoryProviderDescriptor {
        MemoryProviderDescriptor::new(
            self.provider_id.clone(),
            self.display_name.clone(),
            self.capabilities(),
        )
    }
}

/// Helper for callers that only need a diagnostics string.
pub fn mcp_provider_diagnostics(provider: &McpMemoryProvider) -> String {
    provider.diagnostics()
}

fn parse_memory_id(value: serde_json::Value) -> MacacaResult<MemoryId> {
    if let Some(id) = value.get("id").and_then(serde_json::Value::as_str) {
        return parse_uuid_id(id);
    }
    if let Some(id) = value.get("memory_id").and_then(serde_json::Value::as_str) {
        return parse_uuid_id(id);
    }
    if let Some(id) = value.as_str() {
        return parse_uuid_id(id);
    }
    Err(MacacaError::Memory(
        "mcp memory write returned invalid id schema".into(),
    ))
}

fn parse_uuid_id(value: &str) -> MacacaResult<MemoryId> {
    uuid::Uuid::parse_str(value)
        .map(MemoryId)
        .map_err(|error| MacacaError::Memory(format!("invalid memory id from mcp: {error}")))
}

fn parse_memory_entries(value: serde_json::Value) -> MacacaResult<Vec<MemoryEntry>> {
    if let Some(entries) = value.get("entries") {
        return serde_json::from_value(entries.clone()).map_err(schema_error);
    }
    if value.is_array() {
        return serde_json::from_value(value).map_err(schema_error);
    }
    Err(MacacaError::Memory(
        "mcp memory search returned invalid entries schema".into(),
    ))
}

fn parse_optional_memory_entry(value: serde_json::Value) -> MacacaResult<Option<MemoryEntry>> {
    if value.get("found").and_then(serde_json::Value::as_bool) == Some(false) || value.is_null() {
        return Ok(None);
    }
    if let Some(entry) = value.get("entry") {
        return serde_json::from_value(entry.clone())
            .map(Some)
            .map_err(schema_error);
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(schema_error)
}

fn schema_error(error: serde_json::Error) -> MacacaError {
    MacacaError::Memory(format!(
        "mcp memory response schema validation failed: {error}"
    ))
}
