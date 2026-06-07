//! Application model types: layers, manifests, and status.

use serde::{Deserialize, Serialize};

use crate::service_capability::AppServiceContractConfig;
use crate::ui_runtime::AppUiRuntimeConfig;
use macaca_proto::{
    AgentId, ApplicationExecutionProfileDeclaration, ApplicationId,
    ApplicationWorkbenchManifestDeclaration, ExecutionControlPolicy,
};

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
    /// Optional standard AgentSkills visibility policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<AgentSkillsConfig>,
    /// Optional context engine override for this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_engine: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppContextConfig {
    /// Optional default context engine for this application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// Optional fallback engine for this application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_engine: Option<String>,
    /// Optional override for workspace guide files (priorities and per-file byte budgets).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_guides: Option<macaca_proto::config::WorkspaceGuideSourcesConfig>,
    /// Optional inject of Markdown profile candidates (`AGENTS.md`, etc.) through the composer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_profile: Option<macaca_proto::config::AgentProfileContextConfig>,
}

/// Per-agent standard AgentSkills visibility policy in app manifests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSkillsConfig {
    /// If set, only these skill names are visible to this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    /// Skill names hidden from this agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
}

/// Application-owned autonomy declarations.
///
/// This configuration is intentionally data-only. It declares which generic
/// autonomy surfaces an application opts into, while runtime-host and system
/// services still own policy, trace, scheduling, heartbeat, and execution
/// semantics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppAutonomyConfig {
    /// Optional heartbeat agent participation contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heartbeat: Option<AppHeartbeatConfig>,
}

/// Application-owned heartbeat participation contract.
///
/// The presence of this block does not execute anything by itself. Runtime-host
/// must query the sanitized Application Service projection and then dispatch
/// through Agent Execution, so app packages cannot bypass policy or service
/// audit boundaries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppHeartbeatConfig {
    /// Disable all heartbeat agent dispatch while preserving declarations.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Manifest-declared agents that may receive heartbeat execution intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<AppHeartbeatAgentConfig>,
}

/// One manifest-declared heartbeat agent.
///
/// `profile_id` is a provider-neutral extension point. The first runtime bridge
/// records it for traceability without adding application-specific profile
/// behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppHeartbeatAgentConfig {
    /// Agent name as declared in this application's manifest.
    pub name: String,
    /// Per-agent enable switch for staged rollout and package defaults.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Provider-neutral profile selector recorded in sanitized dispatch views.
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
    /// Optional per-agent cadence policy.
    ///
    /// This policy is declarative only. Runtime-host translates it into a
    /// Heartbeat-owned native profile, while service.heartbeat remains the
    /// owner of cadence evaluation, gate decisions, and run mementos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cadence: Option<AppHeartbeatCadenceConfig>,
    /// Optional per-agent gate policy.
    ///
    /// The first gate exposed to manifests is cooldown because it determines
    /// how often an accepted wake may run after the fixed cadence becomes due.
    /// Missing values preserve the provider default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gates: Option<AppHeartbeatGateConfig>,
    /// Bounded declaration metadata. Projection code sanitizes values before
    /// they cross service boundaries.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub metadata: std::collections::BTreeMap<String, String>,
}

/// Per-agent heartbeat cadence policy declared by an application.
///
/// The shape is intentionally small and provider-neutral. Future cadence
/// variants can extend this object without making Scheduler own heartbeat
/// timing or forcing Web shells to understand raw manifest semantics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppHeartbeatCadenceConfig {
    /// Fixed interval in seconds for this agent's native Heartbeat profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_interval_secs: Option<u64>,
}

/// Per-agent heartbeat gate policy declared by an application.
///
/// Gates are hints for Heartbeat-owned provider policy. They are not direct
/// execution instructions and must remain generic across all application types.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppHeartbeatGateConfig {
    /// Optional cooldown in seconds after an accepted wake for this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_secs: Option<u64>,
}

fn default_true() -> bool {
    true
}

fn default_profile_id() -> String {
    "default".into()
}

/// A capability reference in inline config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRef {
    /// Capability name.
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
}

/// Internal source of application capability information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCapabilitySource {
    Manifest,
    Skill,
    Driver,
    ToolPolicy,
    Provider,
}

/// Internal composite capability node for application-level capability tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCapabilityNode {
    Leaf(CapabilityRef),
    Group {
        source: AppCapabilitySource,
        children: Vec<AppCapabilityNode>,
    },
}

impl AppCapabilityNode {
    fn flatten_into<'a>(&'a self, output: &mut Vec<&'a CapabilityRef>) {
        match self {
            Self::Leaf(capability) => output.push(capability),
            Self::Group { children, .. } => {
                for child in children {
                    child.flatten_into(output);
                }
            }
        }
    }
}

/// Composite capability set for applications. Callers can flatten back to the
/// legacy capability list while preserving source information internally.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppCapabilitySet {
    root: Vec<AppCapabilityNode>,
}

