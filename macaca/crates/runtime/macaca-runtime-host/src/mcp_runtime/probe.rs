//! MCP connectivity probe and descriptor catalog refresh.
//!
//! Probes remote MCP servers without mutating the invocation registry.

use std::time::Duration;

use macaca_framework::mcp::{McpTimeouts, McpTransportConfig};
use tokio::time::timeout;

use crate::transport::{bridge_for_config, McpTransport};

use super::helpers::{
    flatten_timeout_result, missing_required_bin, prefixed_tool_name, status_for_definition,
};
use super::types::{
    McpRuntimeStatus, McpRuntimeStatusState, McpServerDefinition, McpToolPolicy,
};

pub async fn probe_definition_statuses(
    definitions: Vec<McpServerDefinition>,
    policy: &McpToolPolicy,
) -> Vec<McpRuntimeStatus> {
    let mut statuses = Vec::new();
    for definition in definitions {
        if !definition.enabled || !policy.allows_server(&definition.id) {
            statuses.push(status_for_definition(
                &definition,
                McpRuntimeStatusState::Disabled,
                Vec::new(),
                None,
            ));
            continue;
        }
        statuses.push(probe_definition(&definition, policy).await);
    }
    statuses
}

async fn probe_definition(
    definition: &McpServerDefinition,
    policy: &McpToolPolicy,
) -> McpRuntimeStatus {
    if let Some(missing) = missing_required_bin(definition) {
        return status_for_definition(
            definition,
            McpRuntimeStatusState::DependencyMissing,
            Vec::new(),
            Some(format!("missing dependency: {missing}")),
        );
    }

    let transport = bridge_for_config(definition.transport.clone());
    let mut client = match transport.create_client(McpTimeouts::default()) {
        Ok(client) => client,
        Err(error) => {
            return status_for_definition(
                definition,
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some(error.to_string()),
            )
        }
    };
    let connected = timeout(Duration::from_secs(15), client.connect()).await;
    if let Err(error) = flatten_timeout_result(connected) {
        return status_for_definition(
            definition,
            McpRuntimeStatusState::Failed,
            Vec::new(),
            Some(error),
        );
    }

    let tools = match timeout(Duration::from_secs(15), client.list_tools()).await {
        Ok(Ok(tools)) => tools,
        Ok(Err(error)) => {
            let _ = client.close().await;
            return status_for_definition(
                definition,
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some(error.to_string()),
            );
        }
        Err(_) => {
            let _ = client.close().await;
            return status_for_definition(
                definition,
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some("tools_list_timeout".to_string()),
            );
        }
    };
    let _ = client.close().await;

    let exposed_tools = tools
        .into_iter()
        .filter(|tool| policy.allows_tool(&tool.name))
        .map(|tool| prefixed_tool_name(definition, &tool.name))
        .filter(|tool| policy.allows_tool(tool))
        .collect();
    status_for_definition(
        definition,
        McpRuntimeStatusState::Ready,
        exposed_tools,
        None,
    )
}
