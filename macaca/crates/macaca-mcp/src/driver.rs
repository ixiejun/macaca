//! `McpDriver` — wraps an MCP server as a `SoftwareDriver`.
//!
//! Registers the MCP server's tools into the Agent OS driver system,
//! allowing agents to discover and use MCP tools through the standard
//! driver/tool interface.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use macaca_proto::{MacacaResult, DriverId};
use macaca_tools::Tool;

use macaca_driver::driver::{DriverManifest, DriverType, SoftwareDriver};

use crate::adapter::tools_from_client;
use crate::client::{McpClient, McpTransport};

/// A software driver backed by an MCP server.
///
/// When initialized, connects to the MCP server and discovers its tools.
/// The tools are then exposed as standard Agent OS tools.
pub struct McpDriver {
    manifest: DriverManifest,
    client: Arc<RwLock<McpClient>>,
    tools: Vec<Box<dyn Tool>>,
}

impl McpDriver {
    /// Create a new MCP driver for a specific server.
    pub fn new(server_name: impl Into<String>, transport: McpTransport) -> Self {
        let name = server_name.into();
        let client = McpClient::new(name.clone(), transport);

        Self {
            manifest: DriverManifest {
                id: DriverId::new(),
                name: format!("mcp:{}", name),
                version: env!("CARGO_PKG_VERSION").into(),
                driver_type: DriverType::McpProtocol,
                description: format!("MCP server: {}", name),
                capabilities: vec![],
            },
            client: Arc::new(RwLock::new(client)),
            tools: Vec::new(),
        }
    }
}

#[async_trait]
impl SoftwareDriver for McpDriver {
    fn manifest(&self) -> &DriverManifest {
        &self.manifest
    }

    async fn initialize(&mut self) -> MacacaResult<()> {
        // Connect to the MCP server.
        self.client.write().await.connect().await?;

        // Discover tools.
        self.tools = tools_from_client(Arc::clone(&self.client)).await?;

        // Update manifest capabilities from discovered tools.
        self.manifest.capabilities = self
            .tools
            .iter()
            .map(|t| t.name().to_string())
            .collect();

        tracing::info!(
            driver = %self.manifest.name,
            tools = self.tools.len(),
            "MCP driver initialized"
        );

        Ok(())
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        // Re-create adapters from the client (since we can't clone Box<dyn Tool>).
        // In practice, tools are cached after initialize().
        // For now, we return empty since tools require async creation.
        // The real tools were stored during initialize().
        // We need to re-create them from the client.
        let client = Arc::clone(&self.client);
        let tools_info = {
            // We can't await here, but tools were cached during initialize.
            // Return tool adapters using the cached tool info.
            self.manifest
                .capabilities
                .iter()
                .map(|name| {
                    let info = crate::client::McpToolInfo {
                        name: name.clone(),
                        description: String::new(),
                        input_schema: serde_json::json!({"type": "object"}),
                    };
                    Box::new(crate::adapter::McpToolAdapter::new(
                        Arc::clone(&client),
                        info,
                    )) as Box<dyn Tool>
                })
                .collect()
        };
        tools_info
    }

    async fn health_check(&self) -> MacacaResult<bool> {
        Ok(self.client.read().await.is_connected())
    }

    async fn shutdown(&mut self) -> MacacaResult<()> {
        self.client.write().await.disconnect().await?;
        self.tools.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::McpToolInfo;

    #[tokio::test]
    async fn mcp_driver_lifecycle() {
        let mut driver = McpDriver::new(
            "test-mcp",
            McpTransport::Stdio {
                command: "echo".into(),
                args: vec![],
                env: vec![],
            },
        );

        assert_eq!(driver.manifest().driver_type, DriverType::McpProtocol);
        assert!(driver.manifest().name.contains("mcp:test-mcp"));

        // Pre-register tools for testing (since we use a stub client).
        {
            let mut client = driver.client.write().await;
            client.connect().await.unwrap();
            client.register_tools(vec![
                McpToolInfo {
                    name: "search".into(),
                    description: "Search the web".into(),
                    input_schema: serde_json::json!({"type": "object"}),
                },
            ]);
        }

        // Re-initialize to pick up tools.
        // First disconnect so initialize can reconnect.
        driver.client.write().await.disconnect().await.unwrap();
        driver.client.write().await.register_tools(vec![
            McpToolInfo {
                name: "search".into(),
                description: "Search the web".into(),
                input_schema: serde_json::json!({"type": "object"}),
            },
        ]);
        driver.initialize().await.unwrap();

        assert!(driver.health_check().await.unwrap());
        assert_eq!(driver.manifest().capabilities.len(), 1);
        assert!(driver.manifest().capabilities.contains(&"search".into()));

        let tools = driver.tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "search");

        driver.shutdown().await.unwrap();
        assert!(!driver.health_check().await.unwrap());
    }
}
