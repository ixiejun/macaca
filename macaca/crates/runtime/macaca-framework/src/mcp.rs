//! MCP (Model Context Protocol) support.
//!
//! Provides:
//! - `McpClient` trait for connecting to MCP servers
//! - `StdioMcpClient` — JSON-RPC 2.0 over child-process stdin/stdout
//! - `McpToolHandler` — bridges MCP tools into the `Toolkit` system
//! - `register_mcp_tools` — bulk-registers MCP tools into a `Toolkit`

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::time::timeout;

use crate::message::{ContentBlock, TextBlock};
use crate::tool::{ToolError, ToolHandler, ToolResponse, Toolkit, ToolkitResource};

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
    /// Connect to an MCP server over legacy SSE transport.
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

// ---------------------------------------------------------------------------
// StdioMcpClient
// ---------------------------------------------------------------------------

/// MCP client that communicates with a child process via JSON-RPC 2.0 over
/// stdin/stdout.
pub struct StdioMcpClient {
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    cwd: Option<PathBuf>,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<BufReader<ChildStdout>>,
    connected: bool,
    request_id: AtomicU64,
    cached_tools: Option<Vec<McpToolDef>>,
    timeouts: McpTimeouts,
}

impl StdioMcpClient {
    /// Create a new client. The child process is **not** started until
    /// [`connect`] is called.
    pub fn new(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            command: command.into(),
            args,
            env: BTreeMap::new(),
            cwd: None,
            child: None,
            stdin: None,
            stdout_reader: None,
            connected: false,
            request_id: AtomicU64::new(1),
            cached_tools: None,
            timeouts: McpTimeouts::default(),
        }
    }

    /// Create a stdio client from a transport config.
    pub fn from_stdio_config(
        command: impl Into<String>,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        cwd: Option<PathBuf>,
        timeouts: McpTimeouts,
    ) -> Self {
        Self {
            command: command.into(),
            args,
            env,
            cwd,
            child: None,
            stdin: None,
            stdout_reader: None,
            connected: false,
            request_id: AtomicU64::new(1),
            cached_tools: None,
            timeouts,
        }
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Send a JSON-RPC request (with `id`) and return the parsed response.
    async fn send_request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, McpError> {
        let id = self.next_id();
        let mut req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        });
        if let Some(p) = params {
            req["params"] = p;
        }
        self.write_message(&req).await?;
        timeout(self.timeouts.call_tool, self.read_response())
            .await
            .map_err(|_| McpError::Timeout)?
    }

    /// Send a JSON-RPC notification (no `id`, no response expected).
    async fn send_notification(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), McpError> {
        let mut req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        if let Some(p) = params {
            req["params"] = p;
        }
        self.write_message(&req).await
    }

    async fn write_message(&mut self, value: &Value) -> Result<(), McpError> {
        let stdin = self.stdin.as_mut().ok_or(McpError::NotConnected)?;
        let mut line =
            serde_json::to_string(value).map_err(|e| McpError::Protocol(e.to_string()))?;
        line.push('\n');
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| McpError::Io(e.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|e| McpError::Io(e.to_string()))?;
        Ok(())
    }

    async fn read_response(&mut self) -> Result<Value, McpError> {
        let reader = self.stdout_reader.as_mut().ok_or(McpError::NotConnected)?;
        let mut line = String::new();
        // Read lines, skipping empty ones and notifications (no "id" field).
        loop {
            line.clear();
            let n = reader
                .read_line(&mut line)
                .await
                .map_err(|e| McpError::Io(e.to_string()))?;
            if n == 0 {
                return Err(McpError::Connection("EOF from MCP server".into()));
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parsed: Value =
                serde_json::from_str(trimmed).map_err(|e| McpError::Protocol(e.to_string()))?;
            // Skip notifications (messages without "id").
            if parsed.get("id").is_some() {
                if let Some(err) = parsed.get("error") {
                    let msg = err["message"].as_str().unwrap_or("unknown error");
                    return Err(McpError::Protocol(msg.to_string()));
                }
                return Ok(parsed["result"].clone());
            }
        }
    }
}

