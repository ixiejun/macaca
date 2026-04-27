//! `macaca-runtime-host` — Agent OS runtime host.
//!
//! This crate owns OS-level runtime glue that is independent of any single
//! Agent OS host (HTTP, CLI, gateway, background schedulers). It contains:
//!
//! - [`mcp_runtime`] — MCP registry, runtime manager and per-scope lifecycle
//! - [`compat`] — external, declarative compatibility mappings from
//!   skill packages/binaries to MCP server definitions (no product-name
//!   hardcoding in control flow)
//!
//! Framework protocol handling stays in [`macaca_framework::mcp`]; this crate
//! provides the Agent OS-level registry, policy, status and toolkit
//! registration layered on top.

pub mod compat;
pub mod env_bridge;
pub mod mcp_runtime;

pub use env_bridge::{apply_mcp_env, McpEnvApplyOutcome};
pub use mcp_runtime::{
    ConcurrencyIsolationPolicy, McpLifecycleScope, McpRuntimeManager, McpRuntimeStatus,
    McpServerDefinition, McpToolPolicy,
};
