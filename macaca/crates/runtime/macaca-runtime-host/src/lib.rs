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

pub mod application_service_provider;
pub mod compat;
pub mod context_service_provider;
pub mod driver_service_provider;
pub mod entitlement;
pub mod entitlement_service_provider;
pub mod env_bridge;
pub mod evm_service_provider;
pub mod factory;
pub mod lease;
pub mod llm_service_provider;
pub mod mcp_runtime;
pub mod mcp_service_provider;
pub mod memory_service_provider;
pub mod package;
pub mod payment_adapter;
pub mod payment_admission;
pub mod payment_service_provider;
pub mod plugin;
pub mod plugin_control;
pub mod plugin_control_service_provider;
pub mod route_c_bootstrap;
pub mod service_decorator;
pub mod service_provider;
pub mod service_runtime;
pub mod service_runtime_error;
pub mod service_runtime_event;
pub mod skill_service_provider;
pub mod store_entitlement_admission;
pub mod store_service_provider;
pub mod transport;
pub mod web3_service_provider;

pub use application_service_provider::ApplicationSystemServiceProvider;
pub use context_service_provider::ContextSystemServiceProvider;
pub use driver_service_provider::DriverSystemServiceProvider;
pub use entitlement::{CapabilityCallContext, EntitlementOperation, EntitlementRuntimeFacade};
pub use entitlement_service_provider::{
    entitlement_service_descriptor, EntitlementSystemServiceProvider,
};
#[allow(deprecated)]
pub use env_bridge::{apply_mcp_env, McpEnvApplyOutcome};
pub use evm_service_provider::{
    evm_service_descriptor, EvmProviderStrategy, EvmSystemServiceProvider, MockEvmProvider,
    UnavailableEvmProvider,
};
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
pub use payment_adapter::{LocalSimulatedPaymentAdapter, PaymentAdapterStrategy};
pub use payment_service_provider::{payment_service_descriptor, PaymentSystemServiceProvider};
pub use plugin::{
    plugin_failure_event, DescriptorPluginHost, PluginHost, PluginHostFactory,
    PluginLifecycleController, PluginManifestValidator, PluginRuntimeFacade, PluginRuntimeGuard,
    PluginRuntimeResult,
};
pub use plugin_control::{
    AdmissionCheck, AdmissionContext, AdmissionDecision, CompatibilityAdmissionCheck,
    InMemoryPluginRepository, ManifestShapeAdmissionCheck, PluginAdmissionChain,
    PluginControlService, PluginControlServiceBuilder, PluginRepository, PluginRepositoryMutation,
    PluginRepositorySnapshot, SourcePolicyAdmissionCheck,
};
pub use plugin_control_service_provider::{
    plugin_control_service_command, plugin_control_service_descriptor,
    PluginControlSystemServiceProvider,
};
pub use route_c_bootstrap::{
    bootstrap_route_c_optional_services, RouteCBootstrapDiagnostic, RouteCHostRuntimeBundle,
    RouteCOptionalServicesBootstrap, RouteCOptionalServicesBootstrapInputs,
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
pub use store_service_provider::{store_service_descriptor, StoreSystemServiceProvider};
pub use transport::{ConfigBackedMcpTransport, McpTransport};
pub use web3_service_provider::{
    web3_service_descriptor, MockWeb3Provider, UnavailableWeb3Provider, Web3ProviderStrategy,
    Web3SystemServiceProvider,
};
