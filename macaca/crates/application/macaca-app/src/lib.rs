//! `aos-app` — three-layer application model for Agent OS.
//!
//! Supports native (L1), WASM (L2, stub), and declarative (L3) application
//! layers. Apps are loaded from manifest files and managed by the [`AppRuntime`].

pub mod abi;
pub mod certification;
pub mod commercial_package;
pub mod conformance_checker;
pub mod consumption;
pub mod dapp_capability;
pub mod domain_pack_catalog;
pub mod genui;
pub mod host;
pub mod lifecycle;
pub mod llm_proxy;
pub mod loader;
pub mod manifest_v1;
pub mod model;
pub mod package;
pub mod package_loader;
pub mod registry;
pub mod runtime;
pub mod runtime_guard;
pub mod service_adapter;
pub mod service_admission;
pub mod service_capability;
pub mod service_projection;
pub mod skills;
pub mod ui_runtime;
pub mod wasm;
pub mod web3_capability;
pub mod workflow;

pub use abi::{
    is_runtime_unavailable, ApplicationAbiAdapter, ApplicationAbiDescriptor,
    ApplicationAbiInstance, ApplicationAbiLoadResult, MetadataOnlyApplicationAbiInstance,
    WasmApplicationAbiAdapter, YamlApplicationAbiAdapter,
};
pub use certification::{
    ApplicationCertificationContext, ApplicationCertificationDiagnostic,
    ApplicationCertificationFixture, ApplicationCertificationKit, ApplicationCertificationReport,
    ApplicationCertificationStatus,
};
pub use commercial_package::{
    is_commercial_package, AppCapabilityCallContext, ApplicationEntitlementAuthorizer,
    CommercialPackageGuard,
};
pub use conformance_checker::{
    ConformanceDiagnostic, ConformanceHostContext, ConformanceReport, ConformanceSeverity,
    ConformanceStatus, ConformanceTraceEvent, PackageConformanceChecker,
};
pub use consumption::{
    app_agent_manifest_view, app_agent_prompt_semantics, app_entry_agent_name,
    app_entry_workflow_name, app_task_planning_contract, discovered_app_agent_names,
    discovered_app_runtime_builder, AppAgentManifestView, AppAgentPromptSemantics,
    AppPlanningAgentProfile, AppTaskPlanningContract, AppToolPolicyView,
};
pub use dapp_capability::AppDappCapabilityRequest;
pub use domain_pack_catalog::{
    compose_installed_domain_pack_catalog, empty_domain_pack_catalog, SharedDomainPackCatalog,
};
pub use genui::{text_component, GenUiResult, GenUiRuntime, UiIntentValidator};
pub use host::{
    is_unavailable as is_application_host_unavailable, ApplicationHost, ApplicationHostBackend,
    UnavailableApplicationHostBackend,
};
pub use lifecycle::ApplicationLifecycle;
pub use llm_proxy::{LlmProxy, UserLlmOverride};
pub use loader::AppLoader;
pub use manifest_v1::{
    YamlApplicationManifestAdapter, YamlApplicationManifestProjection, YamlProjectionDiagnostic,
    YamlToApplicationManifestV1Report,
};
pub use model::{
    AppCapabilityNode, AppCapabilitySet, AppCapabilitySource, AppLayer, AppLlmConfig, AppManifest,
    AppStatus,
};
pub use package::{
    application_manifest_v1_to_package_descriptor, load_yaml_app_package_descriptor,
    AppPackageDescriptorBuilder,
};
pub use package_loader::{PackageLoaderFactory, PackageLoaderKind};
pub use registry::{AppRegistry, DiscoveredApp, DEFAULT_APP, STANDARD_APP_DIRS};
pub use runtime::{
    AppRuntime, AppRuntimeBuilder, ApplicationRuntimeFactory, DefaultApplicationRuntimeFactory,
};
pub use runtime_guard::{
    package_requires_wasm_runtime, PackageGuardContext, PackageGuardTraceEvent, PackageRuntimeGuard,
};
pub use service_adapter::application_service_descriptor;
pub use service_admission::{
    app_status_from_lifecycle, lifecycle_from_app_status, ApplicationAbilitySpec,
    ApplicationAdmissionDiagnostic, ApplicationAdmissionReport, ApplicationManifestSpec,
    ApplicationManifestV1Spec, ApplicationRuntimeKindSpec, ApplicationScopeSpec,
    ApplicationTraceSpec,
};
pub use service_capability::{
    expand_service_capabilities, AppServiceContractConfig, AppServicePolicyOverride,
    DomainPackCatalog, DomainPackDefinition, EffectiveServiceCapabilities,
    InMemoryDomainPackCatalog,
};
pub use service_projection::{
    app_manifest_to_heartbeat_agent_views, app_manifest_to_metadata_view,
    app_manifest_to_metadata_view_with_catalog, app_manifest_to_service_app_view,
    app_manifest_to_service_app_view_with_catalog,
};
pub use skills::{global_skills_dir, SkillLoader};
pub use ui_runtime::*;
pub use wasm::WasmComponentApplicationDescriptor;
pub use web3_capability::AppWeb3CapabilityRequest;
pub use workflow::{
    DefaultWorkflowPromptStrategy, WorkflowEngine, WorkflowPromptContext, WorkflowPromptParts,
    WorkflowPromptStrategy, DEFAULT_WORKFLOW,
};
