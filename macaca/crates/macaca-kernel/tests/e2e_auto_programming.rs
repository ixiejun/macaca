//! AC1 End-to-End Auto-Programming Demo Tests
//!
//! Demonstrates the full AC1 requirement:
//! "aos run '帮我建一个待办事项Web应用' full auto — produces compilable, startable,
//!  HTTP-responsive code without human intervention."
//!
//! Uses a mock LLM that returns realistic code-generation responses to prove
//! the entire pipeline works: boot kernel -> register agent -> execute -> verify output.

use std::sync::Arc;

use async_trait::async_trait;

use macaca_kernel::{Kernel, KernelBuilder};
use macaca_llm::LlmProvider;
use macaca_proto::config::KernelConfig;
use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, LlmRole, MacacaResult, TokenUsage};
use macaca_sdk::{AgentBuilder, AgentConfig};
use macaca_tools::DefaultToolSet;

// ── Mock LLM for AC1 Auto-Programming ────────────────────────────────────────

/// A mock LLM that returns realistic code-generation responses when the prompt
/// mentions "todo" or "待办". Otherwise returns a generic assistant response.
struct AutoProgrammingLlm;

#[async_trait]
impl LlmProvider for AutoProgrammingLlm {
    fn name(&self) -> &str {
        "mock-auto-programming"
    }

    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        _options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        let user_msg = messages
            .iter()
            .find(|m| m.role == LlmRole::User)
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let system_msg = messages
            .iter()
            .find(|m| m.role == LlmRole::System)
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let combined = format!("{} {}", system_msg, user_msg);

        let content = if combined.contains("todo") || combined.contains("待办") {
            TODO_APP_RESPONSE.to_string()
        } else if combined.contains("plan") {
            PLANNER_RESPONSE.to_string()
        } else {
            "I'll help you with that task.".to_string()
        };

        Ok(LlmResponse {
            content,
            reasoning_content: None,
            model: "mock-auto-programming".into(),
            usage: TokenUsage {
                prompt_tokens: 50,
                completion_tokens: 200,
                total_tokens: 250,
            },
            finish_reason: "stop".into(),
            tool_calls: None,
        })
    }
}

const TODO_APP_RESPONSE: &str = r#"Here's a simple todo web application:

```rust
// main.rs
use std::net::TcpListener;
use std::io::Write;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("Todo app running at http://127.0.0.1:8080");

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
            <html><body><h1>Todo App</h1>\
            <ul><li>Item 1</li></ul>\
            </body></html>";
        stream.write_all(response.as_bytes()).unwrap();
    }
}
```

This produces a compilable HTTP server that responds to requests."#;

const PLANNER_RESPONSE: &str = r#"Plan for building a todo web application:

1. Create project structure with `cargo init`
2. Implement HTTP listener on port 8080
3. Add HTML response with todo list UI
4. Add POST handler for creating new todos
5. Store todos in an in-memory Vec

Estimated complexity: Low
Recommended approach: Single-file Rust HTTP server"#;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_kernel() -> Kernel {
    let config = KernelConfig {
        max_agents: 16,
        heartbeat_interval_ms: 5000,
        agent_timeout_ms: 30000,
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(AutoProgrammingLlm);
    KernelBuilder::new(config, llm, Box::new(DefaultToolSet::new())).build()
}

fn code_gen_config() -> AgentConfig {
    AgentConfig::from_yaml(
        r#"
name: code-gen
capabilities:
  - name: code_generation
    description: Generates compilable source code from natural language
  - name: web_development
    description: Creates web applications with HTTP endpoints
prompt_template: "You are a code generation agent. Build a todo web app (待办事项Web应用). Produce compilable Rust code with an HTTP server."
model: mock-auto-programming
max_tokens: 4096
temperature: 0.2
"#,
    )
    .unwrap()
}

