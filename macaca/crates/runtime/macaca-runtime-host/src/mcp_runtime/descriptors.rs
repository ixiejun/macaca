//! Capability tool descriptor construction from MCP tool definitions.
//!
//! Maps framework MCP tool defs into service-owned [`CapabilityToolDescriptor`] values
//! with routing metadata for later `mcp.tool.invoke` calls.

use macaca_framework::mcp::McpTimeouts;
use macaca_proto::{
    CapabilityToolDescriptor, CapabilityToolOriginKind, MCP_DESCRIPTOR_BACKEND_TOOL_NAME,
    MCP_DESCRIPTOR_DEFINITION_SOURCE, MCP_DESCRIPTOR_LIFECYCLE_SCOPE, MCP_SERVICE_ID,
};
use tokio::time::timeout;

use crate::transport::{bridge_for_config, McpTransport};

use super::helpers::{
    lifecycle_scope_name, missing_required_bin, prefixed_tool_name,
    resource_scope_for_lifecycle,
};
use super::types::{McpServerDefinition, McpToolPolicy};

/// Build capability descriptors for one MCP server definition by probing its tool catalog.
pub(crate) async fn descriptors_for_definition(
    definition: &McpServerDefinition,
    policy: &McpToolPolicy,
) -> Vec<Result<CapabilityToolDescriptor, String>> {
    if !definition.enabled || !policy.allows_server(&definition.id) {
        return Vec::new();
    }
    if let Some(missing) = missing_required_bin(definition) {
        tracing::warn!(
            server_id = %definition.id,
            missing_dependency = %missing,
            "mcp descriptor index skipped unavailable server"
        );
        return vec![Err(format!("missing dependency: {missing}"))];
    }

    let transport = bridge_for_config(definition.transport.clone());
    let mut client = match transport.create_client(McpTimeouts::default()) {
        Ok(client) => client,
        Err(error) => return vec![Err(error.to_string())],
    };
    if let Err(error) = timeout(McpTimeouts::default().connect, client.connect())
        .await
        .map_err(|_| "connect_timeout".to_string())
        .and_then(|result| result.map_err(|error| error.to_string()))
    {
        let _ = client.close().await;
        return vec![Err(error)];
    }
    let tools = match timeout(McpTimeouts::default().list_tools, client.list_tools()).await {
        Ok(Ok(tools)) => tools,
        Ok(Err(error)) => {
            let _ = client.close().await;
            return vec![Err(error.to_string())];
        }
        Err(_) => {
            let _ = client.close().await;
            return vec![Err("tools_list_timeout".to_string())];
        }
    };
    let _ = client.close().await;

    tools
        .into_iter()
        .filter(|tool| policy.allows_tool(&tool.name))
        .filter_map(|tool| {
            let visible_name = prefixed_tool_name(definition, &tool.name);
            policy
                .allows_tool(&visible_name)
                .then_some((tool, visible_name))
        })
        .map(|(tool, visible_name)| descriptor_from_tool(definition, tool, visible_name))
        .collect()
}

pub(crate) fn descriptor_from_tool(
    definition: &McpServerDefinition,
    tool: macaca_framework::mcp::McpToolDef,
    visible_name: String,
) -> Result<CapabilityToolDescriptor, String> {
    let mut descriptor = CapabilityToolDescriptor::new(
        MCP_SERVICE_ID,
        definition.id.clone(),
        format!("mcp.tool.{}.{}", definition.id, tool.name),
        visible_name,
        tool.description,
        tool.input_schema,
        CapabilityToolOriginKind::Mcp,
    )
    .map_err(|error| error.to_string())?
    .with_display(Some(tool.name.clone()), definition.tool_prefix.clone())
    .with_policy_hints(
        vec!["mcp.tool.invoke".into()],
        vec![resource_scope_for_lifecycle(&definition.lifecycle)],
    );
    descriptor
        .metadata
        .insert(MCP_DESCRIPTOR_BACKEND_TOOL_NAME.into(), tool.name);
    descriptor.metadata.insert(
        MCP_DESCRIPTOR_LIFECYCLE_SCOPE.into(),
        lifecycle_scope_name(&definition.lifecycle).into(),
    );
    descriptor.metadata.insert(
        MCP_DESCRIPTOR_DEFINITION_SOURCE.into(),
        format!("{:?}", definition.source),
    );
    Ok(descriptor)
}
