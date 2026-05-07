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
pub mod entitlement;
pub mod env_bridge;
pub mod factory;
pub mod lease;
pub mod mcp_runtime;
pub mod package;
pub mod plugin;
pub mod transport;

#[allow(deprecated)]
pub use env_bridge::{apply_mcp_env, McpEnvApplyOutcome};
pub use factory::{McpServerFactory, RuntimeEnvBuilder};
pub use entitlement::{CapabilityCallContext, EntitlementOperation, EntitlementRuntimeFacade};
pub use lease::McpSessionLease;
#[allow(deprecated)]
pub use mcp_runtime::{
    apply_concurrency_isolation, probe_definition_statuses, ConcurrencyIsolationPolicy,
    McpDefinitionSource, McpLifecycleScope, McpRegistryConfig, McpRuntimeContext, McpRuntimeFacade,
    McpRuntimeManager, McpRuntimeStatus, McpRuntimeStatusState, McpServerDefinition, McpToolPolicy,
};
pub use package::{runtime_host_mcp_package_descriptor, RuntimeHostPackageRequirement};
pub use plugin::{
    plugin_failure_event, DescriptorPluginHost, PluginHost, PluginHostFactory,
    PluginLifecycleController, PluginManifestValidator, PluginRuntimeFacade, PluginRuntimeGuard,
    PluginRuntimeResult,
};
pub use transport::{ConfigBackedMcpTransport, McpTransport};
