//! Toolkit registration for MCP tools (Abstract Factory + Observer).
//!
//! Connects to MCP servers, registers tools into a framework [`Toolkit`], and
//! wires lease release callbacks for lifecycle cleanup.

use std::sync::Arc;

use macaca_framework::mcp::{
    register_mcp_tools_with_options, McpClient, McpTimeouts, McpToolNameConflictPolicy,
    McpToolRegistrationOptions,
};
use macaca_framework::tool::Toolkit;
use tokio::sync::RwLock;

use crate::transport::{bridge_for_config, McpTransport};

use super::client::ClientBox;
use super::helpers::{missing_required_bin, status_for_definition};
use super::manager::McpRuntimeManager;
use super::types::{
    McpRuntimeContext, McpRuntimeStatus, McpRuntimeStatusState, McpServerDefinition, McpToolPolicy,
};

impl McpRuntimeManager {
    pub(crate) async fn register_definition_tools(
        self: &Arc<Self>,
        toolkit: &mut Toolkit,
        definition: &McpServerDefinition,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
        on_closed: Option<Arc<dyn Fn(McpRuntimeStatus) + Send + Sync>>,
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
        if let Err(error) = client.connect().await {
            return status_for_definition(
                definition,
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some(error.to_string()),
            );
        }

        let lease = self.acquire_lease(definition, context).await;
        let runtime = Arc::clone(self);
        let closed_definition = definition.clone();
        let close_callback = on_closed.map(|on_closed| {
            let runtime = Arc::clone(&runtime);
            let lease = lease.clone();
            Arc::new(move || {
                let runtime = Arc::clone(&runtime);
                let lease = lease.clone();
                let closed_definition = closed_definition.clone();
                let on_closed = Arc::clone(&on_closed);
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        let status = runtime.release_lease(lease).await.unwrap_or_else(|| {
                            status_for_definition(
                                &closed_definition,
                                McpRuntimeStatusState::Ready,
                                Vec::new(),
                                None,
                            )
                        });
                        on_closed(status);
                    });
                }
            }) as Arc<dyn Fn() + Send + Sync>
        });

        let client: Arc<RwLock<dyn McpClient>> = Arc::new(RwLock::new(ClientBox::new(client)));
        let options = McpToolRegistrationOptions {
            group_name: format!("mcp:{}", definition.id),
            conflict_policy: definition
                .tool_prefix
                .clone()
                .map(McpToolNameConflictPolicy::Prefix)
                .unwrap_or(McpToolNameConflictPolicy::Raise),
            disabled_tools: policy.deny_tools.clone(),
            on_close: close_callback,
        };
        let result = register_mcp_tools_with_options(toolkit, Arc::clone(&client), options).await;
        match result {
            Ok(registered_tools) => {
                let exposed_tools = registered_tools
                    .into_iter()
                    .filter(|name| policy.allows_tool(name))
                    .collect();
                status_for_definition(
                    definition,
                    McpRuntimeStatusState::Ready,
                    exposed_tools,
                    None,
                )
            }
            Err(error) => status_for_definition(
                definition,
                McpRuntimeStatusState::Failed,
                Vec::new(),
                Some(error.to_string()),
            ),
        }
    }
}
