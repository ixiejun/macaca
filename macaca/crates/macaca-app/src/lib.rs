//! `aos-app` — three-layer application model for Agent OS.
//!
//! Supports native (L1), WASM (L2, stub), and declarative (L3) application
//! layers. Apps are loaded from manifest files and managed by the [`AppRuntime`].

pub mod llm_proxy;
pub mod loader;
pub mod model;
pub mod registry;
pub mod runtime;
pub mod skills;
pub mod workflow;

pub use llm_proxy::{LlmProxy, UserLlmOverride};
pub use loader::AppLoader;
pub use model::{AppLayer, AppLlmConfig, AppManifest, AppStatus};
pub use registry::{AppRegistry, DiscoveredApp, DEFAULT_APP, STANDARD_APP_DIRS};
pub use runtime::AppRuntime;
pub use skills::{global_skills_dir, SkillLoader};
pub use workflow::{
    WorkflowContext, WorkflowEngine, WorkflowResult, DEFAULT_COORDINATOR, DEFAULT_WORKFLOW,
};
