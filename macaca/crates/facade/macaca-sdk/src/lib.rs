//! `aos-sdk` — declarative agent SDK for Agent OS.
//!
//! Provides YAML/TOML configuration parsing, a fluent builder API,
//! and registration helpers to construct and register agents with
//! the kernel from declarative config files.

pub mod ability_kit;
pub mod app_protocol_client;
pub mod application;
pub mod application_client;
pub mod application_kit;
pub mod application_testkit;
pub mod autonomy_evolution_client;
pub mod builder;
pub mod config;
pub mod context_client;
pub mod driver_client;
pub mod entitlement_client;
pub mod evm;
pub mod evm_client;
pub mod facade;
pub mod heartbeat_client;
pub mod interaction_client;
pub mod llm_client;
pub mod mcp_client;
pub mod memory_client;
pub mod package_client;
pub mod package_fixtures;
pub mod payment_client;
pub mod persona;
pub mod persona_prototype;
pub mod plugin_capability_client;
pub mod plugin_client;
pub mod plugin_hook_client;
pub mod plugin_sdk;
pub mod registry_api;
pub mod scheduled_agent_task_client;
pub mod scheduler_client;
pub mod service_client;
pub mod skill_client;
mod skill_client_service_backed;
pub mod skill_operator_client;
pub mod spec;
pub mod status_client;
pub mod store_client;
pub mod system_facade;
pub mod task_client;
pub mod tool_client;
pub mod trace_client;
pub mod validation;
pub mod web3_client;
pub mod workbench_client;

