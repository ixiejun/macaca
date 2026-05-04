//! SDK agent declaration product.

use chrono::Utc;
use macaca_proto::{
    AgentId, AgentManifest, AgentState, Capability, LlmOptions, MacacaResult, Permission,
};

use crate::builder::DeclarativeAgent;
use crate::config::AgentConfig;

/// Trace behavior required for SDK-built agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracePolicy {
    /// Agent activity must be traceable by the runtime that executes it.
    Required,
}

impl Default for TracePolicy {
    fn default() -> Self {
        Self::Required
    }
}

/// Stable SDK product for a declarative agent before runtime registration.
#[derive(Debug, Clone)]
pub struct AgentSpec {
    id: AgentId,
    name: String,
    capabilities: Vec<Capability>,
    permission: Permission,
    prompt_template: String,
    llm_options: LlmOptions,
    trace_policy: TracePolicy,
    state: AgentState,
}

impl AgentSpec {
    /// Build an agent spec from declarative config.
    pub fn from_config(config: AgentConfig) -> MacacaResult<Self> {
        AgentSpecBuilder::from_config(config).build()
    }

    /// Agent id.
    pub fn id(&self) -> AgentId {
        self.id
    }

    /// Agent name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Declared capabilities.
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// Permission descriptor.
    pub fn permission(&self) -> &Permission {
        &self.permission
    }

    /// Prompt template.
    pub fn prompt_template(&self) -> &str {
        &self.prompt_template
    }

    /// LLM options.
    pub fn llm_options(&self) -> &LlmOptions {
        &self.llm_options
    }

    /// Trace policy metadata.
    pub fn trace_policy(&self) -> TracePolicy {
        self.trace_policy
    }

    /// Produce the manifest used for kernel registration.
    pub fn manifest(&self) -> AgentManifest {
        AgentManifest {
            id: self.id,
            name: self.name.clone(),
            capabilities: self.capabilities.clone(),
            permission: self.permission.clone(),
            state: self.state,
            created_at: Utc::now(),
            model: self.llm_options.model.clone(),
        }
    }

    /// Convert this declaration into the current runtime agent.
    pub fn into_agent(self) -> DeclarativeAgent {
        DeclarativeAgent::from_spec(self)
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        AgentId,
        String,
        Vec<Capability>,
        Permission,
        String,
        LlmOptions,
        AgentState,
    ) {
        (
            self.id,
            self.name,
            self.capabilities,
            self.permission,
            self.prompt_template,
            self.llm_options,
            self.state,
        )
    }
}

/// Builder for [`AgentSpec`].
pub struct AgentSpecBuilder {
    config: AgentConfig,
    id: Option<AgentId>,
    trace_policy: TracePolicy,
}

impl AgentSpecBuilder {
    /// Start building from a parsed config.
    pub fn from_config(config: AgentConfig) -> Self {
        Self {
            config,
            id: None,
            trace_policy: TracePolicy::Required,
        }
    }

    /// Override the agent id.
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

    /// Override trace policy metadata.
    pub fn with_trace_policy(mut self, trace_policy: TracePolicy) -> Self {
        self.trace_policy = trace_policy;
        self
    }

    /// Build the spec.
    pub fn build(self) -> MacacaResult<AgentSpec> {
        self.config.validate()?;

        let capabilities = self
            .config
            .capabilities
            .iter()
            .map(|c| Capability {
                name: c.name.clone(),
                description: c.description.clone(),
            })
            .collect();

        let permission = Permission {
            level: self.config.resolved_permission_level(),
            allowed_tools: self.config.allowed_tools.clone(),
            allowed_paths: self.config.allowed_paths.clone(),
            network_access: self.config.network_access,
        };

        let llm_options = LlmOptions {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stop_sequences: Vec::new(),
            tools: None,
        };

        Ok(AgentSpec {
            id: self.id.unwrap_or_default(),
            name: self.config.name.clone(),
            capabilities,
            permission,
            prompt_template: self.config.prompt_template.clone(),
            llm_options,
            trace_policy: self.trace_policy,
            state: AgentState::Created,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> AgentConfig {
        AgentConfig::from_yaml(
            r#"
name: spec-agent
capabilities:
  - name: code_gen
    description: Generates code
permission_level: system
allowed_tools:
  - file_read
allowed_paths:
  - /tmp
network_access: true
prompt_template: "You are a spec agent."
model: mock-model
max_tokens: 128
temperature: 0.4
"#,
        )
        .unwrap()
    }

    #[test]
    fn agent_spec_from_config_preserves_fields() {
        let spec = AgentSpec::from_config(sample_config()).unwrap();

        assert_eq!(spec.name(), "spec-agent");
        assert_eq!(spec.capabilities().len(), 1);
        assert_eq!(spec.capabilities()[0].name, "code_gen");
        assert_eq!(spec.permission().allowed_tools, vec!["file_read"]);
        assert_eq!(spec.permission().allowed_paths, vec!["/tmp"]);
        assert!(spec.permission().network_access);
        assert_eq!(spec.prompt_template(), "You are a spec agent.");
        assert_eq!(spec.llm_options().model, "mock-model");
        assert_eq!(spec.llm_options().max_tokens, Some(128));
        assert_eq!(spec.llm_options().temperature, Some(0.4));
        assert_eq!(spec.trace_policy(), TracePolicy::Required);
    }

    #[test]
    fn manifest_matches_spec() {
        let spec = AgentSpec::from_config(sample_config()).unwrap();
        let manifest = spec.manifest();

        assert_eq!(manifest.id, spec.id());
        assert_eq!(manifest.name, spec.name());
        assert_eq!(manifest.capabilities.len(), spec.capabilities().len());
        assert_eq!(manifest.capabilities[0].name, spec.capabilities()[0].name);
        assert_eq!(
            manifest.capabilities[0].description,
            spec.capabilities()[0].description
        );
        assert_eq!(manifest.permission.level, spec.permission().level);
        assert_eq!(
            manifest.permission.allowed_tools,
            spec.permission().allowed_tools
        );
        assert_eq!(
            manifest.permission.allowed_paths,
            spec.permission().allowed_paths
        );
        assert_eq!(
            manifest.permission.network_access,
            spec.permission().network_access
        );
        assert_eq!(manifest.state, AgentState::Created);
        assert_eq!(manifest.model, spec.llm_options().model);
    }
}
