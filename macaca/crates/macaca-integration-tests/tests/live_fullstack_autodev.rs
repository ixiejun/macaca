//! Live LLM test for the fullstack-autodev application.
//!
//! Runs the architect agent with a real DashScope/Qwen LLM provider.
//! Requires: DASHSCOPE_API_KEY environment variable (or config key).
//!
//! Run with:
//!   cargo test -p aos-integration-tests --test live_fullstack_autodev -- --ignored --nocapture

use std::sync::Arc;

use macaca_app::loader::AppLoader;
use macaca_kernel::Kernel;
use macaca_llm::{DashScopeProvider, LlmProvider};
use macaca_proto::config::KernelConfig;
use macaca_proto::LlmOptions;
use macaca_sdk::{AgentBuilder, AgentPersona};
use macaca_skill::SkillCatalog;
use macaca_tools::DefaultToolSet;

fn app_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/apps/fullstack-autodev")
}

fn dashscope_api_key() -> String {
    std::env::var("DASHSCOPE_API_KEY")
        .unwrap_or_else(|_| "sk-7720115f25214ebaa4f83d5857224f9a".into())
}

fn make_kernel_with_dashscope() -> Kernel {
    let config = KernelConfig {
        max_agents: 64,
        heartbeat_interval_ms: 5000,
        agent_timeout_ms: 60000,
    };
    let llm: Arc<dyn LlmProvider> = Arc::new(DashScopeProvider::new(dashscope_api_key()));
    Kernel::new(&config, llm, Box::new(DefaultToolSet::new()))
}

// ---------------------------------------------------------------------------
// Live test: Direct DashScope call with qwen3-max
// ---------------------------------------------------------------------------

/// Simple smoke test: call DashScope with qwen3-max directly.
#[tokio::test]
#[ignore]
async fn live_dashscope_qwen3_max_chat() {
    let provider = DashScopeProvider::new(dashscope_api_key());
    let messages = vec![
        macaca_proto::LlmMessage::system("You are a helpful assistant. Reply concisely."),
        macaca_proto::LlmMessage::user("What is 2+2? Reply with just the number."),
    ];
    let options = LlmOptions {
        model: "qwen3-max".into(),
        max_tokens: Some(100),
        temperature: Some(0.1),
        ..Default::default()
    };

    let response = provider.chat(messages, &options).await.unwrap();

    println!("\n=== DashScope qwen3-max Response ===");
    println!("Model: {}", response.model);
    println!("Content: {}", response.content);
    println!(
        "Tokens: {} prompt + {} completion = {} total",
        response.usage.prompt_tokens,
        response.usage.completion_tokens,
        response.usage.total_tokens,
    );
    println!("Finish reason: {}", response.finish_reason);

    assert!(!response.content.is_empty());
    assert!(response.content.contains('4'));
}

// ---------------------------------------------------------------------------
// Live test: Load app, build persona prompt, call qwen3-max as architect
// ---------------------------------------------------------------------------

