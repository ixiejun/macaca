//! Integration tests for the full Kernel lifecycle:
//! agent registration, execution, listing, and unregistration.

use std::sync::Arc;

use async_trait::async_trait;

use macaca_agent::{AgentExecutionPort, LegacyAgentExecutionAdapter, ToolCatalog};
use macaca_kernel::{Kernel, KernelBuilder};
use macaca_llm::LlmProvider;
use macaca_proto::config::KernelConfig;
use macaca_proto::{
    AgentId, AgentManifest, LlmMessage, LlmOptions, LlmResponse, MacacaError, MacacaResult,
    TokenUsage,
};
use macaca_sdk::{AgentBuilder, AgentConfig, DeclarativeAgent, MacacaSdk};
use macaca_tools::DefaultToolSet;

// ---------------------------------------------------------------------------
// Mock LLM provider
// ---------------------------------------------------------------------------

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
            content: "mock response".into(),
            reasoning_content: None,
            model: "mock".into(),
            usage: TokenUsage {
                prompt_tokens: 5,
                completion_tokens: 3,
                total_tokens: 8,
            },
            finish_reason: "stop".into(),
            tool_calls: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_kernel() -> Kernel {
    let config = KernelConfig {
        max_agents: 16,
        heartbeat_interval_ms: 5000,
        agent_timeout_ms: 30000,
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
    let execution_port: Arc<dyn AgentExecutionPort> = Arc::new(LegacyAgentExecutionAdapter::new(llm, Arc::from(Box::new(DefaultToolSet::new()) as Box<dyn ToolCatalog>)));
    KernelBuilder::from_execution_port(config, execution_port).build()
}

fn sample_agent_config(name: &str) -> AgentConfig {
    AgentConfig::from_yaml(&format!(
        r#"
name: {name}
capabilities:
  - name: test_cap
    description: A test capability
prompt_template: "You are a test agent named {name}."
model: mock
max_tokens: 100
"#
    ))
    .unwrap()
}

fn build_declarative_agent(config: AgentConfig) -> (DeclarativeAgent, AgentManifest) {
    let spec = AgentBuilder::from_config(config).build_spec().unwrap();
    let manifest = spec.manifest();
    let agent = spec.into_agent();
    (agent, manifest)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Full lifecycle: register a declarative agent via SDK, execute it, list, unregister.
#[tokio::test]
async fn declarative_agent_full_lifecycle() {
    let kernel = make_kernel();

    // Build agent from config via SDK
    let config = sample_agent_config("lifecycle-agent");
    let (agent, manifest) = build_declarative_agent(config);
    let agent_id = manifest.id;

    // Register
    kernel
        .register_agent(Box::new(agent), manifest)
        .await
        .unwrap();
    assert_eq!(kernel.agent_count().await, 1);

    // Execute
    let output = kernel.execute_agent(&agent_id).await.unwrap();
    assert_eq!(output.result, "mock response");
    assert_eq!(output.tokens_used.total_tokens, 8);

    // List
    let agents = kernel.list_agents().await;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].id, agent_id);
    assert_eq!(agents[0].name, "lifecycle-agent");

    // Unregister
    kernel.unregister_agent(&agent_id).await.unwrap();
    assert_eq!(kernel.agent_count().await, 0);
}

/// Register multiple agents and verify listing returns all of them.
#[tokio::test]
async fn register_multiple_agents() {
    let kernel = make_kernel();

    let mut ids = Vec::new();
    for i in 0..4 {
        let config = sample_agent_config(&format!("agent-{i}"));
        let (agent, manifest) = build_declarative_agent(config);
        let id = manifest.id;
        kernel
            .register_agent(Box::new(agent), manifest)
            .await
            .unwrap();
        ids.push(id);
    }

    assert_eq!(kernel.agent_count().await, 4);

    let agents = kernel.list_agents().await;
    for id in &ids {
        assert!(agents.iter().any(|a| a.id == *id));
    }

    // Unregister all
    for id in &ids {
        kernel.unregister_agent(id).await.unwrap();
    }
    assert_eq!(kernel.agent_count().await, 0);
}

/// Executing a non-existent agent returns NotFound.
#[tokio::test]
async fn execute_nonexistent_agent_returns_not_found() {
    let kernel = make_kernel();
    let err = kernel.execute_agent(&AgentId::new()).await.unwrap_err();
    assert!(matches!(err, MacacaError::NotFound(_)));
}

/// Unregistering a non-existent agent returns an error.
#[tokio::test]
async fn unregister_nonexistent_agent_returns_error() {
    let kernel = make_kernel();
    let result = kernel.unregister_agent(&AgentId::new()).await;
    assert!(result.is_err());
}

/// SDK facade registration works end-to-end.
#[tokio::test]
async fn sdk_facade_register_config_end_to_end() {
    let kernel = make_kernel();
    let config = sample_agent_config("sdk-agent");

    let id = MacacaSdk::for_kernel(&kernel)
        .register_config(config)
        .await
        .unwrap();

    assert_eq!(kernel.agent_count().await, 1);
    let agents = kernel.list_agents().await;
    assert_eq!(agents[0].id, id);
    assert_eq!(agents[0].name, "sdk-agent");

    // Execute via kernel
    let output = kernel.execute_agent(&id).await.unwrap();
    assert_eq!(output.result, "mock response");
}
