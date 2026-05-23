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

pub mod agent_context_service_provider;
pub mod agent_execution_service_provider;
pub mod application_hosts;
pub mod application_service_provider;
pub mod autonomy_dispatch;
pub(crate) mod autonomy_result_evidence;
pub mod autonomy_runtime_config;
pub mod autonomy_service_provider;
pub mod autonomy_supervisor;
pub mod compat;
pub mod context_service_provider;
pub mod domain_pack_service_provider;
pub mod driver_service_provider;
pub mod entitlement;
pub mod entitlement_service_provider;
pub mod env_bridge;
pub mod evm_service_provider;
pub mod execution_control;
pub mod execution_control_runtime;
pub mod execution_control_service_provider;
pub mod factory;
#[cfg(test)]
pub(crate) mod finance_live_data;
#[cfg(test)]
pub mod finance_llm_analysis_provider;
pub mod genui_surface_store;
pub mod lease;
pub mod llm_service_provider;
pub(crate) mod mcp_descriptor_index;
pub(crate) mod mcp_invocation_registry;
pub mod mcp_runtime;
pub mod mcp_service_provider;
pub mod memory_service_provider;
pub mod package;
pub mod payment_adapter;
pub mod payment_admission;
pub mod payment_service_provider;
pub mod plugin;
pub mod plugin_capability;
pub mod plugin_capability_service_provider;
pub mod plugin_control;
pub mod plugin_control_service_provider;
pub mod plugin_hook;
pub mod plugin_hook_service_provider;
pub mod plugin_hosts;
pub mod route_c_bootstrap;
pub mod service_audit_runtime_bundle;
pub mod service_call_audit;
pub mod service_call_audit_service_provider;
pub mod service_contract_registry;
pub mod service_decorator;
pub mod service_policy_engine;
pub mod service_provider;
pub mod service_provider_selector;
pub mod service_router;
pub mod service_runtime;
pub mod service_runtime_error;
pub mod service_runtime_event;
pub(crate) mod skill_service_codec;
pub(crate) mod skill_service_content_mutation;
pub(crate) mod skill_service_experience_routing;
pub(crate) mod skill_service_governance_store;
pub mod skill_service_provider;
pub(crate) mod skill_service_provider_curation;
pub(crate) mod skill_service_provider_lifecycle;
pub(crate) mod skill_service_provider_proposals;
pub(crate) mod skill_service_provider_state;
pub mod store_entitlement_admission;
pub mod store_service_provider;
pub mod transport;
pub mod wasm_runtime_provider;
pub mod web3_service_provider;

#[cfg(test)]
mod service_router_tests;
#[cfg(test)]
mod skill_content_mutation_tests;
#[cfg(test)]
mod skill_governance_store_logging_tests;
#[cfg(test)]
mod skill_proposal_lifecycle_tests;
#[cfg(test)]
mod skill_service_lifecycle_tests;
#[cfg(test)]
mod skill_service_provider_tests;
#[cfg(test)]
mod skill_service_usage_tests;