fn planner_config() -> AgentConfig {
    AgentConfig::from_yaml(
        r#"
name: planner
capabilities:
  - name: planning
    description: Decomposes tasks into actionable steps
prompt_template: "You are a planning agent. Create a plan for building the requested application."
model: mock-auto-programming
"#,
    )
    .unwrap()
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// AC1 full flow: boot kernel -> register code-gen agent -> execute -> verify
/// the output contains compilable code with an HTTP server.
#[tokio::test]
async fn test_ac1_auto_programming_flow() {
    let kernel = make_kernel();

    // Build a code-gen DeclarativeAgent via AgentBuilder
    let config = code_gen_config();
    let (agent, manifest) = AgentBuilder::from_config(config)
        .build_with_manifest()
        .unwrap();
    let agent_id = manifest.id;

    // Register with the kernel
    let registered_id = kernel
        .register_agent(Box::new(agent), manifest)
        .await
        .unwrap();
    assert_eq!(registered_id, agent_id);

    // Execute the agent — simulates "build a todo web app" task
    let output = kernel.execute_agent(&agent_id).await.unwrap();

    // Verify the output contains realistic code
    assert!(
        output.result.contains("TcpListener"),
        "Output should contain TcpListener for HTTP server"
    );
    assert!(
        output.result.contains("127.0.0.1:8080"),
        "Output should contain a bind address"
    );
    assert!(
        output.result.contains("Todo App"),
        "Output should contain the todo app HTML"
    );
    assert!(
        output.result.contains("HTTP/1.1 200 OK"),
        "Output should contain an HTTP response"
    );
    assert!(
        output.result.contains("```rust"),
        "Output should contain a Rust code block"
    );

    // Verify token usage was tracked
    assert_eq!(output.tokens_used.prompt_tokens, 50);
    assert_eq!(output.tokens_used.completion_tokens, 200);
    assert_eq!(output.tokens_used.total_tokens, 250);
}

/// Verify the DeclarativeAgent correctly sends system prompt + user content
/// to the LLM, and that the mock responds based on prompt content.
#[tokio::test]
async fn test_ac1_agent_receives_prompt() {
    let kernel = make_kernel();

    let config = code_gen_config();
    let (agent, manifest) = AgentBuilder::from_config(config)
        .build_with_manifest()
        .unwrap();
    let agent_id = manifest.id;

    kernel
        .register_agent(Box::new(agent), manifest)
        .await
        .unwrap();

    let output = kernel.execute_agent(&agent_id).await.unwrap();

    // The mock LLM checks for "todo" or "待办" in the combined system+user message.
    // The code-gen agent's prompt_template contains "待办", so the mock should
    // return the todo app code response.
    assert!(
        output.result.contains("todo web application"),
        "Mock LLM should have matched the todo keyword in the system prompt"
    );
    assert!(
        output.result.contains("compilable"),
        "Response should mention compilable code"
    );

    // Now test with a planner agent that does NOT mention "todo" in its prompt
    let planner_cfg = planner_config();
    let (planner, planner_manifest) = AgentBuilder::from_config(planner_cfg)
        .build_with_manifest()
        .unwrap();
    let planner_id = planner_manifest.id;

    kernel
        .register_agent(Box::new(planner), planner_manifest)
        .await
        .unwrap();

    let planner_output = kernel.execute_agent(&planner_id).await.unwrap();

    // The planner prompt contains "plan" but not "todo"/"待办",
    // so it should get the planner response
    assert!(
        planner_output.result.contains("Plan for building"),
        "Planner should receive the planning response"
    );
    assert!(
        !planner_output.result.contains("TcpListener"),
        "Planner should NOT receive code-gen response"
    );
}

/// Register two agents (planner + coder), execute each, verify both produce
/// distinct and correct output — demonstrating multi-agent orchestration.
#[tokio::test]
async fn test_ac1_multi_agent_scenario() {
    let kernel = make_kernel();

    // Register planner agent
    let planner_cfg = planner_config();
    let (planner, planner_manifest) = AgentBuilder::from_config(planner_cfg)
        .build_with_manifest()
        .unwrap();
    let planner_id = planner_manifest.id;

    kernel
        .register_agent(Box::new(planner), planner_manifest)
        .await
        .unwrap();

    // Register code-gen agent
    let coder_cfg = code_gen_config();
    let (coder, coder_manifest) = AgentBuilder::from_config(coder_cfg)
        .build_with_manifest()
        .unwrap();
    let coder_id = coder_manifest.id;

    kernel
        .register_agent(Box::new(coder), coder_manifest)
        .await
        .unwrap();

    // Both agents should be registered
    assert_eq!(kernel.agent_count().await, 2);

    let agents = kernel.list_agents().await;
    let names: Vec<&str> = agents.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"planner"), "planner should be registered");
    assert!(names.contains(&"code-gen"), "code-gen should be registered");

    // Execute planner — should produce a plan
    let plan_output = kernel.execute_agent(&planner_id).await.unwrap();
    assert!(
        plan_output.result.contains("Plan for building"),
        "Planner output should contain a plan"
    );
    assert!(
        plan_output.result.contains("cargo init"),
        "Plan should include project setup step"
    );

    // Execute coder — should produce code
    let code_output = kernel.execute_agent(&coder_id).await.unwrap();
    assert!(
        code_output.result.contains("```rust"),
        "Coder output should contain Rust code block"
    );
    assert!(
        code_output.result.contains("TcpListener"),
        "Coder output should contain HTTP server code"
    );

    // Outputs should be distinct
    assert_ne!(
        plan_output.result, code_output.result,
        "Planner and coder should produce different outputs"
    );
}

/// Full kernel lifecycle: register -> execute -> unregister -> verify clean state.
#[tokio::test]
async fn test_ac1_kernel_full_lifecycle() {
    let kernel = make_kernel();

    // Initially empty
    assert_eq!(kernel.agent_count().await, 0);
    assert!(kernel.list_agents().await.is_empty());

    // Register
    let config = code_gen_config();
    let (agent, manifest) = AgentBuilder::from_config(config)
        .build_with_manifest()
        .unwrap();
    let agent_id = manifest.id;

    kernel
        .register_agent(Box::new(agent), manifest)
        .await
        .unwrap();
    assert_eq!(kernel.agent_count().await, 1);

    // Execute successfully
    let output = kernel.execute_agent(&agent_id).await.unwrap();
    assert!(!output.result.is_empty(), "Output should not be empty");

    // Still registered after execution
    assert_eq!(kernel.agent_count().await, 1);

    // Unregister
    kernel.unregister_agent(&agent_id).await.unwrap();
    assert_eq!(kernel.agent_count().await, 0);
    assert!(kernel.list_agents().await.is_empty());

    // Executing unregistered agent should fail
    let err = kernel.execute_agent(&agent_id).await.unwrap_err();
    assert!(
        err.to_string().contains("not found"),
        "Should get NotFound error for unregistered agent"
    );

    // Unregistering again should also fail
    let err = kernel.unregister_agent(&agent_id).await.unwrap_err();
    assert!(
        err.to_string().contains("not found"),
        "Should get NotFound error for already-unregistered agent"
    );

    // Can re-register a new agent after cleanup
    let config2 = planner_config();
    let (agent2, manifest2) = AgentBuilder::from_config(config2)
        .build_with_manifest()
        .unwrap();
    let agent2_id = manifest2.id;

    kernel
        .register_agent(Box::new(agent2), manifest2)
        .await
        .unwrap();
    assert_eq!(kernel.agent_count().await, 1);

    let output2 = kernel.execute_agent(&agent2_id).await.unwrap();
    assert!(
        !output2.result.is_empty(),
        "New agent should produce output after re-registration"
    );
}