impl Drop for StdioMcpClient {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
    }
}

#[async_trait]
impl McpClient for StdioMcpClient {
    async fn connect(&mut self) -> Result<(), McpError> {
        if self.connected {
            return Err(McpError::AlreadyConnected);
        }

        let mut command = tokio::process::Command::new(&self.command);
        command
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }
        if !self.env.is_empty() {
            command.envs(&self.env);
        }
        let mut child = command
            .spawn()
            .map_err(|e| McpError::Connection(e.to_string()))?;

        self.stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Connection("failed to capture child stdout".into()))?;
        self.stdout_reader = Some(BufReader::new(stdout));
        self.child = Some(child);

        // Send initialize request.
        let init_params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "macaca",
                "version": "0.1.0"
            }
        });
        let _result = timeout(
            self.timeouts.connect,
            self.send_request("initialize", Some(init_params)),
        )
        .await
        .map_err(|_| McpError::Timeout)??;

        // Send initialized notification.
        self.send_notification("notifications/initialized", None)
            .await?;

        self.connected = true;
        Ok(())
    }

    async fn list_tools(&mut self) -> Result<Vec<McpToolDef>, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }
        let result = timeout(
            self.timeouts.list_tools,
            self.send_request("tools/list", None),
        )
        .await
        .map_err(|_| McpError::Timeout)??;
        let tools_value = result.get("tools").cloned().unwrap_or(Value::Array(vec![]));
        let tools: Vec<McpToolDef> =
            serde_json::from_value(tools_value).map_err(|e| McpError::Protocol(e.to_string()))?;
        self.cached_tools = Some(tools.clone());
        Ok(tools)
    }

    async fn call_tool(&mut self, name: &str, args: Value) -> Result<McpCallResult, McpError> {
        self.call_tool_mut(name, args).await
    }

    async fn list_resources(&mut self) -> Result<Vec<McpResourceDef>, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }
        let result = timeout(
            self.timeouts.call_tool,
            self.send_request("resources/list", None),
        )
        .await
        .map_err(|_| McpError::Timeout)??;
        let resources = result
            .get("resources")
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        serde_json::from_value(resources).map_err(|e| McpError::Protocol(e.to_string()))
    }

    async fn list_resource_templates(&mut self) -> Result<Vec<McpResourceTemplateDef>, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }
        let result = timeout(
            self.timeouts.call_tool,
            self.send_request("resources/templates/list", None),
        )
        .await
        .map_err(|_| McpError::Timeout)??;
        let templates = result
            .get("resourceTemplates")
            .or_else(|| result.get("resource_templates"))
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        serde_json::from_value(templates).map_err(|e| McpError::Protocol(e.to_string()))
    }

    async fn read_resource(&mut self, uri: &str) -> Result<McpResourceRead, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }
        let result = timeout(
            self.timeouts.call_tool,
            self.send_request("resources/read", Some(serde_json::json!({ "uri": uri }))),
        )
        .await
        .map_err(|_| McpError::Timeout)??;
        let mut contents: Vec<McpResourceRead> = serde_json::from_value(
            result
                .get("contents")
                .cloned()
                .unwrap_or_else(|| Value::Array(vec![])),
        )
        .map_err(|e| McpError::Protocol(e.to_string()))?;
        contents
            .pop()
            .ok_or_else(|| McpError::Protocol("resource read returned no contents".into()))
    }

    async fn close(&mut self) -> Result<(), McpError> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }
        self.stdin = None;
        self.stdout_reader = None;
        self.connected = false;
        self.cached_tools = None;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

impl StdioMcpClient {
    /// Mutable variant of `call_tool` that can actually perform IO.
    pub async fn call_tool_mut(
        &mut self,
        name: &str,
        args: Value,
    ) -> Result<McpCallResult, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }

        let params = serde_json::json!({
            "name": name,
            "arguments": args,
        });
        let result = timeout(
            self.timeouts.call_tool,
            self.send_request("tools/call", Some(params)),
        )
        .await
        .map_err(|_| McpError::Timeout)??;

        parse_call_result(&result)
    }
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
fn parse_call_result(result: &Value) -> Result<McpCallResult, McpError> {
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
// HTTP MCP client
// ---------------------------------------------------------------------------

/// HTTP transport variant for MCP.
#[cfg(feature = "mcp-http")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMcpTransport {
    Sse,
    StreamableHttp,
}

