//! Live LLM integration test — validates real API connectivity.
//!
//! Run with: `cargo test --test live_llm_test -- --ignored --nocapture`
//!
//! Requires valid API keys in `config/default.toml`.

use macaca_llm::LlmRouter;
use macaca_proto::config::MacacaConfig;
use macaca_proto::types::{LlmMessage, LlmOptions};
use std::path::PathBuf;

/// Resolve config path relative to workspace root (not crate dir).
fn config_path() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest)
        .parent().unwrap() // crates/
        .parent().unwrap() // workspace root
        .join("config/default.toml")
}

fn load_config() -> MacacaConfig {
    MacacaConfig::load(config_path())
        .expect("Failed to load config/default.toml")
}

fn load_router() -> LlmRouter {
    let config = load_config();
    LlmRouter::from_config(&config.llm)
        .expect("Failed to create router from config")
}

#[tokio::test]
#[ignore]
async fn live_dashscope_qwen_chat() {
    let router = load_router();

    let messages = vec![LlmMessage::user("Say 'hello' in one word. Nothing else.")];
    let options = LlmOptions {
        model: "qwen-turbo".into(),
        max_tokens: Some(10),
        temperature: Some(0.0),
        ..Default::default()
    };

    let resp = router.chat(messages, &options).await.unwrap();
    println!("[DashScope qwen-turbo] Response: {:?}", resp.content);
    println!("[DashScope qwen-turbo] Model: {}", resp.model);
    println!("[DashScope qwen-turbo] Tokens: {:?}", resp.usage);
    assert!(!resp.content.is_empty(), "Response should not be empty");
    assert!(resp.usage.total_tokens > 0, "Should have used tokens");
}

#[tokio::test]
#[ignore]
async fn live_dashscope_qwen_max_chat() {
    let router = load_router();

    let messages = vec![
        LlmMessage::system("You are a helpful coding assistant. Reply concisely."),
        LlmMessage::user("Write a Rust function that adds two numbers. Just the code, no explanation."),
    ];
    let options = LlmOptions {
        model: "qwen-max".into(),
        max_tokens: Some(200),
        temperature: Some(0.0),
        ..Default::default()
    };

    let resp = router.chat(messages, &options).await.unwrap();
    println!("[DashScope qwen-max] Response:\n{}", resp.content);
    println!("[DashScope qwen-max] Tokens: {:?}", resp.usage);
    assert!(!resp.content.is_empty());
    // The response should contain Rust code
    assert!(
        resp.content.contains("fn") || resp.content.contains("pub"),
        "Response should contain Rust code"
    );
}

#[tokio::test]
#[ignore]
async fn live_dashscope_embedding() {
    let config = load_config();

    let api_key = config.memory.embedding.resolve_api_key()
        .expect("Failed to resolve embedding API key");
    let base_url = &config.memory.embedding.base_url;

    // Use the OpenAI-compatible embedding endpoint
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "text-embedding-v4",
        "input": ["Agent OS 是一个借鉴 Linux 进程模型的 AI Agent 操作系统"],
    });

    let resp = client
        .post(format!("{}/embeddings", base_url))
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
        .expect("Request failed");

    let status = resp.status();
    let text = resp.text().await.unwrap();
    println!("[Embedding] Status: {status}");
    println!("[Embedding] Response: {}", &text[..text.len().min(500)]);
    assert!(status.is_success(), "Embedding request failed: {text}");

    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    let embedding = &parsed["data"][0]["embedding"];
    assert!(embedding.is_array(), "Should return embedding array");
    let dims = embedding.as_array().unwrap().len();
    println!("[Embedding] Dimensions: {dims}");
    assert_eq!(dims, 1024, "text-embedding-v4 should return 1024 dims");
}

#[tokio::test]
#[ignore]
async fn live_router_from_config_builds_all_providers() {
    let config = load_config();
    let router = LlmRouter::from_config(&config.llm)
        .expect("Failed to build router");

    // Verify dashscope provider is registered by making a request
    let messages = vec![LlmMessage::user("Reply with just the number 42")];
    let options = LlmOptions {
        model: "qwen-turbo".into(),
        max_tokens: Some(5),
        temperature: Some(0.0),
        ..Default::default()
    };

    let resp = router.chat(messages, &options).await.unwrap();
    println!("[Router test] Response: {:?}", resp.content);
    assert!(!resp.content.is_empty());
}
