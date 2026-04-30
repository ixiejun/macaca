//! BasicAgent: a minimal Agent implementation that calls an LLM with a task description.

use async_trait::async_trait;
use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentId, AgentOutput, AgentState, Capability, LlmMessage, LlmOptions, MacacaResult,
};
use macaca_tools::ToolSet;
use tracing::instrument;

use crate::agent::{Agent, AgentServices};

#[derive(Debug, Clone)]
pub enum CapabilitySource {
    Legacy,
    Manifest,
    Persona,
    Skill,
    Driver,
    Mcp,
}

#[derive(Debug, Clone)]
pub enum AgentCapabilityNode {
    Leaf(Capability),
    Group {
        source: CapabilitySource,
        children: Vec<AgentCapabilityNode>,
    },
}

impl AgentCapabilityNode {
    fn flatten_into(&self, out: &mut Vec<Capability>) {
        match self {
            Self::Leaf(capability) => out.push(capability.clone()),
            Self::Group { children, .. } => {
                for child in children {
                    child.flatten_into(out);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentCapabilitySet {
    nodes: Vec<AgentCapabilityNode>,
}

impl AgentCapabilitySet {
    pub fn from_legacy(capabilities: Vec<Capability>) -> Self {
        Self {
            nodes: capabilities
                .into_iter()
                .map(AgentCapabilityNode::Leaf)
                .collect(),
        }
    }

    pub fn push_group(&mut self, source: CapabilitySource, children: Vec<Capability>) {
        self.nodes.push(AgentCapabilityNode::Group {
            source,
            children: children
                .into_iter()
                .map(AgentCapabilityNode::Leaf)
                .collect(),
        });
    }

    pub fn flatten_for_legacy_api(&self) -> Vec<Capability> {
        let mut flattened = Vec::new();
        for node in &self.nodes {
            node.flatten_into(&mut flattened);
        }
        flattened
    }
}

pub struct BasicAgentBuilder {
    id: AgentId,
    task_description: String,
    state: AgentState,
    capability_set: AgentCapabilitySet,
}

/// A simple agent that sends a task description to an LLM and returns its response.
pub struct BasicAgent {
    id: AgentId,
    task_description: String,
    capabilities: Vec<Capability>,
    capability_set: AgentCapabilitySet,
    state: AgentState,
}

impl BasicAgent {
    /// Create a new BasicAgent with the given task description.
    pub fn new(task_description: impl Into<String>) -> Self {
        BasicAgentBuilder::new(task_description).build()
    }

    /// Create with an explicit id (useful for testing).
    pub fn with_id(id: AgentId, task_description: impl Into<String>) -> Self {
        BasicAgentBuilder::new(task_description).with_id(id).build()
    }

    fn default_capabilities() -> Vec<Capability> {
        vec![Capability {
            name: "text_generation".into(),
            description: "Generate text responses via an LLM".into(),
        }]
    }

    fn from_parts(
        id: AgentId,
        task_description: String,
        state: AgentState,
        capability_set: AgentCapabilitySet,
    ) -> Self {
        let capabilities = capability_set.flatten_for_legacy_api();
        Self {
            id,
            task_description,
            capabilities,
            capability_set,
            state,
        }
    }

    pub fn capability_set(&self) -> &AgentCapabilitySet {
        &self.capability_set
    }
}

impl BasicAgentBuilder {
    pub fn new(task_description: impl Into<String>) -> Self {
        Self {
            id: AgentId::new(),
            task_description: task_description.into(),
            state: AgentState::Created,
            capability_set: AgentCapabilitySet::from_legacy(BasicAgent::default_capabilities()),
        }
    }

    pub fn with_id(mut self, id: AgentId) -> Self {
        self.id = id;
        self
    }

    pub fn with_state(mut self, state: AgentState) -> Self {
        self.state = state;
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<Capability>) -> Self {
        self.capability_set = AgentCapabilitySet::from_legacy(capabilities);
        self
    }

    pub fn with_capability_set(mut self, capability_set: AgentCapabilitySet) -> Self {
        self.capability_set = capability_set;
        self
    }

    pub fn build(self) -> BasicAgent {
        BasicAgent::from_parts(
            self.id,
            self.task_description,
            self.state,
            self.capability_set,
        )
    }
}

#[async_trait]
impl Agent for BasicAgent {
    fn id(&self) -> AgentId {
        self.id
    }

    fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn state(&self) -> AgentState {
        self.state
    }

    #[instrument(name = "basic_agent_run", skip(self, llm, _tools, _services), fields(agent_id = ?self.id))]
    async fn run(
        &self,
        llm: &dyn LlmProvider,
        _tools: &dyn ToolSet,
        _services: &AgentServices,
    ) -> MacacaResult<AgentOutput> {
        let messages = vec![
            LlmMessage::system("You are a helpful AI agent. Complete the given task concisely."),
            LlmMessage::user(self.task_description.clone()),
        ];

        let options = LlmOptions::default();
        let response = llm.chat(messages, &options).await?;

        Ok(AgentOutput {
            result: response.content,
            artifacts: Vec::new(),
            tokens_used: response.usage,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use macaca_llm::LlmProvider;
    use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaResult, TokenUsage};
    use macaca_tools::DefaultToolSet;

    struct MockLlm;

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            Ok(LlmResponse {
                content: "42".into(),
                reasoning_content: None,
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 1,
                    total_tokens: 11,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    #[tokio::test]
    async fn basic_agent_returns_llm_response() {
        let agent = BasicAgent::new("What is 6 * 7?");
        let llm = MockLlm;
        let tools = DefaultToolSet::new();
        let services = AgentServices::empty();

        let output = agent.run(&llm, &tools, &services).await.unwrap();
        assert_eq!(output.result, "42");
        assert_eq!(output.tokens_used.total_tokens, 11);
    }

    #[test]
    fn basic_agent_initial_state_is_created() {
        let agent = BasicAgent::new("test");
        assert_eq!(agent.state(), AgentState::Created);
    }

    #[test]
    fn basic_agent_has_capability() {
        let agent = BasicAgent::new("test");
        assert_eq!(agent.capabilities().len(), 1);
        assert_eq!(agent.capabilities()[0].name, "text_generation");
    }

    #[test]
    fn builder_matches_existing_constructor_defaults() {
        let agent = BasicAgentBuilder::new("test").build();
        assert_eq!(agent.state(), AgentState::Created);
        assert_eq!(agent.capabilities().len(), 1);
        assert_eq!(agent.capabilities()[0].name, "text_generation");
    }

    #[test]
    fn capability_set_flattens_legacy_output() {
        let mut capability_set = AgentCapabilitySet::default();
        capability_set.push_group(
            CapabilitySource::Skill,
            vec![Capability {
                name: "browser".into(),
                description: "Operate browser tools".into(),
            }],
        );
        let agent = BasicAgentBuilder::new("test")
            .with_capability_set(capability_set)
            .build();

        assert_eq!(agent.capability_set().flatten_for_legacy_api().len(), 1);
        assert_eq!(agent.capabilities()[0].name, "browser");
    }
}