/// Minimal HTTP MCP client.
///
/// The streamable HTTP path sends JSON-RPC requests with HTTP POST. The SSE
/// variant is represented in the same client abstraction so Agent OS config can
/// resolve it uniformly; full bidirectional SSE session handling can be
/// extended behind this type without changing toolkit registration.
#[cfg(feature = "mcp-http")]
pub struct HttpMcpClient {
    transport: HttpMcpTransport,
    url: String,
    headers: BTreeMap<String, String>,
    http: reqwest::Client,
    connected: bool,
    request_id: AtomicU64,
    timeouts: McpTimeouts,
}

#[cfg(feature = "mcp-http")]
impl HttpMcpClient {
    pub fn new(
        transport: HttpMcpTransport,
        url: impl Into<String>,
        headers: BTreeMap<String, String>,
        timeouts: McpTimeouts,
    ) -> Self {
        Self {
            transport,
            url: url.into(),
            headers,
            http: reqwest::Client::new(),
            connected: false,
            request_id: AtomicU64::new(1),
            timeouts,
        }
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    async fn send_request(
        &self,
        method: &str,
        params: Option<Value>,
        request_timeout: Duration,
    ) -> Result<Value, McpError> {
        let id = self.next_id();
        let mut req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        });
        if let Some(p) = params {
            req["params"] = p;
        }

        let mut builder = self.http.post(&self.url).json(&req);
        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        let response = timeout(request_timeout, builder.send())
            .await
            .map_err(|_| McpError::Timeout)?
            .map_err(|e| McpError::Connection(e.to_string()))?;
        if !response.status().is_success() {
            return Err(McpError::Protocol(format!(
                "HTTP MCP request failed with status {}",
                response.status()
            )));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = timeout(request_timeout, response.text())
            .await
            .map_err(|_| McpError::Timeout)?
            .map_err(|e| McpError::Protocol(e.to_string()))?;
        let parsed = parse_http_mcp_response(&content_type, &body)?;
        if let Some(err) = parsed.get("error") {
            let msg = err["message"].as_str().unwrap_or("unknown error");
            return Err(McpError::Protocol(msg.to_string()));
        }
        Ok(parsed.get("result").cloned().unwrap_or(Value::Null))
    }

    async fn send_notification(&self, method: &str, params: Option<Value>) -> Result<(), McpError> {
        let mut req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        if let Some(p) = params {
            req["params"] = p;
        }

        let mut builder = self.http.post(&self.url).json(&req);
        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }
        let response = timeout(self.timeouts.connect, builder.send())
            .await
            .map_err(|_| McpError::Timeout)?
            .map_err(|e| McpError::Connection(e.to_string()))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(McpError::Protocol(format!(
                "HTTP MCP notification failed with status {}",
                response.status()
            )))
        }
    }
}

