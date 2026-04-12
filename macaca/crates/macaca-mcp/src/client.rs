//! MCP client — connects to MCP servers via stdio or SSE transport.
//!
//! This is a simplified MCP client that handles the core tool listing
//! and invocation protocol. Full MCP spec compliance (resources, prompts,
//! sampling) can be added incrementally.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use macaca_proto::{MacacaError, MacacaResult};

/// Transport configuration for connecting to an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpTransport {
    /// Launch the MCP server as a subprocess, communicating via stdin/stdout.
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: Vec<(String, String)>,
    },
    /// Connect to the MCP server via HTTP Server-Sent Events.
    Sse { url: String },
}

/// Information about a tool exposed by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    /// Tool name.
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    #[serde(default = "default_schema")]
    pub input_schema: Value,
}

fn default_schema() -> Value {
    serde_json::json!({"type": "object", "properties": {}})
}

/// Result from calling an MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    /// The content returned by the tool.
    pub content: Vec<McpContent>,
    /// Whether the tool call resulted in an error.
    #[serde(default)]
    pub is_error: bool,
}

/// A content block in an MCP tool result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { uri: String, text: Option<String> },
}

/// Client for communicating with an MCP server.
///
/// Supports listing available tools and invoking them.
/// Currently a struct-based abstraction — actual transport I/O
/// (subprocess management, SSE streaming) is stubbed for the
/// initial implementation and can be fleshed out incrementally.
pub struct McpClient {
    transport: McpTransport,
    server_name: String,
    tools: Vec<McpToolInfo>,
    connected: bool,
}

impl McpClient {
    /// Create a new MCP client with the given transport configuration.
    pub fn new(server_name: impl Into<String>, transport: McpTransport) -> Self {
        Self {
            transport,
            server_name: server_name.into(),
            tools: Vec::new(),
            connected: false,
        }
    }

    /// Get the server name.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Get the transport configuration.
    pub fn transport(&self) -> &McpTransport {
        &self.transport
    }

    /// Whether the client is connected to the server.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Connect to the MCP server and perform initialization handshake.
    ///
    /// In a full implementation, this would:
    /// - For Stdio: spawn the subprocess, send `initialize` JSON-RPC
    /// - For SSE: connect to the endpoint, send `initialize`
    ///
    /// Currently sets connected state. Real I/O will be added when
    /// integrating with actual MCP servers.
    pub async fn connect(&mut self) -> MacacaResult<()> {
        tracing::debug!(server = %self.server_name, "connecting to MCP server");
        self.connected = true;
        Ok(())
    }

    /// Disconnect from the MCP server.
    pub async fn disconnect(&mut self) -> MacacaResult<()> {
        tracing::debug!(server = %self.server_name, "disconnecting from MCP server");
        self.connected = false;
        self.tools.clear();
        Ok(())
    }

    /// List the tools available from the MCP server.
    ///
    /// In a full implementation, sends `tools/list` JSON-RPC request.
    /// Currently returns tools registered via `register_tools()`.
    pub async fn list_tools(&self) -> MacacaResult<Vec<McpToolInfo>> {
        if !self.connected {
            return Err(MacacaError::Agent("MCP client not connected".into()));
        }
        Ok(self.tools.clone())
    }

    /// Call a tool on the MCP server.
    ///
    /// In a full implementation, sends `tools/call` JSON-RPC request.
    /// Currently returns a stub error for unregistered tools.
    pub async fn call_tool(&self, name: &str, arguments: Value) -> MacacaResult<McpToolResult> {
        if !self.connected {
            return Err(MacacaError::Agent("MCP client not connected".into()));
        }

        // Verify the tool exists.
        if !self.tools.iter().any(|t| t.name == name) {
            return Err(MacacaError::NotFound(format!(
                "MCP tool '{}' not found on server '{}'",
                name, self.server_name
            )));
        }

        tracing::debug!(
            server = %self.server_name,
            tool = %name,
            "calling MCP tool"
        );

        // Stub: return the arguments as a text result.
        // Real implementation would send JSON-RPC to the transport.
        Ok(McpToolResult {
            content: vec![McpContent::Text {
                text: format!("MCP stub: called {} with {}", name, arguments),
            }],
            is_error: false,
        })
    }

    /// Register tools (for testing or manual configuration).
    pub fn register_tools(&mut self, tools: Vec<McpToolInfo>) {
        self.tools = tools;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_transport_serialize() {
        let t = McpTransport::Stdio {
            command: "npx".into(),
            args: vec!["mcp-server".into()],
            env: vec![],
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("npx"));
    }

    #[test]
    fn mcp_tool_info_deserialize() {
        let json = r#"{"name": "read_file", "description": "Read a file"}"#;
        let info: McpToolInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.name, "read_file");
        assert_eq!(info.description, "Read a file");
    }

    #[tokio::test]
    async fn client_lifecycle() {
        let mut client = McpClient::new(
            "test-server",
            McpTransport::Stdio {
                command: "echo".into(),
                args: vec![],
                env: vec![],
            },
        );

        assert!(!client.is_connected());
        assert!(client.list_tools().await.is_err()); // Not connected

        client.connect().await.unwrap();
        assert!(client.is_connected());

        client.register_tools(vec![McpToolInfo {
            name: "greet".into(),
            description: "Say hello".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }]);

        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "greet");

        let result = client
            .call_tool("greet", serde_json::json!({"name": "world"}))
            .await
            .unwrap();
        assert!(!result.is_error);

        // Unknown tool
        assert!(client
            .call_tool("unknown", serde_json::json!({}))
            .await
            .is_err());

        client.disconnect().await.unwrap();
        assert!(!client.is_connected());
    }
}
