//! BasicAgent: a minimal Agent implementation that calls an LLM with a task description.

use async_trait::async_trait;
use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentId, AgentOutput, AgentState, MacacaResult, Capability, LlmMessage, LlmOptions,
};
use macaca_tools::ToolSet;
use tracing::instrument;

use crate::agent::{Agent, AgentServices};

/// A simple agent that sends a task description to an LLM and returns its response.
pub struct BasicAgent {
    id: AgentId,
    task_description: String,
    capabilities: Vec<Capability>,
    state: AgentState,
}

impl BasicAgent {
    /// Create a new BasicAgent with the given task description.
    pub fn new(task_description: impl Into<String>) -> Self {
        Self {
            id: AgentId::new(),
            task_description: task_description.into(),
            capabilities: vec![Capability {
                name: "text_generation".into(),
                description: "Generate text responses via an LLM".into(),
            }],
            state: AgentState::Created,
        }
    }

    /// Create with an explicit id (useful for testing).
    pub fn with_id(id: AgentId, task_description: impl Into<String>) -> Self {
        Self {
            id,
            task_description: task_description.into(),
            capabilities: vec![Capability {
                name: "text_generation".into(),
                description: "Generate text responses via an LLM".into(),
            }],
            state: AgentState::Created,
        }
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
    use macaca_proto::{MacacaResult, LlmMessage, LlmOptions, LlmResponse, TokenUsage};
    use macaca_llm::LlmProvider;
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
}