#[cfg(feature = "mcp-http")]
#[async_trait]
impl McpClient for HttpMcpClient {
    async fn connect(&mut self) -> Result<(), McpError> {
        if self.connected {
            return Err(McpError::AlreadyConnected);
        }
        if self.transport == HttpMcpTransport::Sse {
            tracing::debug!(url = %self.url, "initializing MCP SSE client via HTTP request path");
        }
        let init_params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "macaca",
                "version": "0.1.0"
            }
        });
        let _ = self
            .send_request("initialize", Some(init_params), self.timeouts.connect)
            .await?;
        self.send_notification("notifications/initialized", None)
            .await?;
        self.connected = true;
        Ok(())
    }

    async fn list_tools(&mut self) -> Result<Vec<McpToolDef>, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }
        let result = self
            .send_request("tools/list", None, self.timeouts.list_tools)
            .await?;
        let tools_value = result.get("tools").cloned().unwrap_or(Value::Array(vec![]));
        serde_json::from_value(tools_value).map_err(|e| McpError::Protocol(e.to_string()))
    }

    async fn call_tool(&mut self, name: &str, args: Value) -> Result<McpCallResult, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }
        let params = serde_json::json!({
            "name": name,
            "arguments": args,
        });
        let result = self
            .send_request("tools/call", Some(params), self.timeouts.call_tool)
            .await?;
        parse_call_result(&result)
    }

    async fn list_resources(&mut self) -> Result<Vec<McpResourceDef>, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }
        let result = self
            .send_request("resources/list", None, self.timeouts.call_tool)
            .await?;
        let resources = result
            .get("resources")
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        serde_json::from_value(resources).map_err(|e| McpError::Protocol(e.to_string()))
    }

    async fn list_resource_templates(&mut self) -> Result<Vec<McpResourceTemplateDef>, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }
        let result = self
            .send_request("resources/templates/list", None, self.timeouts.call_tool)
            .await?;
        let templates = result
            .get("resourceTemplates")
            .or_else(|| result.get("resource_templates"))
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        serde_json::from_value(templates).map_err(|e| McpError::Protocol(e.to_string()))
    }

    async fn read_resource(&mut self, uri: &str) -> Result<McpResourceRead, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }
        let result = self
            .send_request(
                "resources/read",
                Some(serde_json::json!({ "uri": uri })),
                self.timeouts.call_tool,
            )
            .await?;
        let mut contents: Vec<McpResourceRead> = serde_json::from_value(
            result
                .get("contents")
                .cloned()
                .unwrap_or_else(|| Value::Array(vec![])),
        )
        .map_err(|e| McpError::Protocol(e.to_string()))?;
        contents
            .pop()
            .ok_or_else(|| McpError::Protocol("resource read returned no contents".into()))
    }

    async fn close(&mut self) -> Result<(), McpError> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

