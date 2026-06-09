//! MCP transport bridge for runtime-host MCP client construction.
//!
//! **Bridge + Strategy**: `McpTransport` abstracts how `McpTransportConfig` values become
//! live `McpClient` handles. `ConfigBackedMcpTransport` is the default adapter that
//! delegates to `macaca-framework::mcp::client_from_transport`, preserving the existing
//! config schema while keeping runtime-host free of transport-specific subprocess code.
//!
//! Trade-off: one adapter covers stdio/SSE/streamable HTTP today; new transports add a
//! type here rather than leaking framework client details into service providers.

use macaca_framework::mcp::{
    client_from_transport, McpClient, McpError, McpTimeouts, McpTransportConfig,
};

/// Runtime-host bridge for MCP transport-backed client creation.
pub trait McpTransport: Send + Sync {
    fn config(&self) -> &McpTransportConfig;
    fn label(&self) -> &'static str;
    fn create_client(&self, timeouts: McpTimeouts) -> Result<Box<dyn McpClient>, McpError>;
}

/// Minimal adapter that preserves the existing `McpTransportConfig` schema
/// while moving client creation behind a bridge boundary.
#[derive(Debug, Clone)]
pub struct ConfigBackedMcpTransport {
    config: McpTransportConfig,
}

impl ConfigBackedMcpTransport {
    pub fn new(config: McpTransportConfig) -> Self {
        Self { config }
    }
}

impl McpTransport for ConfigBackedMcpTransport {
    fn config(&self) -> &McpTransportConfig {
        &self.config
    }

    fn label(&self) -> &'static str {
        match self.config() {
            McpTransportConfig::Stdio { .. } => "stdio",
            McpTransportConfig::Sse { .. } => "sse",
            McpTransportConfig::StreamableHttp { .. } => "streamable_http",
        }
    }

    fn create_client(&self, timeouts: McpTimeouts) -> Result<Box<dyn McpClient>, McpError> {
        client_from_transport(self.config.clone(), timeouts)
    }
}

pub fn bridge_for_config(config: McpTransportConfig) -> ConfigBackedMcpTransport {
    ConfigBackedMcpTransport::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn config_backed_transport_reports_labels() {
        let stdio = ConfigBackedMcpTransport::new(McpTransportConfig::Stdio {
            command: "echo".into(),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
        });
        assert_eq!(stdio.label(), "stdio");

        let sse = ConfigBackedMcpTransport::new(McpTransportConfig::Sse {
            url: "https://example.com/sse".into(),
            headers: BTreeMap::new(),
        });
        assert_eq!(sse.label(), "sse");

        let http = ConfigBackedMcpTransport::new(McpTransportConfig::StreamableHttp {
            url: "https://example.com/http".into(),
            headers: BTreeMap::new(),
        });
        assert_eq!(http.label(), "streamable_http");
    }
}
