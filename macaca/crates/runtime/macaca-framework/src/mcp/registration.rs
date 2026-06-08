//! Bulk MCP tool registration into a framework [`Toolkit`] (**Bridge** pattern).
//!
//! Lists tools from a connected client, applies collision/disabled policies, and
//! registers [`McpToolHandler`] adapters under a named group.

use std::sync::Arc;

use crate::tool::Toolkit;

use super::adapter::{McpClientResource, McpToolHandler};
use super::core::McpClient;
use super::error::McpError;
use super::types::{McpToolNameConflictPolicy, McpToolRegistrationOptions};

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
    tracing::debug!(
        target = "macaca_framework::mcp::registration",
        group_name = %options.group_name,
        conflict_policy = ?options.conflict_policy,
        disabled_count = options.disabled_tools.len(),
        "registering MCP tools into toolkit"
    );

    let tools = {
        let mut c = client.write().await;
        c.list_tools().await?
    };

    let mut registered_names = Vec::new();
    for tool_def in tools {
        let registered_name = match &options.conflict_policy {
            McpToolNameConflictPolicy::Raise => {
                if toolkit.get_tool(&tool_def.name).is_some() {
                    tracing::warn!(
                        target = "macaca_framework::mcp::registration",
                        tool_name = %tool_def.name,
                        "MCP tool name collision — raising"
                    );
                    return Err(McpError::ToolNameCollision(tool_def.name.clone()));
                }
                tool_def.name.clone()
            }
            McpToolNameConflictPolicy::Skip => {
                if toolkit.get_tool(&tool_def.name).is_some() {
                    tracing::debug!(
                        target = "macaca_framework::mcp::registration",
                        tool_name = %tool_def.name,
                        "skipping colliding MCP tool"
                    );
                    continue;
                }
                tool_def.name.clone()
            }
            McpToolNameConflictPolicy::Prefix(prefix) => {
                let candidate = format!("{prefix}{}", tool_def.name);
                if toolkit.get_tool(&candidate).is_some() {
                    tracing::warn!(
                        target = "macaca_framework::mcp::registration",
                        candidate = %candidate,
                        "prefixed MCP tool name collision — raising"
                    );
                    return Err(McpError::ToolNameCollision(candidate));
                }
                candidate
            }
        };
        if options.disabled_tools.contains(&tool_def.name)
            || options.disabled_tools.contains(&registered_name)
        {
            tracing::debug!(
                target = "macaca_framework::mcp::registration",
                tool_name = %tool_def.name,
                "skipping disabled MCP tool"
            );
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

    tracing::debug!(
        target = "macaca_framework::mcp::registration",
        registered_count = registered_names.len(),
        group_name = %options.group_name,
        "MCP tool registration complete"
    );

    toolkit.add_resource(Box::new(McpClientResource {
        client,
        on_close: options.on_close,
    }));

    Ok(registered_names)
}
