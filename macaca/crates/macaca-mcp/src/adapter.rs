//! `McpToolAdapter` — wraps an MCP tool as a native Agent OS `Tool`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::RwLock;

use macaca_proto::{MacacaError, MacacaResult};
use macaca_tools::Tool;

use crate::client::{McpClient, McpContent, McpToolInfo};

/// Adapts a single MCP tool to the `macaca_tools::Tool` trait.
///
/// Holds a shared reference to the `McpClient` so multiple
/// tool adapters can share the same MCP server connection.
pub struct McpToolAdapter {
    client: Arc<RwLock<McpClient>>,
    tool_info: McpToolInfo,
}

impl McpToolAdapter {
    /// Create a new adapter for a specific MCP tool.
    pub fn new(client: Arc<RwLock<McpClient>>, tool_info: McpToolInfo) -> Self {
        Self { client, tool_info }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.tool_info.name
    }

    fn description(&self) -> &str {
        &self.tool_info.description
    }

    fn parameters_schema(&self) -> Value {
        self.tool_info.input_schema.clone()
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let client = self.client.read().await;
        let result = client.call_tool(&self.tool_info.name, input).await?;

        if result.is_error {
            let error_text = result
                .content
                .iter()
                .filter_map(|c| match c {
                    McpContent::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Err(MacacaError::Agent(format!(
                "MCP tool '{}' error: {}",
                self.tool_info.name, error_text
            )));
        }

        // Collect text content into a single result.
        let texts: Vec<&str> = result
            .content
            .iter()
            .filter_map(|c| match c {
                McpContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();

        Ok(serde_json::json!({
            "result": texts.join("\n"),
        }))
    }
}

/// Create Tool adapters for all tools from a connected MCP client.
pub async fn tools_from_client(
    client: Arc<RwLock<McpClient>>,
) -> MacacaResult<Vec<Box<dyn Tool>>> {
    let tools_info = client.read().await.list_tools().await?;
    let mut tools: Vec<Box<dyn Tool>> = Vec::new();
    for info in tools_info {
        tools.push(Box::new(McpToolAdapter::new(Arc::clone(&client), info)));
    }
    Ok(tools)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{McpClient, McpToolInfo, McpTransport};

    async fn make_client() -> Arc<RwLock<McpClient>> {
        let mut client = McpClient::new(
            "test",
            McpTransport::Stdio {
                command: "echo".into(),
                args: vec![],
                env: vec![],
            },
        );
        client.connect().await.unwrap();
        client.register_tools(vec![
            McpToolInfo {
                name: "read".into(),
                description: "Read something".into(),
                input_schema: serde_json::json!({"type": "object"}),
            },
            McpToolInfo {
                name: "write".into(),
                description: "Write something".into(),
                input_schema: serde_json::json!({"type": "object"}),
            },
        ]);
        Arc::new(RwLock::new(client))
    }

    #[tokio::test]
    async fn adapter_exposes_tool_metadata() {
        let client = make_client().await;
        let info = McpToolInfo {
            name: "read".into(),
            description: "Read something".into(),
            input_schema: serde_json::json!({"type": "object"}),
        };
        let adapter = McpToolAdapter::new(client, info);

        assert_eq!(adapter.name(), "read");
        assert_eq!(adapter.description(), "Read something");
    }

    #[tokio::test]
    async fn adapter_execute_returns_result() {
        let client = make_client().await;
        let info = McpToolInfo {
            name: "read".into(),
            description: "Read something".into(),
            input_schema: serde_json::json!({"type": "object"}),
        };
        let adapter = McpToolAdapter::new(client, info);
        let result = adapter.execute(serde_json::json!({"path": "/tmp"})).await.unwrap();
        assert!(result["result"].as_str().unwrap().contains("read"));
    }

    #[tokio::test]
    async fn tools_from_client_creates_all() {
        let client = make_client().await;
        let tools = tools_from_client(client).await.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name(), "read");
        assert_eq!(tools[1].name(), "write");
    }
}
