//! MCP protocol client factory Strategy and test adapter wrapper.
//!
//! Production uses [`bridge_for_config`] as the single protocol implementation seam.
//! Tests inject deterministic clients through [`McpClientFactory`].

use std::sync::Arc;

use macaca_framework::mcp::{McpClient, McpError, McpResourceDef, McpResourceRead,
    McpResourceTemplateDef, McpTimeouts, McpTransportConfig};

use crate::transport::{bridge_for_config, McpTransport};

use super::types::{McpServerDefinition};

/// Strategy used by runtime-host to create low-level MCP protocol clients.
///
/// Production code uses [`bridge_for_config`] so `macaca-framework::mcp`
/// remains the single protocol implementation.  Tests may inject deterministic
/// clients through this seam, which lets service-level policy, routing, audit
/// metadata, cleanup, and timeout behavior be validated without starting
/// concrete MCP servers.
pub(crate) type McpClientFactory =
    dyn Fn(&McpServerDefinition, McpTimeouts) -> Result<Box<dyn McpClient>, McpError> + Send + Sync;

pub(crate) struct ClientBox {
    inner: Box<dyn McpClient>,
}

impl ClientBox {
    /// Wrap a protocol client so it can be stored behind `Arc<RwLock<dyn McpClient>>`.
    pub(crate) fn new(inner: Box<dyn McpClient>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl McpClient for ClientBox {
    async fn connect(&mut self) -> Result<(), macaca_framework::mcp::McpError> {
        self.inner.connect().await
    }

    async fn list_tools(
        &mut self,
    ) -> Result<Vec<macaca_framework::mcp::McpToolDef>, macaca_framework::mcp::McpError> {
        self.inner.list_tools().await
    }

    async fn call_tool(
        &mut self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<macaca_framework::mcp::McpCallResult, macaca_framework::mcp::McpError> {
        self.inner.call_tool(name, args).await
    }

    async fn list_resources(&mut self) -> Result<Vec<McpResourceDef>, McpError> {
        self.inner.list_resources().await
    }

    async fn list_resource_templates(&mut self) -> Result<Vec<McpResourceTemplateDef>, McpError> {
        self.inner.list_resource_templates().await
    }

    async fn read_resource(&mut self, uri: &str) -> Result<McpResourceRead, McpError> {
        self.inner.read_resource(uri).await
    }

    async fn close(&mut self) -> Result<(), macaca_framework::mcp::McpError> {
        self.inner.close().await
    }

    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }
}

pub(crate) fn default_mcp_client_factory() -> Arc<McpClientFactory> {
    Arc::new(|definition, timeouts| {
        let transport = bridge_for_config(definition.transport.clone());
        transport.create_client(timeouts)
    })
}
