//! `aos-mcp` — MCP (Model Context Protocol) client for Agent OS.
//!
//! Connects to MCP servers and exposes their tools as native Agent OS tools,
//! allowing agents to use any MCP-compatible tool ecosystem.

pub mod client;
pub mod adapter;
pub mod driver;

pub use client::{McpClient, McpTransport, McpToolInfo};
pub use adapter::McpToolAdapter;
pub use driver::McpDriver;
