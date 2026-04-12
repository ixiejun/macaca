//! `aos-mcp` — MCP (Model Context Protocol) client for Agent OS.
//!
//! Connects to MCP servers and exposes their tools as native Agent OS tools,
//! allowing agents to use any MCP-compatible tool ecosystem.

pub mod adapter;
pub mod client;
pub mod driver;

pub use adapter::McpToolAdapter;
pub use client::{McpClient, McpToolInfo, McpTransport};
pub use driver::McpDriver;
