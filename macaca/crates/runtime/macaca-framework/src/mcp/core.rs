//! [`McpClient`] Strategy trait — transport-agnostic MCP server abstraction.

use async_trait::async_trait;
use serde_json::Value;

use super::error::McpError;
use super::types::{McpCallResult, McpResourceDef, McpResourceRead, McpResourceTemplateDef, McpToolDef};

/// Abstraction over an MCP server connection.
#[async_trait]
pub trait McpClient: Send + Sync {
    /// Connect to the MCP server.
    async fn connect(&mut self) -> Result<(), McpError>;
    /// List all available tools on the server.
    async fn list_tools(&mut self) -> Result<Vec<McpToolDef>, McpError>;
    /// Call a tool by name with the given arguments.
    async fn call_tool(&mut self, name: &str, args: Value) -> Result<McpCallResult, McpError>;
    /// List readable MCP resources. Implementations without resource support
    /// return an explicit unsupported protocol error rather than faking empty
    /// success, so operator diagnostics can distinguish absence from no data.
    async fn list_resources(&mut self) -> Result<Vec<McpResourceDef>, McpError> {
        Err(McpError::UnsupportedTransport(
            "mcp resources/list is not supported by this client".into(),
        ))
    }
    /// List resource templates exposed by the MCP server.
    async fn list_resource_templates(&mut self) -> Result<Vec<McpResourceTemplateDef>, McpError> {
        Err(McpError::UnsupportedTransport(
            "mcp resources/templates/list is not supported by this client".into(),
        ))
    }
    /// Read a single MCP resource by URI.
    async fn read_resource(&mut self, _uri: &str) -> Result<McpResourceRead, McpError> {
        Err(McpError::UnsupportedTransport(
            "mcp resources/read is not supported by this client".into(),
        ))
    }
    /// Close the connection gracefully.
    async fn close(&mut self) -> Result<(), McpError>;
    /// Check if connected.
    fn is_connected(&self) -> bool;
}
