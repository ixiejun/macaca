//! MCP client **Factory** — builds transport-specific [`McpClient`] implementations.
//!
//! Centralizes transport selection so runtime hosts and toolkit registration
//! never branch on wire details beyond [`McpTransportConfig`].

use super::core::McpClient;
use super::error::McpError;
use super::stdio::StdioMcpClient;
use super::types::{McpTimeouts, McpTransportConfig};

#[cfg(feature = "mcp-http")]
use super::http::{HttpMcpClient, HttpMcpTransport};

/// Build a framework MCP client from transport configuration.
pub fn client_from_transport(
    config: McpTransportConfig,
    timeouts: McpTimeouts,
) -> Result<Box<dyn McpClient>, McpError> {
    tracing::debug!(
        target = "macaca_framework::mcp::factory",
        transport = ?config,
        "creating MCP client from transport config"
    );

    match config {
        McpTransportConfig::Stdio {
            command,
            args,
            env,
            cwd,
        } => Ok(Box::new(StdioMcpClient::from_stdio_config(
            command, args, env, cwd, timeouts,
        ))),
        McpTransportConfig::Sse { url, headers } => {
            #[cfg(feature = "mcp-http")]
            {
                Ok(Box::new(HttpMcpClient::new(
                    HttpMcpTransport::Sse,
                    url,
                    headers,
                    timeouts,
                )))
            }
            #[cfg(not(feature = "mcp-http"))]
            {
                let _ = (url, headers);
                Err(McpError::UnsupportedTransport(
                    "sse requires macaca-framework feature mcp-http".to_string(),
                ))
            }
        }
        McpTransportConfig::StreamableHttp { url, headers } => {
            #[cfg(feature = "mcp-http")]
            {
                Ok(Box::new(HttpMcpClient::new(
                    HttpMcpTransport::StreamableHttp,
                    url,
                    headers,
                    timeouts,
                )))
            }
            #[cfg(not(feature = "mcp-http"))]
            {
                let _ = (url, headers);
                Err(McpError::UnsupportedTransport(
                    "streamable_http requires macaca-framework feature mcp-http".to_string(),
                ))
            }
        }
    }
}
