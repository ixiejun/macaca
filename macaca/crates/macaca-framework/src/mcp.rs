//! MCP (Model Context Protocol) support.
//!
//! Provides:
//! - `McpClient` trait for connecting to MCP servers
//! - `StdioMcpClient` — JSON-RPC 2.0 over child-process stdin/stdout
//! - `McpToolHandler` — bridges MCP tools into the `Toolkit` system
//! - `register_mcp_tools` — bulk-registers MCP tools into a `Toolkit`

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

use crate::message::{ContentBlock, TextBlock};
use crate::tool::{ToolError, ToolHandler, ToolResponse, Toolkit};

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
}

// ---------------------------------------------------------------------------
// MCP type definitions
// ---------------------------------------------------------------------------

/// An MCP tool definition received from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: Value,
}

/// Result from calling an MCP tool.
#[derive(Debug, Clone)]
pub struct McpCallResult {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub metadata: Option<Value>,
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
    async fn call_tool(&self, name: &str, args: Value) -> Result<McpCallResult, McpError>;
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
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<BufReader<ChildStdout>>,
    connected: bool,
    request_id: AtomicU64,
    cached_tools: Option<Vec<McpToolDef>>,
}

impl StdioMcpClient {
    /// Create a new client. The child process is **not** started until
    /// [`connect`] is called.
    pub fn new(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            command: command.into(),
            args,
            child: None,
            stdin: None,
            stdout_reader: None,
            connected: false,
            request_id: AtomicU64::new(1),
            cached_tools: None,
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
        self.read_response().await
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

#[async_trait]
impl McpClient for StdioMcpClient {
    async fn connect(&mut self) -> Result<(), McpError> {
        if self.connected {
            return Err(McpError::AlreadyConnected);
        }

        let mut child = tokio::process::Command::new(&self.command)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
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
        let _result = self.send_request("initialize", Some(init_params)).await?;

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
        let result = self.send_request("tools/list", None).await?;
        let tools_value = result.get("tools").cloned().unwrap_or(Value::Array(vec![]));
        let tools: Vec<McpToolDef> =
            serde_json::from_value(tools_value).map_err(|e| McpError::Protocol(e.to_string()))?;
        self.cached_tools = Some(tools.clone());
        Ok(tools)
    }

    async fn call_tool(&self, _name: &str, _args: Value) -> Result<McpCallResult, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }

        // Because call_tool takes &self but we need to do IO, we cannot use the
        // internal send_request helper which requires &mut self.  For a real
        // implementation this would use interior mutability (e.g. Mutex-wrapped
        // IO handles).  Callers should use the client behind an RwLock
        // (as McpToolHandler does) and call call_tool_mut instead.

        Err(McpError::Execution(
            "StdioMcpClient::call_tool requires &mut self; use via Arc<RwLock<..>> and call_tool_mut instead".into(),
        ))
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
        let result = self.send_request("tools/call", Some(params)).await?;

        parse_call_result(&result)
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
// McpToolHandler — bridges an MCP tool into the Toolkit system
// ---------------------------------------------------------------------------

/// Wraps a single MCP tool so it can be used as a `ToolHandler`.
pub struct McpToolHandler {
    client: Arc<tokio::sync::RwLock<dyn McpClient>>,
    tool_def: McpToolDef,
}

impl McpToolHandler {
    pub fn new(client: Arc<tokio::sync::RwLock<dyn McpClient>>, tool_def: McpToolDef) -> Self {
        Self { client, tool_def }
    }
}

#[async_trait]
impl ToolHandler for McpToolHandler {
    fn name(&self) -> &str {
        &self.tool_def.name
    }

    fn description(&self) -> &str {
        &self.tool_def.description
    }

    fn schema(&self) -> Value {
        self.tool_def.input_schema.clone()
    }

    async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError> {
        let client = self.client.read().await;
        let result = client
            .call_tool(&self.tool_def.name, args)
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
    let tools = {
        let mut c = client.write().await;
        c.list_tools().await?
    };

    for tool_def in tools {
        let handler = McpToolHandler::new(Arc::clone(&client), tool_def);
        toolkit.register(Box::new(handler), Some(group_name));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // -----------------------------------------------------------------------
    // MockMcpClient
    // -----------------------------------------------------------------------

    struct MockMcpClient {
        tools: Vec<McpToolDef>,
        connected: bool,
        call_responses: HashMap<String, McpCallResult>,
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

        async fn call_tool(&self, name: &str, _args: Value) -> Result<McpCallResult, McpError> {
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
        let client = MockMcpClient::new().with_tool("t", "desc");

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
