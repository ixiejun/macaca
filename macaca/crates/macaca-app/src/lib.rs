//! `aos-app` — three-layer application model for Agent OS.
//!
//! Supports native (L1), WASM (L2, stub), and declarative (L3) application
//! layers. Apps are loaded from manifest files and managed by the [`AppRuntime`].

pub mod model;
pub mod loader;
pub mod runtime;
pub mod llm_proxy;
pub mod registry;
pub mod workflow;
pub mod skills;

pub use model::{AppLayer, AppLlmConfig, AppManifest, AppStatus};
pub use loader::AppLoader;
pub use runtime::AppRuntime;
pub use llm_proxy::{LlmProxy, UserLlmOverride};
pub use registry::{AppRegistry, DiscoveredApp, STANDARD_APP_DIRS, DEFAULT_APP};
pub use workflow::{WorkflowEngine, WorkflowContext, WorkflowResult, DEFAULT_WORKFLOW, DEFAULT_COORDINATOR};
pub use skills::{SkillLoader, global_skills_dir};
