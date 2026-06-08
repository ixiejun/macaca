//! HTTP MCP client — SSE and streamable HTTP JSON-RPC transports (**Adapter**).
//!
//! Provides a uniform [`McpClient`] implementation for remote MCP servers. The SSE
//! variant reuses the POST request path today; full bidirectional SSE sessions can
//! extend behind [`HttpMcpTransport`] without changing toolkit registration.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio::time::timeout;

use super::core::McpClient;
use super::error::McpError;
use super::parse::parse_call_result;
use super::types::{
    McpCallResult, McpResourceDef, McpResourceRead, McpResourceTemplateDef, McpTimeouts,
    McpToolDef,
};

/// HTTP transport variant for MCP.
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
pub struct HttpMcpClient {
    transport: HttpMcpTransport,
    url: String,
    headers: BTreeMap<String, String>,
    http: reqwest::Client,
    connected: bool,
    request_id: AtomicU64,
    timeouts: McpTimeouts,
}

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

        tracing::debug!(
            target = "macaca_framework::mcp::http",
            method = method,
            request_id = id,
            transport = ?self.transport,
            "sending HTTP MCP JSON-RPC request"
        );

        let mut builder = self.http.post(&self.url).json(&req);
        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        let response = timeout(request_timeout, builder.send())
            .await
            .map_err(|_| McpError::Timeout)?
            .map_err(|e| McpError::Connection(e.to_string()))?;
        if !response.status().is_success() {
            tracing::warn!(
                target = "macaca_framework::mcp::http",
                status = %response.status(),
                "HTTP MCP request failed"
            );
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
            tracing::warn!(
                target = "macaca_framework::mcp::http",
                error = msg,
                "HTTP MCP JSON-RPC error response"
            );
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

#[async_trait]
impl McpClient for HttpMcpClient {
    async fn connect(&mut self) -> Result<(), McpError> {
        if self.connected {
            return Err(McpError::AlreadyConnected);
        }
        if self.transport == HttpMcpTransport::Sse {
            tracing::debug!(
                target = "macaca_framework::mcp::http",
                url = %self.url,
                "initializing MCP SSE client via HTTP request path"
            );
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
        tracing::debug!(
            target = "macaca_framework::mcp::http",
            transport = ?self.transport,
            "HTTP MCP session initialized"
        );
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
        let tools: Vec<McpToolDef> =
            serde_json::from_value(tools_value).map_err(|e| McpError::Protocol(e.to_string()))?;
        tracing::debug!(
            target = "macaca_framework::mcp::http",
            tool_count = tools.len(),
            "listed MCP tools via HTTP"
        );
        Ok(tools)
    }

    async fn call_tool(&mut self, name: &str, args: Value) -> Result<McpCallResult, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected);
        }
        tracing::debug!(
            target = "macaca_framework::mcp::http",
            tool = name,
            "calling MCP tool via HTTP transport"
        );
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
        tracing::debug!(
            target = "macaca_framework::mcp::http",
            "closing HTTP MCP session"
        );
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Decode an HTTP MCP response body (JSON or SSE `data:` frames) into JSON-RPC.
pub(crate) fn parse_http_mcp_response(content_type: &str, body: &str) -> Result<Value, McpError> {
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
