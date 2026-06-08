//! Test doubles for MCP client and toolkit contract tests.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{ToolError, ToolHandler, ToolResponse};

use super::super::core::McpClient;
use super::super::error::McpError;
use super::super::types::{McpCallResult, McpToolDef};

/// In-memory MCP client stub for unit tests.
pub(crate) struct MockMcpClient {
    pub tools: Vec<McpToolDef>,
    pub connected: bool,
    pub call_responses: HashMap<String, McpCallResult>,
}

/// Local toolkit tool used to simulate name collisions during registration tests.
pub(crate) struct LocalEchoTool {
    name: String,
}

impl LocalEchoTool {
    pub fn new(name: &str) -> Self {
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
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            connected: false,
            call_responses: HashMap::new(),
        }
    }

    pub fn with_tool(mut self, name: &str, description: &str) -> Self {
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

    pub fn with_response(mut self, tool_name: &str, result: McpCallResult) -> Self {
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
