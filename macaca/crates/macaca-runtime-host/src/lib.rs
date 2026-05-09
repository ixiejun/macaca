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
pub mod context_service_provider;
pub mod driver_service_provider;
pub mod entitlement;
pub mod env_bridge;
pub mod factory;
pub mod lease;
pub mod llm_service_provider;
pub mod mcp_runtime;
pub mod mcp_service_provider;
pub mod memory_service_provider;
pub mod package;
pub mod plugin;
pub mod service_decorator;
pub mod service_provider;
pub mod service_runtime;
pub mod service_runtime_error;
pub mod service_runtime_event;
pub mod skill_service_provider;
pub mod transport;

pub use context_service_provider::ContextSystemServiceProvider;
pub use driver_service_provider::DriverSystemServiceProvider;
pub use entitlement::{CapabilityCallContext, EntitlementOperation, EntitlementRuntimeFacade};
#[allow(deprecated)]
pub use env_bridge::{apply_mcp_env, McpEnvApplyOutcome};
pub use factory::{McpServerFactory, RuntimeEnvBuilder};
pub use lease::McpSessionLease;
pub use llm_service_provider::LlmSystemServiceProvider;
#[allow(deprecated)]
pub use mcp_runtime::{
    apply_concurrency_isolation, probe_definition_statuses, ConcurrencyIsolationPolicy,
    McpDefinitionSource, McpLifecycleScope, McpRegistryConfig, McpRuntimeContext, McpRuntimeFacade,
    McpRuntimeManager, McpRuntimeStatus, McpRuntimeStatusState, McpServerDefinition, McpToolPolicy,
};
pub use mcp_service_provider::{mcp_service_descriptor, McpSystemServiceProvider};
pub use memory_service_provider::MemorySystemServiceProvider;
pub use package::{runtime_host_mcp_package_descriptor, RuntimeHostPackageRequirement};
pub use plugin::{
    plugin_failure_event, DescriptorPluginHost, PluginHost, PluginHostFactory,
    PluginLifecycleController, PluginManifestValidator, PluginRuntimeFacade, PluginRuntimeGuard,
    PluginRuntimeResult,
};
pub use service_decorator::{
    AllowAllServiceRuntimePolicy, DenyAllServiceRuntimePolicy, EntitlementRuntimeDecorator,
    MeteringRuntimeDecorator, PolicyRuntimeDecorator, ResourceRuntimeDecorator,
    ServiceRuntimeCallContext, ServiceRuntimeDecorator, ServiceRuntimePolicy,
    ServiceRuntimePolicyDecision, TraceRequiredRuntimeDecorator,
};
pub use service_provider::{
    ServiceProviderFactory, ServiceProviderFactoryContext, ServiceProviderInstance,
    StaticServiceProviderFactory,
};
pub use service_runtime::ServiceRuntime;
pub use service_runtime_error::{ServiceRuntimeConfig, ServiceRuntimeError};
pub use service_runtime_event::{
    InMemoryServiceRuntimeEventSink, ServiceRuntimeEvent, ServiceRuntimeEventSink,
    ServiceRuntimeServiceSnapshot, ServiceRuntimeSnapshot,
};
pub use skill_service_provider::SkillSystemServiceProvider;
pub use transport::{ConfigBackedMcpTransport, McpTransport};
