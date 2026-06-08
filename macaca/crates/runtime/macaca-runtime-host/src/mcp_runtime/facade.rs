//! Stable host-facing Facade for MCP runtime orchestration (Adapter pattern).
//!
//! Delegates all operations to [`super::manager::McpRuntimeManager`] while preserving
//! a narrow public API for service providers and legacy bootstrap paths.

use std::sync::Arc;
use std::time::Duration;

use macaca_framework::mcp::{McpResourceDef, McpResourceRead, McpResourceTemplateDef};
use macaca_framework::tool::Toolkit;
use macaca_proto::{ApplicationId, CapabilityToolDescriptor, CapabilityToolInvocationResult, TraceContext};

use crate::lease::McpSessionLease;

use super::manager::McpRuntimeManager;
// Facade holds the manager behind Arc; all host-facing MCP operations delegate here.
use super::types::{
    McpRuntimeContext, McpRuntimeStatus, McpServerDefinition, McpToolPolicy,
};

/// Stable host-facing facade for MCP runtime orchestration.
#[allow(deprecated)]
#[derive(Debug, Clone, Default)]
pub struct McpRuntimeFacade {
    manager: Arc<McpRuntimeManager>,
}

#[allow(deprecated)]
impl McpRuntimeFacade {
    #[allow(deprecated)]
    pub fn new() -> Self {
        Self {
            manager: Arc::new(McpRuntimeManager::new()),
        }
    }

    pub fn from_manager(manager: Arc<McpRuntimeManager>) -> Self {
        Self { manager }
    }

    #[allow(deprecated)]
    pub async fn load_default() -> Self {
        Self {
            manager: Arc::new(McpRuntimeManager::load_default().await),
        }
    }

    #[allow(deprecated)]
    pub async fn upsert_definition(&self, definition: McpServerDefinition) {
        self.manager.upsert_definition(definition).await;
    }

    /// Snapshot registered MCP server definitions for service provider assembly.
    ///
    /// External consumers must obtain MCP catalogs through `service.mcp` snapshot
    /// commands via `SystemMcpClient` instead of reading runtime-host internals.
    #[allow(deprecated)]
    pub async fn snapshot_server_definitions(&self) -> Vec<McpServerDefinition> {
        tracing::trace!("mcp facade delegating server definition snapshot to manager");
        self.manager.snapshot_server_definitions().await
    }

    #[allow(deprecated)]
    pub async fn probe(&self, policy: &McpToolPolicy) -> Vec<McpRuntimeStatus> {
        self.manager.probe_statuses(policy).await
    }

    /// Return service-owned MCP descriptors that carry explicit routing
    /// metadata for later `mcp.tool.invoke` calls.
    ///
    /// This is the descriptor-index side of the service boundary.  Shells may
    /// render or adapt the descriptors, but the backend server/tool mapping is
    /// emitted here by the runtime-host owner instead of being reconstructed by
    /// string parsing in Web or framework code.
    #[allow(deprecated)]
    pub async fn tool_descriptors(
        &self,
        policy: &McpToolPolicy,
    ) -> Vec<Result<CapabilityToolDescriptor, String>> {
        self.manager.tool_descriptors(policy).await
    }

    /// Invoke an MCP tool through the service-owned runtime path.
    ///
    /// The method creates a scoped runtime lease before protocol side effects,
    /// dispatches to the backend MCP tool name carried by the descriptor, and
    /// releases call-scoped leases immediately after completion.  The current
    /// implementation uses the existing protocol client factory as the low-level
    /// Strategy while keeping Agent OS invocation semantics in runtime-host.
    #[allow(deprecated)]
    pub async fn invoke_tool(
        &self,
        server_id: &str,
        backend_tool_name: &str,
        visible_tool_name: &str,
        input: serde_json::Value,
        trace: TraceContext,
        context: &McpRuntimeContext,
        policy: &McpToolPolicy,
    ) -> CapabilityToolInvocationResult {
        self.manager
            .invoke_tool(
                server_id,
                backend_tool_name,
                visible_tool_name,
                input,
                trace,
                context,
                policy,
            )
            .await
    }

    /// List MCP resources through the runtime-owned protocol strategy.
    #[allow(deprecated)]
    pub async fn list_resources(
        &self,
        server_id: Option<&str>,
        policy: &McpToolPolicy,
    ) -> Vec<Result<(String, Vec<McpResourceDef>), String>> {
        self.manager.list_resources(server_id, policy).await
    }

    /// List MCP resource templates through the runtime-owned protocol strategy.
    #[allow(deprecated)]
    pub async fn list_resource_templates(
        &self,
        server_id: Option<&str>,
        policy: &McpToolPolicy,
    ) -> Vec<Result<(String, Vec<McpResourceTemplateDef>), String>> {
        self.manager
            .list_resource_templates(server_id, policy)
            .await
    }

    /// Read one MCP resource through the runtime-owned protocol strategy.
    #[allow(deprecated)]
    pub async fn read_resource(
        &self,
        server_id: &str,
        uri: &str,
        policy: &McpToolPolicy,
    ) -> Result<McpResourceRead, String> {
        self.manager.read_resource(server_id, uri, policy).await
    }

    #[allow(deprecated)]
    pub async fn register(
        &self,
        toolkit: &mut Toolkit,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
    ) -> Vec<McpRuntimeStatus> {
        Arc::clone(&self.manager)
            .register_tools(toolkit, policy, context)
            .await
    }

    #[allow(deprecated)]
    pub async fn register_definitions(
        &self,
        toolkit: &mut Toolkit,
        definitions: Vec<McpServerDefinition>,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
        on_closed: Option<Arc<dyn Fn(McpRuntimeStatus) + Send + Sync>>,
    ) -> Vec<McpRuntimeStatus> {
        Arc::clone(&self.manager)
            .register_definitions(toolkit, definitions, policy, context, on_closed)
            .await
    }

    #[allow(deprecated)]
    pub async fn cleanup_session(&self, session_id: &str) -> Vec<McpRuntimeStatus> {
        self.manager.cleanup_session(session_id).await
    }

    #[allow(deprecated)]
    pub async fn cleanup_app(&self, app_id: &ApplicationId) -> Vec<McpRuntimeStatus> {
        self.manager.cleanup_app(app_id).await
    }

    #[allow(deprecated)]
    pub async fn cleanup_all(&self) -> Vec<McpRuntimeStatus> {
        self.manager.cleanup_all().await
    }

    #[allow(deprecated)]
    pub async fn cleanup_idle(&self, ttl: Duration) -> Vec<McpRuntimeStatus> {
        self.manager.cleanup_idle(ttl).await
    }

    #[allow(deprecated)]
    pub async fn acquire_lease(
        &self,
        definition: &McpServerDefinition,
        context: &McpRuntimeContext,
    ) -> McpSessionLease {
        self.manager.acquire_lease(definition, context).await
    }

    #[allow(deprecated)]
    pub async fn release_lease(&self, lease: McpSessionLease) -> Option<McpRuntimeStatus> {
        self.manager.release_lease(lease).await
    }
}
