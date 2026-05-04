//! Declarative agent configuration parsing from YAML or TOML.

use serde::{Deserialize, Serialize};
use std::path::Path;

use macaca_proto::{MacacaError, MacacaResult, PermissionLevel};

use crate::validation::SdkValidationChain;

/// Declarative configuration for an agent, loadable from YAML or TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Human-readable agent name (required).
    pub name: String,

    /// List of capability names this agent declares.
    #[serde(default)]
    pub capabilities: Vec<CapabilityDef>,

    /// Permission level — `system` or `user`. Defaults to `user`.
    #[serde(default = "default_permission_level")]
    pub permission_level: String,

    /// Allowed tool names for this agent.
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Allowed filesystem paths.
    #[serde(default)]
    pub allowed_paths: Vec<String>,

    /// Whether network access is permitted.
    #[serde(default)]
    pub network_access: bool,

    /// System prompt / prompt template for the declarative agent.
    #[serde(default)]
    pub prompt_template: String,

    /// Preferred LLM model name (e.g. "gpt-4o", "claude-3-opus").
    #[serde(default = "default_model")]
    pub model: String,

    /// Max tokens for LLM responses.
    pub max_tokens: Option<u32>,

    /// Temperature for LLM sampling.
    pub temperature: Option<f32>,

    /// Directory containing persona Markdown files (IDENTITY.md, SOUL.md, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona_dir: Option<String>,

    /// Optional standard AgentSkills visibility policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<AgentSkillsConfig>,
}

/// Per-agent standard AgentSkills visibility policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSkillsConfig {
    /// If set, only these skill names are visible to this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    /// Skill names hidden from this agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
}

/// A capability definition within agent config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDef {
    /// Capability name.
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
}

fn default_permission_level() -> String {
    "user".into()
}

fn default_model() -> String {
    "gpt-4".into()
}

impl AgentConfig {
    /// Parse an `AgentConfig` from a YAML string.
    pub fn from_yaml(yaml: &str) -> MacacaResult<Self> {
        let config: Self =
            serde_yaml::from_str(yaml).map_err(|e| MacacaError::Config(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Parse an `AgentConfig` from a TOML string.
    pub fn from_toml(toml_str: &str) -> MacacaResult<Self> {
        let config: Self =
            toml::from_str(toml_str).map_err(|e| MacacaError::Config(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Load an `AgentConfig` from a file. Format is detected by extension
    /// (`.yaml`/`.yml` for YAML, `.toml` for TOML).
    pub fn from_file(path: impl AsRef<Path>) -> MacacaResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "yaml" | "yml" => Self::from_yaml(&content),
            "toml" => Self::from_toml(&content),
            other => Err(MacacaError::Config(format!(
                "Unsupported config file extension: '{other}'. Use .yaml, .yml, or .toml"
            ))),
        }
    }

    /// Validate required fields and values.
    pub fn validate(&self) -> MacacaResult<()> {
        SdkValidationChain::default_rules().validate(self)
    }

    /// Resolve the parsed `permission_level` string into a [`PermissionLevel`].
    pub fn resolved_permission_level(&self) -> PermissionLevel {
        match self.permission_level.as_str() {
            "system" => PermissionLevel::System,
            _ => PermissionLevel::User,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_yaml_minimal() {
        let yaml = r#"
name: my-agent
capabilities:
  - name: code_gen
    description: Generates code
prompt_template: "You are a coding assistant."
"#;
        let config = AgentConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.name, "my-agent");
        assert_eq!(config.capabilities.len(), 1);
        assert_eq!(config.capabilities[0].name, "code_gen");
        assert_eq!(config.permission_level, "user");
        assert_eq!(config.model, "gpt-4");
    }

    #[test]
    fn parse_yaml_full() {
        let yaml = r#"
name: system-agent
capabilities:
  - name: planning
    description: Plans tasks
  - name: execution
    description: Executes tasks
permission_level: system
allowed_tools:
  - file_read
  - shell
allowed_paths:
  - /tmp
network_access: true
prompt_template: "You are an OS-level agent."
model: claude-3-opus
max_tokens: 8192
temperature: 0.5
"#;
        let config = AgentConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.name, "system-agent");
        assert_eq!(config.capabilities.len(), 2);
        assert_eq!(config.permission_level, "system");
        assert_eq!(config.resolved_permission_level(), PermissionLevel::System);
        assert_eq!(config.allowed_tools, vec!["file_read", "shell"]);
        assert!(config.network_access);
        assert_eq!(config.model, "claude-3-opus");
        assert_eq!(config.max_tokens, Some(8192));
        assert_eq!(config.temperature, Some(0.5));
    }

    #[test]
    fn parse_toml_config() {
        let toml_str = r#"
name = "toml-agent"
permission_level = "user"
prompt_template = "You help with tasks."
model = "gpt-4o"

[[capabilities]]
name = "search"
description = "Searches the web"
"#;
        let config = AgentConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.name, "toml-agent");
        assert_eq!(config.capabilities[0].name, "search");
        assert_eq!(config.model, "gpt-4o");
    }

    #[test]
    fn empty_name_fails_validation() {
        let yaml = r#"
name: ""
"#;
        let err = AgentConfig::from_yaml(yaml).unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn invalid_permission_level_fails() {
        let yaml = r#"
name: bad-agent
permission_level: admin
"#;
        let err = AgentConfig::from_yaml(yaml).unwrap_err();
        assert!(err.to_string().contains("permission_level"));
    }

    #[test]
    fn empty_capability_name_fails() {
        let yaml = r#"
name: bad-cap
capabilities:
  - name: ""
"#;
        let err = AgentConfig::from_yaml(yaml).unwrap_err();
        assert!(err.to_string().contains("Capability name"));
    }

    #[test]
    fn temperature_out_of_range_fails() {
        let yaml = r#"
name: hot-agent
temperature: 3.0
"#;
        let err = AgentConfig::from_yaml(yaml).unwrap_err();
        assert!(err.to_string().contains("Temperature"));
    }

    #[test]
    fn unsupported_extension_fails() {
        let dir = std::env::temp_dir().join("macaca_sdk_ext_test");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("agent.json");
        std::fs::write(&file, "{}").unwrap();
        let err = AgentConfig::from_file(&file).unwrap_err();
        assert!(err.to_string().contains("Unsupported"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolved_permission_defaults_to_user() {
        let yaml = r#"
name: default-perm
"#;
        let config = AgentConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.resolved_permission_level(), PermissionLevel::User);
    }

    #[test]
    fn persona_dir_optional() {
        let yaml = r#"
name: persona-agent
persona_dir: "./personas/agent-x"
"#;
        let config = AgentConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.persona_dir.as_deref(), Some("./personas/agent-x"));
    }

    #[test]
    fn persona_dir_defaults_to_none() {
        let yaml = r#"
name: no-persona
"#;
        let config = AgentConfig::from_yaml(yaml).unwrap();
        assert!(config.persona_dir.is_none());
    }
}