impl AppCapabilitySet {
    /// Construct a manifest-backed capability set from inline agent definitions.
    pub fn from_manifest_agents(agents: &[AgentSource]) -> Self {
        let mut root = Vec::new();
        for source in agents {
            if let AgentSource::Inline(inline) = source {
                if inline.capabilities.is_empty() {
                    continue;
                }
                root.push(AppCapabilityNode::Group {
                    source: AppCapabilitySource::Manifest,
                    children: inline
                        .capabilities
                        .iter()
                        .cloned()
                        .map(AppCapabilityNode::Leaf)
                        .collect(),
                });
            }
        }
        Self { root }
    }

    /// Add a capability group from a specific source.
    pub fn push_group(&mut self, source: AppCapabilitySource, capabilities: Vec<CapabilityRef>) {
        if capabilities.is_empty() {
            return;
        }
        self.root.push(AppCapabilityNode::Group {
            source,
            children: capabilities
                .into_iter()
                .map(AppCapabilityNode::Leaf)
                .collect(),
        });
    }

    /// Flatten all capabilities back into the legacy output format.
    pub fn flatten_legacy(&self) -> Vec<CapabilityRef> {
        let mut refs = Vec::new();
        for node in &self.root {
            node.flatten_into(&mut refs);
        }
        refs.into_iter().cloned().collect()
    }

    /// Returns the internal root nodes for structured callers.
    pub fn roots(&self) -> &[AppCapabilityNode] {
        &self.root
    }
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
    ///
    /// This is the Application Framework-owned contract that lets YAML, WASM,
    /// GenUI, headless, and workbench-style applications request generic
    /// substrate services. It is declarative only: service clients, policy,
    /// sandboxing, plugin/MCP/skill lifecycle, and app protocol still own the
    /// runtime effects and audit chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workbench: Option<ApplicationWorkbenchManifestDeclaration>,
    /// Optional application-owned autonomy declaration block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autonomy: Option<AppAutonomyConfig>,
    /// Optional application-owned UI runtime declaration.
    ///
    /// The declaration points to UI artifacts inside the application package
    /// and lists host bridge capabilities. It is data-only so Web/Desktop
    /// shells can host app-owned UI without learning app-specific business
    /// logic or rendering rules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<AppUiRuntimeConfig>,
    /// Optional provider-neutral execution-control policy for this application.
    ///
    /// Applications declare pause/resume triggers and resume sources here so the
    /// OS can resolve `service.execution_control` without hard-coded intent
    /// branches.  The YAML adapter projects this field into Manifest v1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_control: Option<ExecutionControlPolicy>,
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
                context: None,
                service_contract: None,
                execution_profile: None,
                workbench: None,
                autonomy: None,
                ui: None,
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

    #[test]
    fn app_capability_set_flattens_manifest_capabilities() {
        let agents = vec![AgentSource::Inline(InlineAgentConfig {
            name: "a".into(),
            capabilities: vec![
                CapabilityRef {
                    name: "design".into(),
                    description: "design cap".into(),
                },
                CapabilityRef {
                    name: "build".into(),
                    description: "build cap".into(),
                },
            ],
            prompt_template: String::new(),
            model: default_model(),
            permission_level: default_permission(),
            allowed_tools: vec![],
            max_tokens: None,
            temperature: None,
            skills: None,
            context_engine: None,
        })];

        let set = AppCapabilitySet::from_manifest_agents(&agents);
        let flattened = set.flatten_legacy();
        assert_eq!(flattened.len(), 2);
        assert_eq!(flattened[0].name, "design");
        assert_eq!(flattened[1].name, "build");
    }

    #[test]
    fn app_capability_set_preserves_visible_capability_count_across_sources() {
        let agents = vec![AgentSource::Inline(InlineAgentConfig {
            name: "a".into(),
            capabilities: vec![CapabilityRef {
                name: "design".into(),
                description: "design cap".into(),
            }],
            prompt_template: String::new(),
            model: default_model(),
            permission_level: default_permission(),
            allowed_tools: vec![],
            max_tokens: None,
            temperature: None,
            skills: None,
            context_engine: None,
        })];

        let mut set = AppCapabilitySet::from_manifest_agents(&agents);
        set.push_group(
            AppCapabilitySource::Driver,
            vec![CapabilityRef {
                name: "opencode".into(),
                description: "driver capability".into(),
            }],
        );
        set.push_group(
            AppCapabilitySource::Skill,
            vec![CapabilityRef {
                name: "playwright_search".into(),
                description: "skill capability".into(),
            }],
        );

        assert_eq!(set.roots().len(), 3);
        let flattened = set.flatten_legacy();
        assert_eq!(flattened.len(), 3);
        assert_eq!(
            flattened
                .iter()
                .map(|cap| cap.name.as_str())
                .collect::<Vec<_>>(),
            vec!["design", "opencode", "playwright_search"]
        );
    }
}
