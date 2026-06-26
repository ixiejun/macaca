//! `aos-sdk` — declarative agent SDK for Agent OS.
//!
//! Provides YAML/TOML configuration parsing, a fluent builder API,
//! and facade clients that register declarative agents through stable
//! service-oriented boundaries.

pub mod ability_kit;
pub mod alert_client;
pub mod app_protocol_client;
pub mod application;
pub mod application_client;
pub mod application_execution_client;
pub mod application_execution_facade;
pub mod application_kit;
pub mod application_testkit;
pub mod autonomy_evolution_client;
pub mod builder;
pub mod config;
pub mod domain_pack_bridge;
pub mod driver_client;
pub mod entitlement_client;
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
pub mod scheduled_agent_task_client;
pub mod scheduler_client;
pub mod service_client;
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

#[cfg(test)]
mod application_execution_client_tests;

pub use ability_kit::{AbilityDescriptorBuilder, AbilityKit};
pub use alert_client::{
    ServiceBackedAlertClient, SystemAlertClient, UnavailableSystemAlertClient,
    ALERT_HEALTH_COMMAND, ALERT_RAISE_COMMAND, ALERT_RESOLVE_COMMAND, ALERT_SERVICE_ID,
    ALERT_SNAPSHOT_COMMAND,
};
pub use app_protocol_client::{
    ServiceBackedAppProtocolClient, SystemAppProtocolClient, UnavailableSystemAppProtocolClient,
};
pub use application::{
    service_call_command, trace_emit_command, ApplicationAbiBuilder, ApplicationHostCommandBuilder,
};
pub use application_client::{
    ServiceBackedApplicationClient, SystemApplicationClient, UnavailableSystemApplicationClient,
};
pub use application_execution_client::{
    ServiceBackedApplicationExecutionClient, SystemApplicationExecutionClient,
    UnavailableSystemApplicationExecutionClient,
};
pub use application_execution_facade::SystemApplicationExecutionFacadeExt;
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
pub use builder::AgentBuilder;
pub use config::AgentConfig;
pub use domain_pack_bridge::{
    compose_installed_domain_pack_catalog, empty_domain_pack_catalog, expand_service_capabilities,
    AppPackPolicyOverride, AppServiceContractConfig, AppServiceContractSpec,
    AppServicePolicyOverride, DomainPackCatalog, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDefinitionSpec, DomainPackDiagnostics, DomainPackHierarchySpec,
    DomainPackIdentitySpec, DomainPackMetadata, DomainPackPolicyTemplate, DomainPackSdkMetadata,
    DomainPackStability, EffectiveServiceCapabilities, InMemoryDomainPackCatalog,
    SharedDomainPackCatalog,
};
pub use driver_client::{
    ServiceBackedDriverClient, SystemDriverClient, UnavailableSystemDriverClient,
};
pub use entitlement_client::{
    ServiceBackedEntitlementClient, SystemEntitlementClient, UnavailableSystemEntitlementClient,
};
pub use evm_client::{ServiceBackedEvmClient, SystemEvmClient, UnavailableSystemEvmClient};
pub use facade::{AgentRegistryApi, MacacaSdk};
pub use heartbeat_client::{
    ServiceBackedHeartbeatClient, SystemHeartbeatClient, UnavailableSystemHeartbeatClient,
};
pub use interaction_client::{
    ServiceBackedInteractionClient, SystemInteractionClient, UnavailableSystemInteractionClient,
};
pub use llm_client::{
    llm_service_chat_client_from_system, ServiceBackedLlmClient, SystemLlmClient,
    UnavailableSystemLlmClient,
};
pub use macaca_proto::{
    driver_service_descriptor, DriverInventoryCommand, DriverLoadServiceCommand, DriverLoadStatus,
    DriverServiceScope, DriverToolCatalogCommand, DRIVER_SERVICE_ID,
};
pub use macaca_proto::{
    Alert, AlertSeverity, SkillCatalogEntryView, TaskServiceSnapshot, TaskServiceSnapshotCommand,
};
pub use macaca_proto::{
    MemoryCapabilitySet, MemoryForgetCommand, MemoryGetCommand, MemoryGetResult, MemoryPolicyHints,
    MemoryPrefetchCommand, MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand,
    MemoryRememberResult, MemoryScope, MemoryServiceSnapshot, MemoryServiceSnapshotCommand,
    MemoryStatusCommand, MemoryStatusReport, MemoryVisibility,
};
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
pub use scheduled_agent_task_client::{
    ServiceBackedScheduledAgentTaskClient, SystemScheduledAgentTaskClient,
    UnavailableSystemScheduledAgentTaskClient,
};
pub use scheduler_client::{
    ServiceBackedSchedulerClient, SystemSchedulerClient, UnavailableSystemSchedulerClient,
};
pub use service_client::{ServiceCallCommand, ServiceCallResult, ServiceInspectionResult};
pub use spec::{AgentSpec, AgentSpecBuilder, TracePolicy};
pub use store_client::{ServiceBackedStoreClient, SystemStoreClient, UnavailableSystemStoreClient};
pub use system_facade::{
    ApprovalDecisionCommand, EmptySystemPackageClient, EmptySystemTraceClient,
    PackageInspectionCommand, PackageInspectionResult, ServiceBackedTaskBoardDataSource,
    ServiceInspectionCommand, SessionEventQueryCommand, StaticSystemStatusClient, SystemFacade,
    SystemPackageClient, SystemServiceClient, SystemStatusClient, SystemStatusSnapshot,
    SystemTaskClient, SystemTraceClient, TaskBoardDataSource, TaskBoardQueryCommand,
    TaskBoardQueryResult, TraceQueryResult, TraceTailCommand, UnavailableSystemServiceClient,
};
pub use task_client::{TaskServiceClient, UnavailableTaskServiceClient};
pub use tool_client::{ServiceBackedToolClient, SystemToolClient, UnavailableSystemToolClient};
pub use validation::{SdkValidationChain, SdkValidator};
pub use web3_client::{ServiceBackedWeb3Client, SystemWeb3Client, UnavailableSystemWeb3Client};
pub use workbench_client::{
    is_structured_unavailable, ServiceBackedWorkbenchClient, SystemWorkbenchClient,
    SystemWorkbenchFacadeExt, UnavailableWorkbenchServiceClient, WorkbenchClientCatalog,
};
