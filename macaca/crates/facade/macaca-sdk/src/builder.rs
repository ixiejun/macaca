//! Fluent builder for provider-neutral agent declarations.

use macaca_proto::{AgentId, MacacaResult};

use crate::config::AgentConfig;
use crate::spec::AgentSpec;

/// Fluent builder for constructing an [`AgentSpec`] from an [`AgentConfig`].
pub struct AgentBuilder {
    config: AgentConfig,
    id: Option<AgentId>,
}

impl AgentBuilder {
    /// Start building from a parsed config.
    pub fn from_config(config: AgentConfig) -> Self {
        Self { config, id: None }
    }

    /// Override the agent id (useful for testing).
    pub fn with_id(mut self, id: AgentId) -> Self {
        self.id = Some(id);
        self
    }

    /// Override the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    /// Override the prompt template.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.prompt_template = prompt.into();
        self
    }

    /// Build the SDK declaration product.
    pub fn build_spec(self) -> MacacaResult<AgentSpec> {
        let mut builder = crate::spec::AgentSpecBuilder::from_config(self.config);
        if let Some(id) = self.id {
            builder = builder.with_id(id);
        }
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::AgentState;

    fn sample_config() -> AgentConfig {
        AgentConfig::from_yaml(
            r#"
name: test-agent
capabilities:
  - name: test_cap
    description: A test capability
prompt_template: "You are a test agent."
model: mock-model
"#,
        )
        .unwrap()
    }

    #[test]
    fn builder_builds_agent() {
        let spec = AgentBuilder::from_config(sample_config())
            .build_spec()
            .unwrap();
        assert_eq!(spec.name(), "test-agent");
        assert_eq!(spec.capabilities().len(), 1);
        assert_eq!(spec.manifest().state, AgentState::Created);
        assert_eq!(spec.llm_options().model, "mock-model");
    }

    #[test]
    fn builder_with_id_override() {
        let id = AgentId::new();
        let spec = AgentBuilder::from_config(sample_config())
            .with_id(id)
            .build_spec()
            .unwrap();
        assert_eq!(spec.id(), id);
    }

    #[test]
    fn builder_with_model_override() {
        let spec = AgentBuilder::from_config(sample_config())
            .with_model("gpt-4o")
            .build_spec()
            .unwrap();
        assert_eq!(spec.llm_options().model, "gpt-4o");
    }

    #[test]
    fn builder_with_prompt_override() {
        let spec = AgentBuilder::from_config(sample_config())
            .with_prompt("Custom prompt")
            .build_spec()
            .unwrap();
        // The declaration keeps identity stable while replacing only the prompt template.
        assert_eq!(spec.name(), "test-agent");
        assert_eq!(spec.prompt_template(), "Custom prompt");
    }

    #[test]
    fn build_spec_produces_manifest() {
        let spec = AgentBuilder::from_config(sample_config())
            .build_spec()
            .unwrap();
        let manifest = spec.manifest();
        assert_eq!(manifest.id, spec.id());
        assert_eq!(manifest.name, "test-agent");
        assert_eq!(manifest.capabilities.len(), 1);
    }

    #[test]
    fn build_invalid_config_fails() {
        let mut config = sample_config();
        config.name = String::new();
        let err = AgentBuilder::from_config(config).build_spec().unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn sdk_does_not_build_runtime_agent() {
        let spec = AgentBuilder::from_config(sample_config())
            .build_spec()
            .unwrap();

        // The SDK product is a protocol declaration only; runtime providers decide
        // how to materialize executable agents behind service policy and tracing.
        assert_eq!(spec.prompt_template(), "You are a test agent.");
        assert_eq!(spec.manifest().name, "test-agent");
    }
}