#[cfg(feature = "mcp-http")]
fn parse_http_mcp_response(content_type: &str, body: &str) -> Result<Value, McpError> {
    if content_type.contains("text/event-stream") {
        let mut last_data = None;
        for line in body.lines() {
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            last_data = Some(data.to_string());
        }
        let data = last_data.ok_or_else(|| {
            McpError::Protocol("SSE MCP response did not include a data frame".to_string())
        })?;
        serde_json::from_str(&data).map_err(|e| McpError::Protocol(e.to_string()))
    } else {
        serde_json::from_str(body).map_err(|e| McpError::Protocol(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// McpToolHandler — bridges an MCP tool into the Toolkit system
// ---------------------------------------------------------------------------

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

struct McpClientResource {
    client: Arc<tokio::sync::RwLock<dyn McpClient>>,
    on_close: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl ToolkitResource for McpClientResource {
    fn close(self: Box<Self>) {
        let client = Arc::clone(&self.client);
        let on_close = self.on_close.clone();
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

// ---------------------------------------------------------------------------
// Toolkit integration
// ---------------------------------------------------------------------------

/// Register all tools from an MCP client into the given toolkit.
///
/// Calls `client.list_tools()` and creates an `McpToolHandler` for each one,
/// registering it under `group_name`.
pub async fn register_mcp_tools(
    toolkit: &mut Toolkit,
    client: Arc<tokio::sync::RwLock<dyn McpClient>>,
    group_name: &str,
) -> Result<(), McpError> {
    register_mcp_tools_with_options(toolkit, client, McpToolRegistrationOptions::new(group_name))
        .await
        .map(|_| ())
}

/// Register all tools from an MCP client with explicit registration options.
pub async fn register_mcp_tools_with_options(
    toolkit: &mut Toolkit,
    client: Arc<tokio::sync::RwLock<dyn McpClient>>,
    options: McpToolRegistrationOptions,
) -> Result<Vec<String>, McpError> {
    let tools = {
        let mut c = client.write().await;
        c.list_tools().await?
    };

    let mut registered_names = Vec::new();
    for tool_def in tools {
        let registered_name = match &options.conflict_policy {
            McpToolNameConflictPolicy::Raise => {
                if toolkit.get_tool(&tool_def.name).is_some() {
                    return Err(McpError::ToolNameCollision(tool_def.name.clone()));
                }
                tool_def.name.clone()
            }
            McpToolNameConflictPolicy::Skip => {
                if toolkit.get_tool(&tool_def.name).is_some() {
                    continue;
                }
                tool_def.name.clone()
            }
            McpToolNameConflictPolicy::Prefix(prefix) => {
                let candidate = format!("{prefix}{}", tool_def.name);
                if toolkit.get_tool(&candidate).is_some() {
                    return Err(McpError::ToolNameCollision(candidate));
                }
                candidate
            }
        };
        if options.disabled_tools.contains(&tool_def.name)
            || options.disabled_tools.contains(&registered_name)
        {
            continue;
        }
        let handler = McpToolHandler::with_registered_name(
            Arc::clone(&client),
            tool_def,
            registered_name.clone(),
        );
        toolkit.register(Box::new(handler), Some(&options.group_name));
        registered_names.push(registered_name);
    }
    toolkit.add_resource(Box::new(McpClientResource {
        client,
        on_close: options.on_close,
    }));

    Ok(registered_names)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};

    // -----------------------------------------------------------------------
    // MockMcpClient
    // -----------------------------------------------------------------------

    struct MockMcpClient {
        tools: Vec<McpToolDef>,
        connected: bool,
        call_responses: HashMap<String, McpCallResult>,
    }

    struct LocalEchoTool {
        name: String,
    }

    impl LocalEchoTool {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl ToolHandler for LocalEchoTool {
        async fn execute(&self, _args: Value) -> Result<ToolResponse, ToolError> {
            Ok(ToolResponse::text("local"))
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "local echo"
        }

        fn schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
    }

    impl MockMcpClient {
        fn new() -> Self {
            Self {
                tools: Vec::new(),
                connected: false,
                call_responses: HashMap::new(),
            }
        }

        fn with_tool(mut self, name: &str, description: &str) -> Self {
            self.tools.push(McpToolDef {
                name: name.to_string(),
                description: description.to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": { "type": "string" }
                    }
                }),
            });
            self
        }

        fn with_response(mut self, tool_name: &str, result: McpCallResult) -> Self {
            self.call_responses.insert(tool_name.to_string(), result);
            self
        }
    }

    #[async_trait]
    impl McpClient for MockMcpClient {
        async fn connect(&mut self) -> Result<(), McpError> {
            if self.connected {
                return Err(McpError::AlreadyConnected);
            }
            self.connected = true;
            Ok(())
        }

        async fn list_tools(&mut self) -> Result<Vec<McpToolDef>, McpError> {
            if !self.connected {
                return Err(McpError::NotConnected);
            }
            Ok(self.tools.clone())
        }

        async fn call_tool(&mut self, name: &str, _args: Value) -> Result<McpCallResult, McpError> {
            if !self.connected {
                return Err(McpError::NotConnected);
            }
            self.call_responses
                .get(name)
                .cloned()
                .ok_or_else(|| McpError::ToolNotFound(name.to_string()))
        }

        async fn close(&mut self) -> Result<(), McpError> {
            self.connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_mcp_tool_def_serde() {
        let def = McpToolDef {
            name: "read_file".to_string(),
            description: "Reads a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
        };

        let json = serde_json::to_string(&def).unwrap();
        let deser: McpToolDef = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.name, "read_file");
        assert_eq!(deser.description, "Reads a file");
        assert!(deser.input_schema["properties"]["path"]["type"] == "string");
    }

    #[test]
    fn test_mcp_tool_def_accepts_camel_case_schema() {
        let json = r#"{
            "name": "browser_navigate",
            "description": "Navigate",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string" }
                },
                "required": ["url"]
            }
        }"#;
        let def: McpToolDef = serde_json::from_str(json).unwrap();
        assert_eq!(def.name, "browser_navigate");
        assert_eq!(def.input_schema["properties"]["url"]["type"], "string");
    }

    #[tokio::test]
    async fn test_mock_client_connect_and_list() {
        let mut client = MockMcpClient::new()
            .with_tool("tool_a", "Tool A description")
            .with_tool("tool_b", "Tool B description");

        // Not connected yet.
        assert!(!client.is_connected());

        // Connect.
        client.connect().await.unwrap();
        assert!(client.is_connected());

        // List tools.
        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "tool_a");
        assert_eq!(tools[1].name, "tool_b");
    }

    #[tokio::test]
    async fn test_mock_client_call_tool() {
        let result = McpCallResult {
            content: vec![ContentBlock::Text(TextBlock {
                text: "hello from mcp".to_string(),
            })],
            is_error: false,
            metadata: None,
        };

        let mut client = MockMcpClient::new()
            .with_tool("greet", "Greets the user")
            .with_response("greet", result);

        client.connect().await.unwrap();

        let call_result = client
            .call_tool("greet", serde_json::json!({"name": "world"}))
            .await
            .unwrap();

        assert!(!call_result.is_error);
        assert_eq!(call_result.content.len(), 1);
        if let ContentBlock::Text(tb) = &call_result.content[0] {
            assert_eq!(tb.text, "hello from mcp");
        } else {
            panic!("expected TextBlock");
        }
    }

    #[tokio::test]
    async fn test_mock_client_not_connected() {
        let mut client = MockMcpClient::new().with_tool("t", "desc");

        // list_tools without connect should fail.
        let err = client
            .call_tool("t", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::NotConnected));
    }

    #[tokio::test]
    async fn test_mcp_tool_handler_as_tool() {
        let result = McpCallResult {
            content: vec![ContentBlock::Text(TextBlock {
                text: "result data".to_string(),
            })],
            is_error: false,
            metadata: Some(serde_json::json!({"tokens": 42})),
        };

        let mut mock = MockMcpClient::new()
            .with_tool("my_tool", "Does something")
            .with_response("my_tool", result);
        mock.connect().await.unwrap();

        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let tool_def = McpToolDef {
            name: "my_tool".to_string(),
            description: "Does something".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        };

        let handler = McpToolHandler::new(client, tool_def);

        // Verify ToolHandler trait methods.
        assert_eq!(handler.name(), "my_tool");
        assert_eq!(handler.description(), "Does something");
        assert_eq!(handler.schema(), serde_json::json!({"type": "object"}));

        // Execute.
        let resp = handler.execute(serde_json::json!({})).await.unwrap();
        assert_eq!(resp.content.len(), 1);
        if let ContentBlock::Text(tb) = &resp.content[0] {
            assert_eq!(tb.text, "result data");
        } else {
            panic!("expected TextBlock");
        }
        assert_eq!(resp.metadata, Some(serde_json::json!({"tokens": 42})));
    }

    #[tokio::test]
    async fn test_register_mcp_tools() {
        let result_a = McpCallResult {
            content: vec![ContentBlock::Text(TextBlock {
                text: "a_result".to_string(),
            })],
            is_error: false,
            metadata: None,
        };
        let result_b = McpCallResult {
            content: vec![ContentBlock::Text(TextBlock {
                text: "b_result".to_string(),
            })],
            is_error: false,
            metadata: None,
        };

        let mut mock = MockMcpClient::new()
            .with_tool("mcp_a", "Tool A")
            .with_tool("mcp_b", "Tool B")
            .with_response("mcp_a", result_a)
            .with_response("mcp_b", result_b);
        mock.connect().await.unwrap();

        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let mut toolkit = Toolkit::new();
        register_mcp_tools(&mut toolkit, client, "mcp_group")
            .await
            .unwrap();

        // Both tools should be registered.
        assert_eq!(toolkit.tool_count(), 2);
        assert!(toolkit.get_tool("mcp_a").is_some());
        assert!(toolkit.get_tool("mcp_b").is_some());

        // Call one of them.
        let resp = toolkit
            .call_tool("mcp_a", serde_json::json!({}))
            .await
            .unwrap();
        if let ContentBlock::Text(tb) = &resp.content[0] {
            assert_eq!(tb.text, "a_result");
        } else {
            panic!("expected TextBlock");
        }
    }

    #[tokio::test]
    async fn test_call_nonexistent_mcp_tool() {
        let mut mock = MockMcpClient::new().with_tool("exists", "Exists");
        mock.connect().await.unwrap();

        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let mut toolkit = Toolkit::new();
        register_mcp_tools(&mut toolkit, client, "mcp")
            .await
            .unwrap();

        // Calling a tool that was never registered should fail.
        let err = toolkit
            .call_tool("does_not_exist", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));

        // Calling a registered MCP tool whose backend doesn't have a response
        // should also fail (ToolNotFound from mock → ExecutionFailed).
        let err = toolkit
            .call_tool("exists", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
    }

    #[test]
    fn test_parse_call_result_basic() {
        let result = serde_json::json!({
            "content": [
                {"type": "text", "text": "hello"},
                {"type": "text", "text": " world"}
            ],
            "isError": false
        });
        let parsed = parse_call_result(&result).unwrap();
        assert!(!parsed.is_error);
        assert_eq!(parsed.content.len(), 2);
    }

    #[test]
    fn test_parse_call_result_error() {
        let result = serde_json::json!({
            "content": [{"type": "text", "text": "something went wrong"}],
            "isError": true
        });
        let parsed = parse_call_result(&result).unwrap();
        assert!(parsed.is_error);
    }

    #[test]
    fn test_parse_call_result_multimodal_and_resource_fallback() {
        let result = serde_json::json!({
            "content": [
                {"type": "image", "data": "abc", "mimeType": "image/png"},
                {"type": "audio", "data": "def", "mimeType": "audio/wav"},
                {"type": "resource", "resource": {"uri": "file://tmp.txt", "text": "resource text"}},
                {"type": "unknown", "value": 1}
            ],
            "isError": false,
            "_meta": {"server": "test"}
        });
        let parsed = parse_call_result(&result).unwrap();
        assert!(!parsed.is_error);
        assert_eq!(parsed.content.len(), 4);
        assert!(matches!(parsed.content[0], ContentBlock::Image(_)));
        assert!(matches!(parsed.content[1], ContentBlock::Audio(_)));
        match &parsed.content[2] {
            ContentBlock::Text(text) => assert_eq!(text.text, "resource text"),
            _ => panic!("expected text resource fallback"),
        }
        match &parsed.content[3] {
            ContentBlock::Text(text) => assert!(text.text.contains("\"unknown\"")),
            _ => panic!("expected json text fallback"),
        }
        assert_eq!(parsed.metadata, Some(serde_json::json!({"server": "test"})));
    }

    #[tokio::test]
    async fn test_register_mcp_tools_raises_on_collision() {
        let mut mock = MockMcpClient::new().with_tool("exists", "MCP Exists");
        mock.connect().await.unwrap();
        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(LocalEchoTool::new("exists")), None);

        let err = register_mcp_tools_with_options(
            &mut toolkit,
            client,
            McpToolRegistrationOptions::new("mcp"),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, McpError::ToolNameCollision(_)));
    }

    #[tokio::test]
    async fn test_register_mcp_tools_prefixes_collision() {
        let result = McpCallResult {
            content: vec![ContentBlock::Text(TextBlock {
                text: "mcp result".to_string(),
            })],
            is_error: false,
            metadata: None,
        };
        let mut mock = MockMcpClient::new()
            .with_tool("exists", "MCP Exists")
            .with_response("exists", result);
        mock.connect().await.unwrap();
        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let mut toolkit = Toolkit::new();
        toolkit.register(Box::new(LocalEchoTool::new("exists")), None);

        register_mcp_tools_with_options(
            &mut toolkit,
            client,
            McpToolRegistrationOptions {
                group_name: "mcp".to_string(),
                conflict_policy: McpToolNameConflictPolicy::Prefix("mcp_".to_string()),
                disabled_tools: HashSet::new(),
                on_close: None,
            },
        )
        .await
        .unwrap();

        assert!(toolkit.get_tool("exists").is_some());
        assert!(toolkit.get_tool("mcp_exists").is_some());
        let resp = toolkit
            .call_tool("mcp_exists", serde_json::json!({}))
            .await
            .unwrap();
        match &resp.content[0] {
            ContentBlock::Text(text) => assert_eq!(text.text, "mcp result"),
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn test_register_mcp_tools_skips_disabled_tools() {
        let mut mock = MockMcpClient::new()
            .with_tool("allowed", "Allowed")
            .with_tool("blocked", "Blocked");
        mock.connect().await.unwrap();
        let client: Arc<tokio::sync::RwLock<dyn McpClient>> =
            Arc::new(tokio::sync::RwLock::new(mock));

        let mut disabled_tools = HashSet::new();
        disabled_tools.insert("blocked".to_string());
        let mut toolkit = Toolkit::new();
        let registered = register_mcp_tools_with_options(
            &mut toolkit,
            client,
            McpToolRegistrationOptions {
                group_name: "mcp".to_string(),
                conflict_policy: McpToolNameConflictPolicy::Raise,
                disabled_tools,
                on_close: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(registered, vec!["allowed"]);
        assert!(toolkit.get_tool("allowed").is_some());
        assert!(toolkit.get_tool("blocked").is_none());
    }

    #[cfg(feature = "mcp-http")]
    #[test]
    fn test_parse_streamable_http_json_response() {
        let parsed = parse_http_mcp_response(
            "application/json",
            r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#,
        )
        .unwrap();
        assert!(parsed["result"]["tools"].as_array().unwrap().is_empty());
    }

    #[cfg(feature = "mcp-http")]
    #[test]
    fn test_parse_sse_json_rpc_data_frame() {
        let parsed = parse_http_mcp_response(
            "text/event-stream",
            "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}\n\n",
        )
        .unwrap();
        assert!(parsed["result"]["tools"].as_array().unwrap().is_empty());
    }

    #[cfg(feature = "mcp-http")]
    async fn spawn_http_mcp_test_server(content_type: &'static str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            for _ in 0..3 {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                tokio::spawn(async move {
                    let mut buf = vec![0_u8; 8192];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let body = if req.contains("\"method\":\"tools/list\"") {
                        r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"web_search","description":"Search","inputSchema":{"type":"object"}}]}}"#
                    } else if req.contains("\"method\":\"initialize\"") {
                        r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{}}}"#
                    } else {
                        r#"{"jsonrpc":"2.0","result":{}}"#
                    };
                    let response_body = if content_type == "text/event-stream" {
                        format!("event: message\ndata: {body}\n\n")
                    } else {
                        body.to_string()
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        response_body.len(),
                        response_body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        });
        format!("http://{addr}/mcp")
    }

    #[cfg(feature = "mcp-http")]
    #[tokio::test]
    async fn test_streamable_http_client_lists_tools() {
        let url = spawn_http_mcp_test_server("application/json").await;
        let mut client = client_from_transport(
            McpTransportConfig::StreamableHttp {
                url,
                headers: BTreeMap::new(),
            },
            McpTimeouts::default(),
        )
        .unwrap();
        client.connect().await.unwrap();
        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools[0].name, "web_search");
    }

    #[cfg(feature = "mcp-http")]
    #[tokio::test]
    async fn test_sse_client_lists_tools_from_event_stream_response() {
        let url = spawn_http_mcp_test_server("text/event-stream").await;
        let mut client = client_from_transport(
            McpTransportConfig::Sse {
                url,
                headers: BTreeMap::new(),
            },
            McpTimeouts::default(),
        )
        .unwrap();
        client.connect().await.unwrap();
        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools[0].name, "web_search");
    }

    #[tokio::test]
    async fn test_mock_client_already_connected() {
        let mut client = MockMcpClient::new();
        client.connect().await.unwrap();

        let err = client.connect().await.unwrap_err();
        assert!(matches!(err, McpError::AlreadyConnected));
    }

    #[tokio::test]
    async fn test_mock_client_close() {
        let mut client = MockMcpClient::new();
        client.connect().await.unwrap();
        assert!(client.is_connected());

        client.close().await.unwrap();
        assert!(!client.is_connected());
    }
}