/// Load the fullstack-autodev app, build architect persona prompt,
/// and execute a design analysis task via DashScope qwen3-max.
#[tokio::test]
#[ignore]
async fn live_fullstack_autodev_architect_qwen3() {
    // 1. Load the app manifest.
    let manifest_path = app_dir().join("app.yaml");
    let manifest = AppLoader::load_manifest(&manifest_path).unwrap();
    println!("\n=== App: {} v{} ===", manifest.name, manifest.version);
    println!("Agents: {}", manifest.agents.len());

    // 2. Load the architect persona.
    let persona_dir = app_dir().join("personas/architect");
    let persona = AgentPersona::load_from_directory(&persona_dir).await.unwrap();
    let system_prompt = persona.to_system_prompt(Some(
        "You are an agent in the Agent OS fullstack-autodev application. \
         Your task is to analyze project requirements and create a technical specification.",
    ));
    println!(
        "\n=== Architect System Prompt ({} chars) ===",
        system_prompt.len()
    );
    println!("{}", &system_prompt[..system_prompt.len().min(500)]);
    if system_prompt.len() > 500 {
        println!("...");
    }

    // 3. Load skill catalog and activate relevant skills.
    let skills_dir = app_dir().join("skills");
    let mut catalog = SkillCatalog::new();
    catalog.load_from_directory(&skills_dir).await.unwrap();
    let catalog_prompt = catalog.catalog_prompt();
    println!("\n=== Skill Catalog ===\n{catalog_prompt}");

    // 4. Build messages and call DashScope.
    let provider = DashScopeProvider::new(dashscope_api_key());
    let messages = vec![
        macaca_proto::LlmMessage::system(format!("{system_prompt}\n\n{catalog_prompt}")),
        macaca_proto::LlmMessage::user(
            "Analyze this project requirement: Build a simple TODO web app with a Next.js frontend \
             and Go backend API. The frontend should use shadcn-ui components. The backend should \
             use chi router with PostgreSQL. Create a brief technical specification with:\n\
             1. Architecture overview\n\
             2. Frontend component list\n\
             3. Backend API endpoints\n\
             4. Database schema\n\
             Keep it concise (under 500 words).",
        ),
    ];

    let options = LlmOptions {
        model: "qwen3-max".into(),
        max_tokens: Some(2048),
        temperature: Some(0.7),
        ..Default::default()
    };

    println!("\n=== Calling DashScope qwen3-max... ===");
    let response = provider.chat(messages, &options).await.unwrap();

    println!("\n=== Architect Response ===");
    println!("Model: {}", response.model);
    println!(
        "Tokens: {} prompt + {} completion = {} total",
        response.usage.prompt_tokens,
        response.usage.completion_tokens,
        response.usage.total_tokens,
    );
    println!("Finish reason: {}", response.finish_reason);
    println!("\n{}", response.content);

    // Verify the response is reasonable.
    assert!(!response.content.is_empty());
    assert!(
        response.content.len() > 100,
        "Response too short: {} chars",
        response.content.len()
    );
    assert!(response.usage.total_tokens > 0);
}

// ---------------------------------------------------------------------------
// Live test: Full app lifecycle with kernel execution
// ---------------------------------------------------------------------------

/// Start the fullstack-autodev app with DashScope kernel and execute
/// the architect agent end-to-end through the kernel.
#[tokio::test]
#[ignore]
async fn live_fullstack_autodev_kernel_execution() {
    let kernel = make_kernel_with_dashscope();

    // 1. Load persona for architect.
    let persona_dir = app_dir().join("personas/architect");
    let persona = AgentPersona::load_from_directory(&persona_dir).await.unwrap();
    let system_prompt = persona.to_system_prompt(Some(
        "You are the Architect Agent in the Agent OS. \
         Analyze requirements and produce technical specifications.",
    ));

    // 2. Build agent config with persona prompt and qwen3-max model.
    let config = macaca_sdk::AgentConfig::from_yaml(
        r#"
name: architect
capabilities:
  - name: design_analysis
    description: Analyze requirements and generate technical specs
prompt_template: "placeholder"
model: qwen3-max
permission_level: system
"#,
    )
    .unwrap();

    // 3. Build agent with persona prompt injected.
    let (agent, manifest) = AgentBuilder::from_config(config)
        .with_prompt(system_prompt)
        .build_with_manifest()
        .unwrap();

    let agent_id = manifest.id;
    kernel
        .register_agent(Box::new(agent), manifest)
        .await
        .unwrap();

    println!("\n=== Executing architect agent via kernel (qwen3-max) ===");
    let output = kernel.execute_agent(&agent_id).await.unwrap();

    println!("\n=== Agent Output ===");
    println!(
        "Tokens: {} prompt + {} completion = {} total",
        output.tokens_used.prompt_tokens,
        output.tokens_used.completion_tokens,
        output.tokens_used.total_tokens,
    );
    println!("\n{}", output.result);

    assert!(!output.result.is_empty());
    assert!(output.tokens_used.total_tokens > 0);
}
