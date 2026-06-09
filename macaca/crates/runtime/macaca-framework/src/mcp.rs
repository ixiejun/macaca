//! MCP (Model Context Protocol) support.
//!
//! Provides:
//! - `McpClient` trait for connecting to MCP servers
//! - `StdioMcpClient` — JSON-RPC 2.0 over child-process stdin/stdout
//! - `McpToolHandler` — bridges MCP tools into the `Toolkit` system
//! - `register_mcp_tools` — bulk-registers MCP tools into a `Toolkit`

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::message::{ContentBlock, TextBlock};

#[path = "mcp_http.rs"]
mod mcp_http;
#[path = "mcp_stdio.rs"]
mod mcp_stdio;
#[path = "mcp_toolkit.rs"]
mod mcp_toolkit;

#[cfg(feature = "mcp-http")]
pub use mcp_http::{HttpMcpClient, HttpMcpTransport};
pub use mcp_stdio::StdioMcpClient;
pub use mcp_toolkit::{register_mcp_tools, register_mcp_tools_with_options, McpToolHandler};

#[cfg(test)]
use crate::tool::{ToolError, ToolHandler, ToolResponse, Toolkit};

#[cfg(all(test, feature = "mcp-http"))]
use mcp_http::parse_http_mcp_response;

// ---------------------------------------------------------------------------
// McpError
// ---------------------------------------------------------------------------

/// Errors specific to MCP operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum McpError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Execution error: {0}")]
    Execution(String),
    #[error("Timeout")]
    Timeout,
    #[error("Not connected")]
    NotConnected,
    #[error("Already connected")]
    AlreadyConnected,
    #[error("IO error: {0}")]
    Io(String),
    #[error("Tool name collision: {0}")]
    ToolNameCollision(String),
    #[error("Unsupported transport: {0}")]
    UnsupportedTransport(String),
}

// ---------------------------------------------------------------------------
// MCP configuration
// ---------------------------------------------------------------------------

/// Transport configuration for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum McpTransportConfig {
    /// Launch a local MCP server process and communicate over stdio.
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
        #[serde(default)]
        cwd: Option<PathBuf>,
    },
    /// Connect to an MCP server over SSE transport.
    Sse {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
    /// Connect to an MCP server over streamable HTTP transport.
    StreamableHttp {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
}

/// Session behavior for an MCP client.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpSessionMode {
    /// One initialized MCP session is reused until close.
    Stateful,
    /// A scoped MCP session may be opened per logical call by higher layers.
    Stateless,
}

/// Timeout configuration for MCP operations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpTimeouts {
    pub connect: Duration,
    pub list_tools: Duration,
    pub call_tool: Duration,
}

impl Default for McpTimeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(15),
            list_tools: Duration::from_secs(15),
            call_tool: Duration::from_secs(60),
        }
    }
}

/// Deterministic policy for MCP tool name collisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpToolNameConflictPolicy {
    /// Fail registration if any MCP tool name already exists.
    Raise,
    /// Skip colliding tools.
    Skip,
    /// Register tools with this prefix.
    Prefix(String),
}

/// Options used when registering MCP tools into a framework [`Toolkit`].
#[derive(Clone)]
pub struct McpToolRegistrationOptions {
    pub group_name: String,
    pub conflict_policy: McpToolNameConflictPolicy,
    pub disabled_tools: HashSet<String>,
    pub on_close: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl McpToolRegistrationOptions {
    pub fn new(group_name: impl Into<String>) -> Self {
        Self {
            group_name: group_name.into(),
            conflict_policy: McpToolNameConflictPolicy::Raise,
            disabled_tools: HashSet::new(),
            on_close: None,
        }
    }
}

// ---------------------------------------------------------------------------
// MCP type definitions
// ---------------------------------------------------------------------------

/// An MCP tool definition received from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    #[serde(default, alias = "inputSchema")]
    pub input_schema: Value,
}

/// Result from calling an MCP tool.
#[derive(Debug, Clone)]
pub struct McpCallResult {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub metadata: Option<Value>,
}

/// Resource metadata returned by MCP `resources/list`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpResourceDef {
    pub uri: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, alias = "mimeType")]
    pub mime_type: Option<String>,
}