pub use ability_kit::{AbilityDescriptorBuilder, AbilityKit};
pub use app_protocol_client::{
    ServiceBackedAppProtocolClient, SystemAppProtocolClient, UnavailableSystemAppProtocolClient,
};
pub use application::{
    service_call_command, trace_emit_command, ApplicationAbiBuilder, ApplicationHostCommandBuilder,
};
pub use application_client::{
    ServiceBackedApplicationClient, SystemApplicationClient, UnavailableSystemApplicationClient,
};
pub use application_kit::{
    generate_wasm_guest_bindings, ApplicationKit, ApplicationManifestBuilder,
    RustWasmBindgenBackend, WasmBindgenBackend, WasmBindgenDiagnostic, WasmBindgenInput,
    WasmBindgenOutput, WasmComponentApplicationDescriptor, WasmComponentApplicationScaffold,
    WasmGuestBindingPlan, WasmMockHostImportBinding,
};
pub use application_testkit::{
    ApplicationContractDiagnostic, ApplicationContractReport, ApplicationContractTestKit,
};
pub use autonomy_evolution_client::{
    ServiceBackedAutonomyEvolutionClient, SystemAutonomyEvolutionClient,
    UnavailableSystemAutonomyEvolutionClient,
};
pub use builder::{AgentBuilder, DeclarativeAgent};
pub use config::AgentConfig;
pub use context_client::{
    ServiceBackedContextClient, SystemContextClient, UnavailableSystemContextClient,
};
pub use driver_client::{
    ServiceBackedDriverClient, SystemDriverClient, UnavailableSystemDriverClient,
};
pub use entitlement_client::{
    ServiceBackedEntitlementClient, SystemEntitlementClient, UnavailableSystemEntitlementClient,
};
pub use evm::MacacaEvmSdk;
pub use evm_client::{ServiceBackedEvmClient, SystemEvmClient, UnavailableSystemEvmClient};
pub use facade::{AgentRegistryApi, KernelAgentRegistry, KernelPrimitiveSdk, MacacaSdk};
pub use heartbeat_client::{
    ServiceBackedHeartbeatClient, SystemHeartbeatClient, UnavailableSystemHeartbeatClient,
};
pub use interaction_client::{
    ServiceBackedInteractionClient, SystemInteractionClient, UnavailableSystemInteractionClient,
};
pub use llm_client::{ServiceBackedLlmClient, SystemLlmClient, UnavailableSystemLlmClient};
pub use macaca_skill::{
    SkillAuthorKind, SkillCurationLifecycleAction, SkillCurationLifecycleCommand,
    SkillCurationRollbackCommand, SkillCurationRunCommand, SkillEvolutionPromoteDraftCommand,
    SkillEvolutionRejectDraftCommand, SkillExperienceProposalSnapshotCommand,
    SkillGovernanceSnapshotCommand, SkillServicePolicyHints, SkillServiceScope,
};
pub use macaca_task::{TaskServiceSnapshot, TaskServiceSnapshotCommand};
pub use mcp_client::{ServiceBackedMcpClient, SystemMcpClient, UnavailableSystemMcpClient};
pub use memory_client::{
    ServiceBackedMemoryClient, SystemMemoryClient, UnavailableSystemMemoryClient,
};
pub use package_fixtures::{
    application_platform_agent_fixture, application_platform_genui_fixture,
    application_platform_headless_fixture, application_platform_plugin_enhanced_fixture,
    application_platform_store_entitled_fixture, application_platform_wasm_skeleton_fixture,
    driver_plugin_fixture, evm_optional_fixture, free_skill_fixture, gateway_plugin_fixture,
    genui_app_fixture, invalid_missing_required_service_fixture, invalid_missing_runtime_fixture,
    paid_skill_fixture, wasm_stub_app_fixture, web3_optional_fixture, yaml_app_fixture,
    ApplicationPlatformFixture, EcosystemPackageFixtureBuilder,
};
pub use payment_client::{
    ServiceBackedPaymentClient, SystemPaymentClient, UnavailableSystemPaymentClient,
};
pub use persona::AgentPersona;
pub use persona_prototype::{PersonaOverrides, PersonaPrototype};
pub use plugin_capability_client::{
    ServiceBackedPluginCapabilityClient, SystemPluginCapabilityClient,
    UnavailableSystemPluginCapabilityClient,
};
pub use plugin_client::{
    ServiceBackedPluginControlClient, SystemPluginControlClient,
    UnavailableSystemPluginControlClient,
};
pub use plugin_hook_client::{
    ServiceBackedPluginHookClient, SystemPluginHookClient, UnavailableSystemPluginHookClient,
};
pub use plugin_sdk::{
    PluginCapabilityBuilder, PluginConfigBuilder, PluginContext, PluginContractDiagnostic,
    PluginContractReport, PluginContractTestKit, PluginHookBuilder, PluginManifestBuilder,
    PluginRegistration, PluginRegistrationBuilder, PluginSdk, PluginSecretRequirementBuilder,
};
#[allow(deprecated)]
pub use registry_api::{register_from_config, register_from_file};
pub use scheduled_agent_task_client::{
    ServiceBackedScheduledAgentTaskClient, SystemScheduledAgentTaskClient,
    UnavailableSystemScheduledAgentTaskClient,
};
pub use scheduler_client::{
    ServiceBackedSchedulerClient, SystemSchedulerClient, UnavailableSystemSchedulerClient,
};
pub use service_client::{ServiceCallCommand, ServiceCallResult, ServiceInspectionResult};
pub use skill_client::{ServiceBackedSkillClient, SystemSkillClient, UnavailableSystemSkillClient};
pub use skill_operator_client::SystemSkillOperatorClient;
pub use spec::{AgentSpec, AgentSpecBuilder, TracePolicy};
pub use store_client::{ServiceBackedStoreClient, SystemStoreClient, UnavailableSystemStoreClient};
pub use system_facade::{
    kernel_status_snapshot, ApprovalDecisionCommand, EmptySystemPackageClient,
    EmptySystemTraceClient, PackageInspectionCommand, PackageInspectionResult,
    ServiceInspectionCommand, SessionEventQueryCommand, StaticSystemStatusDataSource, SystemFacade,
    SystemPackageClient, SystemServiceClient, SystemStatusClient, SystemStatusDataSource,
    SystemStatusSnapshot, SystemTaskClient, SystemTraceClient, TaskBoardDataSource,
    TaskBoardQueryCommand, TaskBoardQueryResult, TodoStoreTaskBoardDataSource, TraceQueryResult,
    TraceTailCommand, UnavailableSystemServiceClient,
};
pub use task_client::{TaskServiceClient, UnavailableTaskServiceClient};
pub use tool_client::{ServiceBackedToolClient, SystemToolClient, UnavailableSystemToolClient};
pub use validation::{SdkValidationChain, SdkValidator};
pub use web3_client::{ServiceBackedWeb3Client, SystemWeb3Client, UnavailableSystemWeb3Client};
pub use workbench_client::{
    is_structured_unavailable, ServiceBackedWorkbenchClient, SystemWorkbenchClient,
    SystemWorkbenchFacadeExt, UnavailableWorkbenchServiceClient, WorkbenchClientCatalog,
};
