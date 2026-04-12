//! Application model types: layers, manifests, and status.

use serde::{Deserialize, Serialize};

use macaca_proto::{AgentId, ApplicationId};

/// The execution layer of an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppLayer {
    /// L1: native Rust agents compiled into the binary.
    L1Native,
    /// L2: WASM-based agents (not yet supported).
    L2Wasm,
    /// L3: declarative agents loaded from YAML/TOML config files.
    L3Declarative,
}

/// Status of a loaded application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppStatus {
    /// App manifest has been loaded but agents are not yet started.
    Loaded,
    /// App agents are running.
    Running,
    /// App has been stopped.
    Stopped,
    /// App failed to start or encountered an error.
    Failed,
}

/// Default LLM configuration declared by an application.
///
/// **Security note:** This does NOT contain API keys. Apps declare which
/// provider/model they prefer; the kernel resolves the actual credentials
/// from the user's configuration. Apps never see API keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLlmConfig {
    /// LLM provider name (e.g., "openai", "anthropic", "dashscope").
    pub provider: String,
    /// Model name (e.g., "gpt-4", "claude-sonnet-4-20250514").
    pub model: String,
}

/// An inline agent configuration within an app manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineAgentConfig {
    /// Human-readable agent name.
    pub name: String,
    /// Capability names.
    #[serde(default)]
    pub capabilities: Vec<CapabilityRef>,
    /// System prompt.
    #[serde(default)]
    pub prompt_template: String,
    /// LLM model name.
    #[serde(default = "default_model")]
    pub model: String,
    /// Permission level.
    #[serde(default = "default_permission")]
    pub permission_level: String,
    /// Allowed tools.
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Max tokens.
    pub max_tokens: Option<u32>,
    /// Temperature.
    pub temperature: Option<f32>,
}

/// A capability reference in inline config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRef {
    /// Capability name.
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
}

fn default_model() -> String {
    "gpt-4".into()
}

fn default_permission() -> String {
    "user".into()
}

/// Source of agent configuration — either a file path or inline config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentSource {
    /// Path to a YAML/TOML agent config file.
    FilePath(String),
    /// Inline agent configuration.
    Inline(InlineAgentConfig),
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
}

/// UI type for frontend rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiType {
    /// Chat interface (default).
    Chat,
    /// Form-based interface.
    Form,
    /// Dashboard interface.
    Dashboard,
    /// Custom interface (frontend handles).
    Custom,
}

impl Default for UiType {
    fn default() -> Self {
        Self::Chat
    }
}

fn default_version() -> String {
    "0.1.0".into()
}

// ---------------------------------------------------------------------------
// Entrypoint and Workflow Configuration
// ---------------------------------------------------------------------------

/// Entry point type for an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrypointType {
    /// Execute a named workflow.
    Workflow,
    /// Invoke a specific agent directly.
    Agent,
    /// Custom entry point (future extensibility).
    Custom,
}

impl Default for EntrypointType {
    fn default() -> Self {
        Self::Workflow
    }
}

/// Configuration for the application entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrypointConfig {
    /// Type of entry point.
    #[serde(default, rename = "type")]
    pub type_: EntrypointType,
    /// Name of the workflow or agent to invoke.
    pub name: String,
}

/// A single step in a workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step name/identifier.
    pub name: String,
    /// Agent ID to execute this step.
    pub agent: String,
    /// Optional prompt template file or inline prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_template: Option<String>,
    /// Names of steps that must complete before this step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

/// Definition of a workflow that can be executed by the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Ordered list of steps to execute.
    pub steps: Vec<WorkflowStep>,
}

/// Resource path configuration for an application.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceConfig {
    /// Path to personas directory (relative to app root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personas: Option<String>,
    /// Path to skills directory (relative to app root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<String>,
    /// Path to prompts directory (relative to app root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<String>,
    /// Path to workflows directory (relative to app root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflows: Option<String>,
}

/// A loaded application with its associated agent ids and status.
#[derive(Debug, Clone)]
pub struct LoadedApp {
    /// The parsed manifest.
    pub manifest: AppManifest,
    /// Ids of agents registered in the kernel for this app.
    pub agent_ids: Vec<AgentId>,
    /// Current status.
    pub status: AppStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_layer_serialization_roundtrip() {
        let layer = AppLayer::L3Declarative;
        let json = serde_json::to_string(&layer).unwrap();
        let parsed: AppLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, AppLayer::L3Declarative);
    }

    #[test]
    fn app_status_values() {
        assert_ne!(AppStatus::Loaded, AppStatus::Running);
        assert_ne!(AppStatus::Running, AppStatus::Stopped);
        assert_ne!(AppStatus::Stopped, AppStatus::Failed);
    }

    #[test]
    fn app_manifest_from_yaml() {
        let yaml = r#"
name: my-app
version: "1.0.0"
layer: L3Declarative
agents:
  - name: helper
    prompt_template: "You help."
    capabilities:
      - name: assist
"#;
        let manifest: AppManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "my-app");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.layer, AppLayer::L3Declarative);
        assert_eq!(manifest.agents.len(), 1);
    }

    #[test]
    fn app_manifest_file_path_agents() {
        let yaml = r#"
name: path-app
layer: L3Declarative
agents:
  - "agents/coder.yaml"
  - "agents/reviewer.yaml"
"#;
        let manifest: AppManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.agents.len(), 2);
        match &manifest.agents[0] {
            AgentSource::FilePath(p) => assert_eq!(p, "agents/coder.yaml"),
            _ => panic!("Expected FilePath"),
        }
    }

    #[test]
    fn loaded_app_defaults() {
        let app = LoadedApp {
            manifest: AppManifest {
                id: ApplicationId::new(),
                name: "test".into(),
                description: None,
                version: "0.1.0".into(),
                layer: AppLayer::L1Native,
                ui_type: None,
                agents: vec![],
                llm_config: None,
                entry_agent: None,
                entrypoint: None,
                workflows: None,
                resources: None,
            },
            agent_ids: vec![],
            status: AppStatus::Loaded,
        };
        assert_eq!(app.status, AppStatus::Loaded);
        assert!(app.agent_ids.is_empty());
    }

    #[test]
    fn entrypoint_config_parsing() {
        let yaml = r#"
name: test-app
layer: L3Declarative
entrypoint:
  type: workflow
  name: sdd
workflows:
  sdd:
    description: Spec-Driven Development
    steps:
      - name: analyze
        agent: architect
        prompt_template: "Analyze: {{input}}"
      - name: execute
        agent: frontend
        depends_on: [analyze]
"#;
        let manifest: AppManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "test-app");

        let entrypoint = manifest.entrypoint.unwrap();
        assert_eq!(entrypoint.type_, EntrypointType::Workflow);
        assert_eq!(entrypoint.name, "sdd");

        let workflows = manifest.workflows.unwrap();
        let sdd = workflows.get("sdd").unwrap();
        assert_eq!(sdd.steps.len(), 2);
        assert_eq!(sdd.steps[1].depends_on, vec!["analyze"]);
    }

    #[test]
    fn resource_config_parsing() {
        let yaml = r#"
name: test-app
layer: L3Declarative
resources:
  personas: personas/
  skills: skills/
  prompts: prompts/
  workflows: workflows/
"#;
        let manifest: AppManifest = serde_yaml::from_str(yaml).unwrap();
        let resources = manifest.resources.unwrap();
        assert_eq!(resources.personas, Some("personas/".into()));
        assert_eq!(resources.skills, Some("skills/".into()));
    }
}
