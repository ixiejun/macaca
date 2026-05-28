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
pub(crate) mod app_protocol_service_commands;
pub mod app_protocol_service_provider;
pub mod application_hosts;
pub mod application_service_provider;
pub(crate) mod approval_service_commands;
pub mod approval_service_provider;
pub mod autonomy_dispatch;
pub mod autonomy_evolution_live_executor;
pub mod autonomy_evolution_service_provider;
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
pub(crate) mod file_service_local;
pub mod file_service_provider;
#[cfg(test)]
pub(crate) mod finance_live_data;
#[cfg(test)]
pub mod finance_llm_analysis_provider;
pub mod genui_surface_store;
pub(crate) mod hook_service_commands;
pub mod hook_service_provider;
pub mod interaction_ledger_store;
pub mod interaction_service_bootstrap;
pub(crate) mod interaction_service_items;
pub mod interaction_service_provider;
pub(crate) mod interaction_service_threads;
pub(crate) mod interaction_service_turns;
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
pub(crate) mod process_service_local;
pub mod process_service_provider;
pub(crate) mod process_service_records;
pub mod route_c_bootstrap;
pub(crate) mod sandbox_service_local;
pub mod sandbox_service_provider;
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
pub(crate) mod skill_alias_resolution;
pub(crate) mod skill_service_codec;
pub(crate) mod skill_service_content_mutation;
pub(crate) mod skill_service_experience_routing;
pub(crate) mod skill_service_governance_store;
pub mod skill_service_provider;
pub(crate) mod skill_service_provider_curation;
pub(crate) mod skill_service_provider_curation_log;
pub(crate) mod skill_service_provider_event_journal;
pub(crate) mod skill_service_provider_lifecycle;
pub(crate) mod skill_service_provider_materialization_operator;
pub(crate) mod skill_service_provider_merge;
pub(crate) mod skill_service_provider_package_recovery;
pub(crate) mod skill_service_provider_proposal_materialization;
pub(crate) mod skill_service_provider_proposal_processing;
pub(crate) mod skill_service_provider_proposals;
pub(crate) mod skill_service_provider_semantic_review;
pub(crate) mod skill_service_provider_state;
pub mod store_entitlement_admission;
pub mod store_service_provider;
pub mod tool_family_providers;
pub mod tool_service_availability;
pub mod tool_service_environment;
pub mod tool_service_gateway;
pub mod tool_service_invocation;
pub mod tool_service_planning;
pub mod tool_service_provider;
pub mod tool_service_provider_state;
pub mod tool_service_result;
pub mod transport;
pub mod wasm_runtime_provider;
pub mod web3_service_provider;

#[cfg(test)]
mod app_protocol_service_provider_tests;
#[cfg(test)]
mod approval_service_provider_tests;
#[cfg(test)]
mod file_service_provider_tests;
#[cfg(test)]
mod hook_service_provider_tests;
#[cfg(test)]
mod interaction_service_provider_tests;
#[cfg(test)]
mod interaction_service_state_tests;
#[cfg(test)]
mod process_service_provider_tests;
#[cfg(test)]
mod sandbox_service_provider_tests;
#[cfg(test)]
mod service_router_tests;
#[cfg(test)]
mod skill_content_mutation_tests;
#[cfg(test)]
mod skill_governance_store_logging_tests;
#[cfg(test)]
mod skill_materialization_operator_tests;
#[cfg(test)]
mod skill_optional_provider_boundary_tests;
#[cfg(test)]
mod skill_proposal_lifecycle_tests;
#[cfg(test)]
mod skill_proposal_materialization_tests;
#[cfg(test)]
mod skill_proposal_processing_tests;
#[cfg(test)]
mod skill_sanitization_boundary_tests;
#[cfg(test)]
mod skill_self_evolution_evaluation_harness_fixture;
#[cfg(test)]
mod skill_self_evolution_evaluation_harness_tests;
#[cfg(test)]
mod skill_service_lifecycle_tests;
#[cfg(test)]
mod skill_service_merge_tests;
#[cfg(test)]
mod skill_service_provider_tests;
#[cfg(test)]
mod skill_service_usage_tests;
#[cfg(test)]
mod tool_service_audit_tests;
#[cfg(test)]
mod tool_service_environment_tests;
#[cfg(test)]
mod tool_service_family_provider_tests;
#[cfg(test)]
mod tool_service_gateway_tests;
#[cfg(test)]
mod tool_service_invocation_tests;
#[cfg(test)]
mod tool_service_planning_tests;

