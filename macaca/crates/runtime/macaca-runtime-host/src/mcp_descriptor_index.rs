//! Service-owned MCP tool descriptor routing index.
//!
//! The index is the Catalog side of the service-backed MCP invocation design.
//! Shells and framework adapters receive model-visible descriptors, but the
//! service keeps the authoritative mapping from that visible name to the
//! backend MCP server and backend tool.  This prevents production invocation
//! paths from reconstructing routing by parsing names or by carrying a
//! host-local MCP client inside a presentation adapter.

use std::collections::BTreeMap;

use macaca_proto::{
    CapabilityToolDescriptor, CapabilityToolResourceScope, MCP_DESCRIPTOR_BACKEND_TOOL_NAME,
    MCP_DESCRIPTOR_DEFINITION_SOURCE, MCP_DESCRIPTOR_LIFECYCLE_SCOPE, MCP_SERVICE_ID,
};
use tokio::sync::RwLock;

use crate::mcp_runtime::McpLifecycleScope;

/// Immutable route extracted from one sanitized MCP tool descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct McpToolDescriptorRoute {
    pub server_id: String,
    pub visible_tool_name: String,
    pub backend_tool_name: String,
    pub lifecycle: McpLifecycleScope,
    pub resource_scope: CapabilityToolResourceScope,
    pub conflict_namespace: Option<String>,
    pub definition_source: Option<String>,
}

impl McpToolDescriptorRoute {
    /// Build an invocation route from descriptor metadata owned by `service.mcp`.
    ///
    /// The conversion is intentionally strict.  If metadata is missing or
    /// malformed, the descriptor is not accepted into the service routing
    /// index, which makes invocation fail closed before any MCP side effect.
    pub fn from_descriptor(descriptor: &CapabilityToolDescriptor) -> Result<Self, String> {
        if descriptor.service_id != MCP_SERVICE_ID {
            return Err(format!(
                "descriptor '{}' is owned by service '{}', not service.mcp",
                descriptor.tool_name, descriptor.service_id
            ));
        }
        let backend_tool_name = descriptor
            .metadata
            .get(MCP_DESCRIPTOR_BACKEND_TOOL_NAME)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "MCP descriptor '{}' is missing backend tool metadata",
                    descriptor.tool_name
                )
            })?;
        let lifecycle = descriptor
            .metadata
            .get(MCP_DESCRIPTOR_LIFECYCLE_SCOPE)
            .map(|value| lifecycle_from_metadata(value))
            .transpose()?
            .unwrap_or(McpLifecycleScope::AgentSession);
        let resource_scope = descriptor
            .resource_scope_hints
            .first()
            .cloned()
            .unwrap_or_else(|| resource_scope_for_lifecycle(&lifecycle));
        Ok(Self {
            server_id: descriptor.provider_id.clone(),
            visible_tool_name: descriptor.tool_name.clone(),
            backend_tool_name,
            lifecycle,
            resource_scope,
            conflict_namespace: descriptor.conflict_namespace.clone(),
            definition_source: descriptor
                .metadata
                .get(MCP_DESCRIPTOR_DEFINITION_SOURCE)
                .cloned(),
        })
    }
}

/// Runtime-host owned routing index for service-backed MCP invocation.
#[derive(Debug, Default)]
pub(crate) struct McpToolDescriptorIndex {
    routes: RwLock<BTreeMap<String, McpToolDescriptorRoute>>,
}

impl McpToolDescriptorIndex {
    /// Store all valid descriptors from a catalog refresh.
    ///
    /// Invalid descriptors are returned as sanitized errors so callers can log
    /// them without exposing raw server configuration.  Existing routes for
    /// untouched tools remain in place because separate catalog calls may be
    /// scoped to different applications or agents.
    pub async fn upsert_descriptors(
        &self,
        descriptors: &[CapabilityToolDescriptor],
    ) -> Vec<Result<McpToolDescriptorRoute, String>> {
        let mut outcomes = Vec::new();
        let mut routes = self.routes.write().await;
        for descriptor in descriptors {
            match McpToolDescriptorRoute::from_descriptor(descriptor) {
                Ok(route) => {
                    routes.insert(route.visible_tool_name.clone(), route.clone());
                    outcomes.push(Ok(route));
                }
                Err(error) => outcomes.push(Err(error)),
            }
        }
        outcomes
    }

