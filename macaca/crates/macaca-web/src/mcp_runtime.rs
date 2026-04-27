//! Backwards-compatible thin shell.
//!
//! The real implementation was promoted to [`macaca_runtime_host::mcp_runtime`]
//! so non-HTTP hosts (CLI, gateway) can mount MCP runtimes without depending
//! on this crate. All existing `crate::mcp_runtime::*` call sites inside
//! `macaca-web` continue to work via this re-export.
pub use macaca_runtime_host::mcp_runtime::*;
