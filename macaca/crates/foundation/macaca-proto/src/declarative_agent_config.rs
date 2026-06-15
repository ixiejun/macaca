//! Foundation contract for declarative agent configuration (YAML/TOML).
//!
//! # Architectural role
//!
//! Agent manifests loaded from application packages are expressed as declarative
//! configuration files. This module owns the **shared data contract** so lower
//! layers (`macaca-app`, `macaca-sdk`) can parse and validate the same shape
//! without creating cyclic Cargo dependencies. The SDK may add product-level
//! builders on top; the proto layer stays transport-agnostic and application-neutral.
//!
//! # Trace and audit
//!
//! Validation and manifest projection emit `tracing` events at decision points so
//! bootstrap failures can be correlated with manifest paths in composition-root logs.

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{
    AgentManifest, AgentState, Capability, MacacaError, MacacaResult, Permission, PermissionLevel,
};

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

    /// Preferred LLM model name (e.g. provider model id).
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

/// Serde placeholder — agent manifests supply the preferred model id at load time.
fn default_model() -> String {
    String::new()
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
        let content =
            std::fs::read_to_string(path).map_err(|e| MacacaError::Config(e.to_string()))?;

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        debug!(
            path = %path.display(),
            extension = %ext,
            "Loading declarative agent config from disk"
        );

        match ext.as_str() {
            "yaml" | "yml" => Self::from_yaml(&content),
            "toml" => Self::from_toml(&content),
            other => Err(MacacaError::Config(format!(
                "Unsupported config file extension: '{other}'. Use .yaml, .yml, or .toml"
            ))),
        }
    }

    /// Validate required fields and values using the default proto rule chain.
    pub fn validate(&self) -> MacacaResult<()> {
        DeclarativeAgentConfigValidation::default_rules().validate(self)
    }

    /// Resolve the parsed `permission_level` string into a [`PermissionLevel`].
    pub fn resolved_permission_level(&self) -> PermissionLevel {
        match self.permission_level.as_str() {
            "system" => PermissionLevel::System,
            _ => PermissionLevel::User,
        }
    }
}

/// Single validator in the declarative agent config validation chain (Chain of Responsibility).
pub trait DeclarativeAgentConfigValidator: Send + Sync {
    /// Validate a config or return a config error.
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()>;
}

/// Ordered validation chain for declarative agent configuration rules.
#[derive(Default)]
pub struct DeclarativeAgentConfigValidation {
    validators: Vec<Box<dyn DeclarativeAgentConfigValidator>>,
}

impl DeclarativeAgentConfigValidation {
    /// Create an empty validation chain.
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    /// Add a validator to the chain.
    pub fn with_validator(
        mut self,
        validator: impl DeclarativeAgentConfigValidator + 'static,
    ) -> Self {
        self.validators.push(Box::new(validator));
        self
    }

    /// Create the default validation chain used by loaders and the SDK facade.
    pub fn default_rules() -> Self {
        Self::new()
            .with_validator(NameValidator)
            .with_validator(PermissionLevelValidator)
            .with_validator(CapabilityNameValidator)
            .with_validator(TemperatureValidator)
    }

    /// Run validators in registration order; first failure short-circuits.
    pub fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        for validator in &self.validators {
            validator.validate(config)?;
        }
        Ok(())
    }
}

struct NameValidator;

impl DeclarativeAgentConfigValidator for NameValidator {
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        if config.name.trim().is_empty() {
            return Err(MacacaError::Config(
                "Agent config 'name' must not be empty".into(),
            ));
        }
        Ok(())
    }
}

struct PermissionLevelValidator;

impl DeclarativeAgentConfigValidator for PermissionLevelValidator {
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        match config.permission_level.as_str() {
            "system" | "user" => Ok(()),
            other => Err(MacacaError::Config(format!(
                "Invalid permission_level '{other}'. Must be 'system' or 'user'"
            ))),
        }
    }
}

struct CapabilityNameValidator;

impl DeclarativeAgentConfigValidator for CapabilityNameValidator {
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        for cap in &config.capabilities {
            if cap.name.trim().is_empty() {
                return Err(MacacaError::Config(
                    "Capability name must not be empty".into(),
                ));
            }
        }
        Ok(())
    }
}

struct TemperatureValidator;

impl DeclarativeAgentConfigValidator for TemperatureValidator {
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        if let Some(temp) = config.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(MacacaError::Config(format!(
                    "Temperature {temp} out of range [0.0, 2.0]"
                )));
            }
        }
        Ok(())
    }
}

/// Project a validated declarative config into a kernel [`AgentManifest`].
///
/// This is the canonical manifest builder for application bootstrap paths that
/// register agents directly with the microkernel (without SDK side registries).
pub fn agent_manifest_from_declarative_config(config: &AgentConfig) -> MacacaResult<AgentManifest> {
    config.validate()?;

    let capabilities = config
        .capabilities
        .iter()
        .map(|c| Capability {
            name: c.name.clone(),
            description: c.description.clone(),
        })
        .collect();

    let permission = Permission {
        level: config.resolved_permission_level(),
        allowed_tools: config.allowed_tools.clone(),
        allowed_paths: config.allowed_paths.clone(),
        network_access: config.network_access,
    };

    let manifest = AgentManifest {
        id: Default::default(),
        name: config.name.clone(),
        capabilities,
        permission,
        state: AgentState::Created,
        created_at: Utc::now(),
        model: config.model.clone(),
    };

    info!(
        agent_name = %manifest.name,
        capability_count = manifest.capabilities.len(),
        permission_level = ?manifest.permission.level,
        "Projected declarative agent config into kernel manifest"
    );

    Ok(manifest)
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
    }

    #[test]
    fn manifest_projection_preserves_permission() {
        let yaml = r#"
name: system-agent
permission_level: system
allowed_tools:
  - file_read
network_access: true
model: mock-model
"#;
        let config = AgentConfig::from_yaml(yaml).unwrap();
        let manifest = agent_manifest_from_declarative_config(&config).unwrap();
        assert_eq!(manifest.name, "system-agent");
        assert_eq!(manifest.permission.level, PermissionLevel::System);
        assert_eq!(manifest.model, "mock-model");
    }

    #[test]
    fn empty_name_fails_validation() {
        let yaml = r#"
name: ""
"#;
        let err = AgentConfig::from_yaml(yaml).unwrap_err();
        assert!(err.to_string().contains("name"));
    }
}