/// Template metadata returned by MCP `resources/templates/list`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpResourceTemplateDef {
    #[serde(alias = "uriTemplate")]
    pub uri_template: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, alias = "mimeType")]
    pub mime_type: Option<String>,
}

/// Bounded resource content returned by MCP `resources/read`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpResourceRead {
    pub uri: String,
    #[serde(default, alias = "mimeType")]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub blob: Option<String>,
}

// ---------------------------------------------------------------------------
// McpClient trait
// ---------------------------------------------------------------------------

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

/// Build a framework MCP client from transport configuration.
pub fn client_from_transport(
    config: McpTransportConfig,
    timeouts: McpTimeouts,
) -> Result<Box<dyn McpClient>, McpError> {
    match config {
        McpTransportConfig::Stdio {
            command,
            args,
            env,
            cwd,
        } => Ok(Box::new(StdioMcpClient::from_stdio_config(
            command, args, env, cwd, timeouts,
        ))),
        McpTransportConfig::Sse { url, headers } => {
            #[cfg(feature = "mcp-http")]
            {
                Ok(Box::new(HttpMcpClient::new(
                    HttpMcpTransport::Sse,
                    url,
                    headers,
                    timeouts,
                )))
            }
            #[cfg(not(feature = "mcp-http"))]
            {
                let _ = (url, headers);
                Err(McpError::UnsupportedTransport(
                    "sse requires macaca-framework feature mcp-http".to_string(),
                ))
            }
        }
        McpTransportConfig::StreamableHttp { url, headers } => {
            #[cfg(feature = "mcp-http")]
            {
                Ok(Box::new(HttpMcpClient::new(
                    HttpMcpTransport::StreamableHttp,
                    url,
                    headers,
                    timeouts,
                )))
            }
            #[cfg(not(feature = "mcp-http"))]
            {
                let _ = (url, headers);
                Err(McpError::UnsupportedTransport(
                    "streamable_http requires macaca-framework feature mcp-http".to_string(),
                ))
            }
        }
    }
}

/// Parse a `tools/call` result value into an `McpCallResult`.
pub(crate) fn parse_call_result(result: &Value) -> Result<McpCallResult, McpError> {
    let is_error = result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let content_arr = result
        .get("content")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut blocks = Vec::new();
    for item in &content_arr {
        let block_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("text");
        match block_type {
            "text" => {
                let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                blocks.push(ContentBlock::Text(TextBlock {
                    text: text.to_string(),
                }));
            }
            "image" => {
                blocks.push(ContentBlock::Image(crate::message::ImageBlock {
                    data: item
                        .get("data")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    url: item
                        .get("url")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    mime_type: item
                        .get("mimeType")
                        .or_else(|| item.get("mime_type"))
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                }));
            }
            "audio" => {
                blocks.push(ContentBlock::Audio(crate::message::AudioBlock {
                    data: item
                        .get("data")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    url: item
                        .get("url")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    mime_type: item
                        .get("mimeType")
                        .or_else(|| item.get("mime_type"))
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                }));
            }
            "resource" => {
                let text = item
                    .get("resource")
                    .and_then(|resource| {
                        resource
                            .get("text")
                            .and_then(|v| v.as_str())
                            .map(ToString::to_string)
                            .or_else(|| serde_json::to_string(resource).ok())
                    })
                    .or_else(|| serde_json::to_string(item).ok())
                    .unwrap_or_default();
                blocks.push(ContentBlock::Text(TextBlock { text }));
            }
            _ => {
                // Fallback: wrap the whole item as JSON text.
                blocks.push(ContentBlock::Text(TextBlock {
                    text: serde_json::to_string(item).unwrap_or_default(),
                }));
            }
        }
    }

    if blocks.is_empty() {
        blocks.push(ContentBlock::Text(TextBlock {
            text: String::new(),
        }));
    }

    let metadata = result.get("_meta").cloned();

    Ok(McpCallResult {
        content: blocks,
        is_error,
        metadata,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;

#[cfg(test)]
#[path = "mcp_http_tests.rs"]
mod mcp_http_tests;
