//! MCP ↔ Toolkit **Adapter** — bridges remote MCP tools into [`ToolHandler`].
//!
//! [`McpToolHandler`] wraps a single MCP tool definition and delegates execution
//! to a shared [`McpClient`]. [`McpClientResource`] ensures graceful session teardown
//! when the parent [`Toolkit`] is dropped.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{ToolError, ToolHandler, ToolResponse, ToolkitResource};

use super::core::McpClient;
use super::types::McpToolDef;

/// Wraps a single MCP tool so it can be used as a `ToolHandler`.
pub struct McpToolHandler {
    client: Arc<tokio::sync::RwLock<dyn McpClient>>,
    tool_def: McpToolDef,
    backend_tool_name: String,
    registered_name: String,
}

impl McpToolHandler {
    pub fn new(client: Arc<tokio::sync::RwLock<dyn McpClient>>, tool_def: McpToolDef) -> Self {
        let backend_tool_name = tool_def.name.clone();
        let registered_name = tool_def.name.clone();
        Self {
            client,
            tool_def,
            backend_tool_name,
            registered_name,
        }
    }

    pub fn with_registered_name(
        client: Arc<tokio::sync::RwLock<dyn McpClient>>,
        tool_def: McpToolDef,
        registered_name: impl Into<String>,
    ) -> Self {
        Self {
            backend_tool_name: tool_def.name.clone(),
            registered_name: registered_name.into(),
            client,
            tool_def,
        }
    }
}

#[async_trait]
impl ToolHandler for McpToolHandler {
    fn name(&self) -> &str {
        &self.registered_name
    }

    fn description(&self) -> &str {
        &self.tool_def.description
    }

    fn schema(&self) -> Value {
        self.tool_def.input_schema.clone()
    }

    async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError> {
        tracing::debug!(
            target = "macaca_framework::mcp::adapter",
            registered_name = %self.registered_name,
            backend_tool_name = %self.backend_tool_name,
            "executing MCP tool via Toolkit adapter"
        );
        let mut client = self.client.write().await;
        let result = client
            .call_tool(&self.backend_tool_name, args)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResponse {
            content: result.content,
            metadata: result.metadata,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        })
    }
}

/// Toolkit lifecycle hook that closes the shared MCP client on toolkit teardown.
pub(crate) struct McpClientResource {
    pub(crate) client: Arc<tokio::sync::RwLock<dyn McpClient>>,
    pub(crate) on_close: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl ToolkitResource for McpClientResource {
    fn close(self: Box<Self>) {
        let client = Arc::clone(&self.client);
        let on_close = self.on_close.clone();
        tracing::debug!(
            target = "macaca_framework::mcp::adapter",
            has_on_close = on_close.is_some(),
            "closing MCP client resource on toolkit teardown"
        );
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = client.write().await.close().await;
                if let Some(on_close) = on_close {
                    on_close();
                }
            });
        } else if let Some(on_close) = on_close {
            on_close();
        }
    }
}
