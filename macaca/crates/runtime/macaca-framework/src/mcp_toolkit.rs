// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Toolkit bridge for MCP tools.
//!
//! This bridge is intentionally small: it adapts MCP tool descriptors into the
//! framework `ToolHandler` trait and leaves policy, entitlement, resource
//! leases, metering, and audit decorators to runtime-host service boundaries.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, info};

use crate::mcp::{
    McpClient, McpError, McpToolDef, McpToolNameConflictPolicy, McpToolRegistrationOptions,
};
use crate::tool::{ToolError, ToolHandler, ToolResponse, Toolkit, ToolkitResource};

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
        debug!(tool = %self.registered_name, "executing MCP toolkit bridge tool");
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

    info!(
        tool_count = registered_names.len(),
        group = %options.group_name,
        "registered MCP tools into toolkit"
    );
    Ok(registered_names)
}