    /// Find the authoritative service route for one visible tool name.
    pub async fn route_for_visible_tool(
        &self,
        visible_tool_name: &str,
    ) -> Option<McpToolDescriptorRoute> {
        self.routes.read().await.get(visible_tool_name).cloned()
    }

    /// Return the number of remembered routes for diagnostics and tests.
    #[cfg(test)]
    pub async fn len(&self) -> usize {
        self.routes.read().await.len()
    }
}

fn lifecycle_from_metadata(value: &str) -> Result<McpLifecycleScope, String> {
    match value {
        "global" => Ok(McpLifecycleScope::Global),
        "app" => Ok(McpLifecycleScope::App),
        "session" => Ok(McpLifecycleScope::Session),
        "agent_session" => Ok(McpLifecycleScope::AgentSession),
        "call" => Ok(McpLifecycleScope::Call),
        other => Err(format!("unsupported MCP lifecycle metadata '{other}'")),
    }
}

fn resource_scope_for_lifecycle(lifecycle: &McpLifecycleScope) -> CapabilityToolResourceScope {
    match lifecycle {
        McpLifecycleScope::Global => CapabilityToolResourceScope::Global,
        McpLifecycleScope::App => CapabilityToolResourceScope::Application,
        McpLifecycleScope::Session => CapabilityToolResourceScope::Session,
        McpLifecycleScope::AgentSession => CapabilityToolResourceScope::AgentSession,
        McpLifecycleScope::Call => CapabilityToolResourceScope::Call,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::CapabilityToolOriginKind;

    fn descriptor() -> CapabilityToolDescriptor {
        let mut descriptor = CapabilityToolDescriptor::new(
            MCP_SERVICE_ID,
            "server-a",
            "mcp.tool.server-a.lookup",
            "mcp_lookup",
            "Lookup values",
            serde_json::json!({"type": "object"}),
            CapabilityToolOriginKind::Mcp,
        )
        .unwrap()
        .with_policy_hints(
            vec!["mcp.tool.invoke".into()],
            vec![CapabilityToolResourceScope::AgentSession],
        );
        descriptor
            .metadata
            .insert(MCP_DESCRIPTOR_BACKEND_TOOL_NAME.into(), "lookup".into());
        descriptor.metadata.insert(
            MCP_DESCRIPTOR_LIFECYCLE_SCOPE.into(),
            "agent_session".into(),
        );
        descriptor
    }

    #[tokio::test]
    async fn index_maps_visible_tool_to_backend_route() {
        let index = McpToolDescriptorIndex::default();
        let descriptor = descriptor();
        let outcomes = index.upsert_descriptors(&[descriptor]).await;
        assert_eq!(outcomes.len(), 1);
        assert_eq!(index.len().await, 1);

        let route = index.route_for_visible_tool("mcp_lookup").await.unwrap();
        assert_eq!(route.server_id, "server-a");
        assert_eq!(route.backend_tool_name, "lookup");
        assert_eq!(route.lifecycle, McpLifecycleScope::AgentSession);
    }

    #[tokio::test]
    async fn index_rejects_descriptor_without_backend_metadata() {
        let index = McpToolDescriptorIndex::default();
        let mut descriptor = descriptor();
        descriptor.metadata.remove(MCP_DESCRIPTOR_BACKEND_TOOL_NAME);

        let outcomes = index.upsert_descriptors(&[descriptor]).await;

        assert!(outcomes[0].is_err());
        assert_eq!(index.len().await, 0);
    }
}