pub use agent_context_service_provider::{
    agent_context_service_descriptor, AgentContextBackend, AgentContextSystemServiceProvider,
};
pub use agent_execution_service_provider::{
    agent_execution_service_descriptor, AgentExecutionBackend, AgentExecutionSystemServiceProvider,
};
pub use application_hosts::{
    is_application_runtime_unavailable, ApplicationHostRuntime, UnavailableApplicationRuntimeHost,
    UnavailableWasmApplicationHost, WasmApplicationHostFactory,
};
pub use application_service_provider::{
    ApplicationOrchestrationBackend, ApplicationSystemServiceProvider,
};
pub use autonomy_runtime_config::{AutonomyProviderMode, AutonomyRuntimeConfig};
pub use autonomy_service_provider::{
    bootstrap_autonomy_local_services, bootstrap_autonomy_services,
    bootstrap_autonomy_unavailable_services, AutonomyRuntimeBundle, HeartbeatSystemServiceProvider,
    ScheduledAgentTaskSystemServiceProvider, SchedulerSystemServiceProvider,
};
pub use autonomy_supervisor::AutonomySupervisor;
pub use context_service_provider::ContextSystemServiceProvider;
#[allow(deprecated)]
pub use domain_pack_service_provider::{
    bootstrap_builtin_domain_pack_services, bootstrap_domain_pack_services,
    DomainPackProviderRegistration, DomainPackRuntimeBundle,
};
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
pub use execution_control::ExecutionControlPolicyResolver;
pub use execution_control_runtime::{
    ExecutionControlExecutionSnapshot, ExecutionControlObserver, ExecutionControlRuntimeCapability,
    ExecutionControlRuntimeSnapshot, NoopExecutionControlObserver,
};
pub use execution_control_service_provider::{
    execution_control_service_descriptor, ExecutionControlSystemServiceProvider,
};
pub use factory::{McpServerFactory, RuntimeEnvBuilder};
#[cfg(test)]
pub use finance_llm_analysis_provider::FinanceLlmAnalysisSystemServiceProvider;
pub use genui_surface_store::ApplicationGenUiSurfaceStore;
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
pub use plugin_capability::{
    built_in_service_capability, detect_slot_conflict, discover_manifest_capabilities,
    CapabilityCallAdmissionChain, CapabilityCallAdmissionCheck, CapabilityCallAdmissionContext,
    CapabilityCallAdmissionDecision, CapabilityHintAdmissionCheck,
    CapabilityOwnershipAdmissionCheck, FailClosedSlotConflictPolicy,
    PluginCapabilityConflictPolicy, PluginCapabilityRegistry, PluginCapabilityRegistryBuilder,
    PluginCapabilityService, PluginCapabilityServiceBuilder,
};
pub use plugin_capability_service_provider::{
    plugin_capability_service_command, plugin_capability_service_descriptor,
    PluginCapabilitySystemServiceProvider,
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
pub use plugin_hook::{
    DefaultPluginHookFailureStrategy, DefaultPluginHookTimeoutStrategy,
    DescriptorOnlyPluginHookExecutor, PluginHookBus, PluginHookBusBuilder, PluginHookExecutor,
    PluginHookFailureStrategy, PluginHookRegistry, PluginHookRegistryBuilder, PluginHookRunner,
    PluginHookTimeoutStrategy,
};
pub use plugin_hook_service_provider::{
    plugin_hook_service_command, plugin_hook_service_descriptor, PluginHookSystemServiceProvider,
};
pub use plugin_hosts::{
    BuiltInAdapterPluginRuntimeHost, DescriptorPluginRuntimeHost, PluginHostLifecycleSupervisor,
    PluginHostRuntime, PluginHostRuntimeFactory, PluginHostRuntimeResult, ProcessPluginRuntimeHost,
    RemoteProxyPluginRuntimeHost, UnavailablePluginRuntimeHost, WasmPluginRuntimeHost,
};
pub use route_c_bootstrap::{
    bootstrap_route_c_optional_services, RouteCBootstrapDiagnostic, RouteCHostRuntimeBundle,
    RouteCOptionalServicesBootstrap, RouteCOptionalServicesBootstrapInputs,
};
pub use service_audit_runtime_bundle::ServiceAuditRuntimeBundle;
pub use service_call_audit::{
    InMemoryServiceCallAuditSink, ServiceCallAuditEvent, ServiceCallAuditSink,
};
pub use service_call_audit_service_provider::{
    service_call_audit_replay_session_command, service_call_audit_replay_trace_command,
    service_call_audit_service_descriptor, ServiceCallAuditReplayResult,
    ServiceCallAuditReplaySessionCommand, ServiceCallAuditReplayTraceCommand,
    ServiceCallAuditSystemServiceProvider, SERVICE_CALL_AUDIT_REPLAY_SESSION_COMMAND,
    SERVICE_CALL_AUDIT_REPLAY_TRACE_COMMAND, SERVICE_CALL_AUDIT_SERVICE_ID,
};
pub use service_contract_registry::{
    InMemoryServiceContractRegistry, ServiceContractDescriptor, ServiceContractRegistry,
};
pub use service_decorator::{
    AllowAllServiceRuntimePolicy, DenyAllServiceRuntimePolicy, EntitlementRuntimeDecorator,
    MeteringRuntimeDecorator, PolicyRuntimeDecorator, ResourceRuntimeDecorator,
    ServiceRuntimeCallContext, ServiceRuntimeDecorator, ServiceRuntimePolicy,
    ServiceRuntimePolicyDecision, TraceRequiredRuntimeDecorator,
};
pub use service_policy_engine::{
    InMemoryServicePolicyEngine, ServicePolicyDecision, ServicePolicyEngine, ServicePolicyInput,
    ServicePolicyLayer,
};
pub use service_provider::{
    ServiceProviderFactory, ServiceProviderFactoryContext, ServiceProviderInstance,
    StaticServiceProviderFactory,
};
pub use service_provider_selector::{
    ProviderSelectionStrategy, ProviderSelector, ProviderSnapshot,
};
pub use service_router::{ServiceRouteRequest, ServiceRouteResponse, ServiceRouter};
pub use service_runtime::ServiceRuntime;
pub use service_runtime_error::{ServiceRuntimeConfig, ServiceRuntimeError};
pub use service_runtime_event::{
    InMemoryServiceRuntimeEventSink, ServiceRuntimeEvent, ServiceRuntimeEventSink,
    ServiceRuntimeServiceSnapshot, ServiceRuntimeSnapshot,
};
pub use skill_service_provider::SkillSystemServiceProvider;
pub use store_service_provider::{store_service_descriptor, StoreSystemServiceProvider};
pub use transport::{ConfigBackedMcpTransport, McpTransport};
pub use wasm_runtime_provider::{
    ComponentModelWasmRuntimeProvider, DefaultInProcessWasmRuntimeProvider,
    UnavailableWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmCertificationFixtureSet,
    WasmCertificationHarness, WasmCertificationProfile, WasmCertificationReport,
    WasmCertificationStatus, WasmConformanceFixtureKind, WasmExampleFixtureKind,
    WasmExecutionSession, WasmGuestHarnessFixture, WasmGuestHarnessReport, WasmGuestRuntimeHarness,
    WasmHardenedProviderEnvelope, WasmHardenedProviderMockAdapter, WasmHardenedProviderResponse,
    WasmMockHostOutcome, WasmRuntimeProviderRegistry, WasmTelemetryEvent, WasmTelemetrySink,
    WasmTelemetrySinkRef, WasmTelemetryStage, WasmToolchainFixtureReport,
};
pub use web3_service_provider::{
    web3_service_descriptor, MockWeb3Provider, UnavailableWeb3Provider, Web3ProviderStrategy,
    Web3SystemServiceProvider,
};
