//! MCP (Model Context Protocol) support (**Facade**).
//!
//! Provides:
//! - [`McpClient`] trait for connecting to MCP servers
//! - [`StdioMcpClient`] — JSON-RPC 2.0 over child-process stdin/stdout
//! - [`McpToolHandler`] — bridges MCP tools into the [`Toolkit`] system
//! - [`register_mcp_tools`] — bulk-registers MCP tools into a [`Toolkit`]
//!
//! # Module tree (P4 iteration 108)
//! - `error` — [`McpError`] taxonomy
//! - `types` — transport config, tool/resource wire types
//! - `core` — [`McpClient`] Strategy trait
//! - `parse` — shared `tools/call` result decoding
//! - `stdio` — [`StdioMcpClient`] child-process Adapter
//! - `http` — [`HttpMcpClient`] SSE/streamable HTTP Adapter (`mcp-http` feature)
//! - `factory` — [`client_from_transport`] Factory
//! - `adapter` — [`McpToolHandler`] Toolkit Adapter
//! - `registration` — bulk tool registration Bridge
//! - `tests` — contract tests per transport and registration policy

mod adapter;
mod core;
mod error;
mod factory;
#[cfg(feature = "mcp-http")]
mod http;
mod parse;
mod registration;
mod stdio;
mod types;

#[cfg(test)]
mod tests;

pub use adapter::McpToolHandler;
pub use core::McpClient;
pub use error::McpError;
pub use factory::client_from_transport;
#[cfg(feature = "mcp-http")]
pub use http::{HttpMcpClient, HttpMcpTransport};
pub use registration::{register_mcp_tools, register_mcp_tools_with_options};
pub use stdio::StdioMcpClient;
pub use types::{
    McpCallResult, McpResourceDef, McpResourceRead, McpResourceTemplateDef, McpSessionMode,
    McpTimeouts, McpToolDef, McpToolNameConflictPolicy, McpToolRegistrationOptions,
    McpTransportConfig,
};
