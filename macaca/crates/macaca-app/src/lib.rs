//! `aos-app` — three-layer application model for Agent OS.
//!
//! Supports native (L1), WASM (L2, stub), and declarative (L3) application
//! layers. Apps are loaded from manifest files and managed by the [`AppRuntime`].

pub mod consumption;
pub mod llm_proxy;
pub mod loader;
pub mod model;
pub mod registry;
pub mod runtime;
pub mod skills;
pub mod workflow;

#[allow(deprecated)]
pub use consumption::{
    app_agent_base_prompt, app_agent_manifest_view, app_agent_prompt_semantics,
    app_entry_agent_name, app_entry_agent_name_or, app_entry_workflow_name,
    app_task_planning_contract, discovered_app_agent_names, discovered_app_runtime_builder,
    legacy_app_task_planning_contract, AppAgentManifestView, AppAgentPromptSemantics,
    AppPlanningAgentProfile, AppTaskPlanningContract, AppToolPolicyView,
};
pub use llm_proxy::{LlmProxy, UserLlmOverride};
pub use loader::AppLoader;
pub use model::{
    AppCapabilityNode, AppCapabilitySet, AppCapabilitySource, AppLayer, AppLlmConfig, AppManifest,
    AppStatus,
};
pub use registry::{AppRegistry, DiscoveredApp, DEFAULT_APP, STANDARD_APP_DIRS};
pub use runtime::{
    AppRuntime, AppRuntimeBuilder, ApplicationRuntimeFactory, DefaultApplicationRuntimeFactory,
};
pub use skills::{global_skills_dir, SkillLoader};
pub use workflow::{
    DefaultWorkflowPromptStrategy, WorkflowContext, WorkflowEngine, WorkflowPromptContext,
    WorkflowPromptParts, WorkflowPromptStrategy, WorkflowResult, DEFAULT_COORDINATOR,
    DEFAULT_WORKFLOW,
};
