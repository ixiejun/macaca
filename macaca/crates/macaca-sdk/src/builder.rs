//! Fluent builder and `DeclarativeAgent` implementation.

use async_trait::async_trait;
use chrono::Utc;

use macaca_agent::{Agent, AgentServices};
use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentId, AgentManifest, AgentOutput, AgentState, Capability, LlmMessage, LlmOptions,
    MacacaError, MacacaResult, Permission,
};
use macaca_tools::ToolSet;

use crate::config::AgentConfig;

/// Fluent builder for constructing a [`DeclarativeAgent`] from an [`AgentConfig`].
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

    /// Build the [`DeclarativeAgent`].
    pub fn build(self) -> MacacaResult<DeclarativeAgent> {
        self.config.validate()?;

        let id = self.id.unwrap_or_default();

        let capabilities: Vec<Capability> = self
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

        let options = LlmOptions {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stop_sequences: Vec::new(),
            tools: None,
        };

        Ok(DeclarativeAgent {
            id,
            name: self.config.name.clone(),
            capabilities,
            permission,
            prompt_template: self.config.prompt_template.clone(),
            llm_options: options,
            state: AgentState::Created,
        })
    }

    /// Build and also produce the corresponding [`AgentManifest`].
    pub fn build_with_manifest(self) -> MacacaResult<(DeclarativeAgent, AgentManifest)> {
        let agent = self.build()?;
        let manifest = agent.manifest();
        Ok((agent, manifest))
    }
}

/// An agent constructed from declarative config. Uses its prompt template and
/// the injected LLM provider to generate responses.
#[derive(Debug)]
pub struct DeclarativeAgent {
    id: AgentId,
    name: String,
    capabilities: Vec<Capability>,
    permission: Permission,
    prompt_template: String,
    llm_options: LlmOptions,
    state: AgentState,
}

impl DeclarativeAgent {
    /// The agent's human-readable name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The LLM options this agent was configured with.
    pub fn llm_options(&self) -> &LlmOptions {
        &self.llm_options
    }

    /// The agent's permission descriptor.
    pub fn permission(&self) -> &Permission {
        &self.permission
    }

    /// Produce an [`AgentManifest`] for kernel registration.
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
}

#[async_trait]
impl Agent for DeclarativeAgent {
    fn id(&self) -> AgentId {
        self.id
    }

    fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn state(&self) -> AgentState {
        self.state
    }

    async fn run(
        &self,
        llm: &dyn LlmProvider,
        _tools: &dyn ToolSet,
        _services: &AgentServices,
    ) -> MacacaResult<AgentOutput> {
        if self.prompt_template.is_empty() {
            return Err(MacacaError::Agent(
                "DeclarativeAgent has no prompt template configured".into(),
            ));
        }

        let messages = vec![
            LlmMessage::system(self.prompt_template.clone()),
            LlmMessage::user(format!(
                "You are the '{}' agent. Execute your configured task.",
                self.name
            )),
        ];

        let response = llm.chat(messages, &self.llm_options).await?;

        Ok(AgentOutput {
            result: response.content,
            artifacts: Vec::new(),
            tokens_used: response.usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, TokenUsage};
    use macaca_tools::DefaultToolSet;

    struct MockLlm;

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }
        async fn chat(
            &self,
            messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            Ok(LlmResponse {
                content: format!("response for: {}", messages[0].content),
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

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
        let agent = AgentBuilder::from_config(sample_config()).build().unwrap();
        assert_eq!(agent.name(), "test-agent");
        assert_eq!(agent.capabilities().len(), 1);
        assert_eq!(agent.state(), AgentState::Created);
        assert_eq!(agent.llm_options().model, "mock-model");
    }

    #[test]
    fn builder_with_id_override() {
        let id = AgentId::new();
        let agent = AgentBuilder::from_config(sample_config())
            .with_id(id)
            .build()
            .unwrap();
        assert_eq!(agent.id(), id);
    }

    #[test]
    fn builder_with_model_override() {
        let agent = AgentBuilder::from_config(sample_config())
            .with_model("gpt-4o")
            .build()
            .unwrap();
        assert_eq!(agent.llm_options().model, "gpt-4o");
    }

    #[test]
    fn builder_with_prompt_override() {
        let agent = AgentBuilder::from_config(sample_config())
            .with_prompt("Custom prompt")
            .build()
            .unwrap();
        // The agent still builds — prompt is overridden internally
        assert_eq!(agent.name(), "test-agent");
    }

    #[test]
    fn build_with_manifest_produces_both() {
        let (agent, manifest) = AgentBuilder::from_config(sample_config())
            .build_with_manifest()
            .unwrap();
        assert_eq!(manifest.id, agent.id());
        assert_eq!(manifest.name, "test-agent");
        assert_eq!(manifest.capabilities.len(), 1);
    }

    #[test]
    fn build_invalid_config_fails() {
        let mut config = sample_config();
        config.name = String::new();
        let err = AgentBuilder::from_config(config).build().unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[tokio::test]
    async fn declarative_agent_run_calls_llm() {
        let agent = AgentBuilder::from_config(sample_config()).build().unwrap();
        let llm = MockLlm;
        let tools = DefaultToolSet::new();
        let services = AgentServices::empty();

        let output = agent.run(&llm, &tools, &services).await.unwrap();
        assert!(output.result.contains("You are a test agent."));
        assert_eq!(output.tokens_used.total_tokens, 15);
    }

    #[tokio::test]
    async fn declarative_agent_empty_prompt_errors() {
        let mut config = sample_config();
        config.prompt_template = String::new();
        let agent = AgentBuilder::from_config(config).build().unwrap();

        let llm = MockLlm;
        let tools = DefaultToolSet::new();
        let services = AgentServices::empty();

        let err = agent.run(&llm, &tools, &services).await.unwrap_err();
        assert!(err.to_string().contains("prompt template"));
    }
}
