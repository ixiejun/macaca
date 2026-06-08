//! Root application manifest aggregate type.
//!
//! `AppManifest` is the **Aggregate Root** for declarative application
//! configuration. It composes layer/status enums, agent sources, workflow
//! graphs, service contracts, and policy blocks into one serde-friendly DTO
//! consumed by loader, projection, and runtime admission surfaces.

use serde::{Deserialize, Serialize};

use crate::service_capability::AppServiceContractConfig;
use crate::ui_runtime::AppUiRuntimeConfig;
use macaca_proto::{
    ApplicationExecutionProfileDeclaration, ApplicationId,
    ApplicationWorkbenchManifestDeclaration, ExecutionControlPolicy,
};

use super::agent_config::{AgentSource, AppContextConfig, AppLlmConfig};
use super::autonomy::AppAutonomyConfig;
use super::core::{AppLayer, UiType};
use super::workflow::{EntrypointConfig, ResourceConfig, WorkflowDefinition};

/// Default semantic version when manifests omit `version`.
pub(super) fn default_version() -> String {
    "0.1.0".into()
}

/// Application manifest describing an app and its agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    /// Unique application ID. Auto-generated if not provided.
    #[serde(default = "ApplicationId::new")]
    pub id: ApplicationId,
    /// Application name.
    pub name: String,
    /// Application description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Application version.
    #[serde(default = "default_version")]
    pub version: String,
    /// Execution layer.
    pub layer: AppLayer,
    /// UI type for frontend rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_type: Option<UiType>,
    /// Agent configurations — file paths or inline definitions.
    #[serde(default)]
    pub agents: Vec<AgentSource>,
    /// Default LLM configuration for this app's agents.
    /// Apps declare preferences; the kernel resolves actual API keys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_config: Option<AppLlmConfig>,
    /// Name of the agent that receives user messages (the "entry" agent).
    /// If not set, falls back to the first agent in the workflow, or the
    /// first agent with `delegate_task` capability, or the first registered agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_agent: Option<String>,
    /// Application entry point configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<EntrypointConfig>,
    /// Named workflow definitions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflows: Option<std::collections::HashMap<String, WorkflowDefinition>>,
    /// Resource path configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceConfig>,
    /// Optional context engine configuration for this application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<AppContextConfig>,
    /// Optional generic service declaration block for contract-driven routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_contract: Option<AppServiceContractConfig>,
    /// Optional provider-neutral application execution profile.
    ///
    /// This profile is declarative only. Application Framework admission and
    /// runtime-host provider adapters validate it before any transport, lease,
    /// EventLog, or control-delivery side effect can occur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_profile: Option<ApplicationExecutionProfileDeclaration>,
    /// Optional generic workbench capability declaration block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workbench: Option<ApplicationWorkbenchManifestDeclaration>,
    /// Optional application-owned autonomy declaration block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autonomy: Option<AppAutonomyConfig>,
    /// Optional application-owned UI runtime declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<AppUiRuntimeConfig>,
    /// Optional provider-neutral execution-control policy for this application.
    ///
    /// Applications declare pause/resume triggers and resume sources here so the
    /// OS can resolve `service.execution_control` without hard-coded intent
    /// branches. The YAML adapter projects this field into Manifest v1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_control: Option<ExecutionControlPolicy>,
}