pub use agent_context_service_provider::{
    agent_context_service_descriptor, AgentContextBackend, AgentContextSystemServiceProvider,
};
pub use agent_execution_service_provider::{
    agent_execution_service_descriptor, AgentExecutionBackend, AgentExecutionSystemServiceProvider,
};
pub use app_protocol_service_commands::app_protocol_service_command;
pub use app_protocol_service_provider::AppProtocolSystemServiceProvider;
pub use application_hosts::{
    is_application_runtime_unavailable, ApplicationHostRuntime, UnavailableApplicationRuntimeHost,
    UnavailableWasmApplicationHost, WasmApplicationHostFactory,
};
pub use application_service_provider::{
    ApplicationOrchestrationBackend, ApplicationSystemServiceProvider,
};
pub use approval_service_provider::{
    bootstrap_local_approval_service, ApprovalReviewerPolicy, ApprovalSystemServiceProvider,
    LocalApprovalReviewerPolicy,
};
pub use autonomy_evolution_live_executor::{
    AutonomyEvolutionLiveExecutionCommand, AutonomyEvolutionLiveExecutionResult,
    AutonomyEvolutionLiveExecutor, AutonomyEvolutionTargetExecutionOutcome,
};
pub use autonomy_evolution_service_provider::AutonomyEvolutionSystemServiceProvider;
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
pub use file_service_local::{FileProvider, LocalFileProvider};
pub use file_service_provider::{bootstrap_local_file_service, FileSystemServiceProvider};
#[cfg(test)]
pub use finance_llm_analysis_provider::FinanceLlmAnalysisSystemServiceProvider;
pub use genui_surface_store::ApplicationGenUiSurfaceStore;
pub use hook_service_provider::{bootstrap_local_hook_service, HookSystemServiceProvider};
pub use interaction_ledger_store::{InteractionLedgerStore, PersistInteractionLedgerStore};
pub use interaction_service_bootstrap::bootstrap_interaction_service;
pub use interaction_service_provider::{
    interaction_service_command, InteractionLedgerEvent, InteractionSystemServiceProvider,
};
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
pub use process_service_local::{LocalProcessProvider, ProcessProvider};
pub use process_service_provider::{bootstrap_local_process_service, ProcessSystemServiceProvider};
pub use route_c_bootstrap::{
    bootstrap_route_c_optional_services, RouteCBootstrapDiagnostic, RouteCHostRuntimeBundle,
    RouteCOptionalServicesBootstrap, RouteCOptionalServicesBootstrapInputs,
};
pub use sandbox_service_local::{LocalSandboxProvider, MockSandboxProvider, SandboxProvider};
pub use sandbox_service_provider::{bootstrap_local_sandbox_service, SandboxSystemServiceProvider};
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
pub use tool_family_providers::{
    industrial_tool_family_provider_contributor, industrial_tool_family_provider_inventory,
    industrial_tool_family_toolsets, industrial_tool_planning_service,
    IndustrialToolFamilyProviderInventory, REQUIRED_INDUSTRIAL_TOOL_FAMILIES,
};
pub use tool_service_availability::{AvailabilitySignalSet, ToolAvailabilityEvaluator};
pub use tool_service_environment::{
    industrial_tool_runtime_environment_service, StaticToolRuntimeEnvironmentProvider,
    ToolRuntimeEnvironmentInvocation, ToolRuntimeEnvironmentProvider,
    ToolRuntimeEnvironmentService, UnavailableToolRuntimeEnvironmentProvider,
};
pub use tool_service_gateway::{
    industrial_tool_managed_gateway_service, StaticToolManagedGatewayProvider,
    ToolManagedGatewayInvocation, ToolManagedGatewayProvider, ToolManagedGatewayService,
    UnavailableToolManagedGatewayProvider,
};
pub use tool_service_planning::{
    CapabilityToolDescriptorContributor, StaticToolDescriptorContributor,
    ToolDescriptorContributor, ToolPlanningService, ToolPlanningToolsetResolver,
};
pub use tool_service_provider::{bootstrap_tool_planning_service, ToolSystemServiceProvider};
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
