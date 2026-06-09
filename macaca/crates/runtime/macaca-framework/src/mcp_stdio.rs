// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Stdio transport adapter for framework-level MCP clients.
//!
//! This module keeps child-process JSON-RPC mechanics out of the public MCP
//! contract module. The type remains framework-local and provider-neutral: the
//! runtime host decides whether this built-in adapter, a plugin adapter, a
//! remote adapter, a mock, or an unavailable adapter is wired into an OS
//! service boundary.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::time::timeout;
use tracing::{debug, info};

use crate::mcp::{
    parse_call_result, McpCallResult, McpClient, McpError, McpResourceDef, McpResourceRead,
    McpResourceTemplateDef, McpTimeouts, McpToolDef,
};

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
    /// [`connect`](McpClient::connect) is called.
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

    /// Send a JSON-RPC request with an id and return the parsed result.
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

    /// Send a JSON-RPC notification without waiting for a response.
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
        // MCP servers may emit notifications while a request is in flight. The
        // client keeps reading until it finds a JSON-RPC response carrying an id.
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

        debug!("starting MCP stdio client process");
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

        self.send_notification("notifications/initialized", None)
            .await?;

        self.connected = true;
        info!("MCP stdio client initialized");
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
        debug!(tool_count = tools.len(), "MCP stdio client listed tools");
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
        info!("MCP stdio client closed");
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
