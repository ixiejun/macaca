//! Contract tests for the Messages API adapter.

use serde_json::json;

use crate::formatter::Formatter;
use crate::model::{ChatOptions, ToolChoice};

use super::super::anthropic::AnthropicChatModel;


#[test]
fn test_anthropic_build_request_with_system() {
    let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
    let messages =
        vec![json!({"role": "user", "content": [{"type": "text", "text": "Hello"}]})];
    let opts = ChatOptions {
        max_tokens: Some(4096),
        temperature: Some(0.7),
        ..Default::default()
    };

    let body = model.build_request_body(&messages, Some("Be helpful."), &opts);

    assert_eq!(body["model"], "claude-sonnet-4-20250514");
    assert_eq!(body["max_tokens"], 4096);
    assert_eq!(body["system"], "Be helpful.");
    assert_eq!(body["temperature"], 0.7);
    assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    assert!(body.get("tools").is_none());
}

#[test]
fn test_anthropic_build_request_no_system() {
    let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
    let messages =
        vec![json!({"role": "user", "content": [{"type": "text", "text": "Hello"}]})];
    let opts = ChatOptions::default();

    let body = model.build_request_body(&messages, None, &opts);

    assert!(body.get("system").is_none());
    assert_eq!(body["max_tokens"], 4096); // default
}

#[test]
fn test_anthropic_build_request_with_tools() {
    let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
    let messages =
        vec![json!({"role": "user", "content": [{"type": "text", "text": "Search"}]})];
    let tools = vec![json!({
        "name": "search",
        "description": "Search the web",
        "input_schema": {"type": "object", "properties": {"q": {"type": "string"}}}
    })];
    let opts = ChatOptions {
        tools: Some(tools),
        tool_choice: Some(ToolChoice::Auto),
        ..Default::default()
    };

    let body = model.build_request_body(&messages, None, &opts);

    assert!(body["tools"].as_array().unwrap().len() == 1);
    assert_eq!(body["tool_choice"]["type"], "auto");
}

#[test]
fn test_anthropic_parse_success_response() {
    let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
    let raw = json!({
        "id": "msg-abc123",
        "type": "message",
        "content": [
            {"type": "text", "text": "Hello from Claude!"}
        ],
        "usage": {
            "input_tokens": 12,
            "output_tokens": 6
        }
    });

    let resp = model.formatter.parse_response(raw).unwrap();
    assert_eq!(resp.id, "msg-abc123");
    assert_eq!(resp.get_text(), "Hello from Claude!");
    assert_eq!(resp.usage.input_tokens, 12);
    assert_eq!(resp.usage.output_tokens, 6);
    assert_eq!(resp.usage.total_tokens, 18);
    assert!(!resp.has_tool_calls());
}

#[test]
fn test_anthropic_parse_tool_use_response() {
    let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
    let raw = json!({
        "id": "msg-tools",
        "type": "message",
        "content": [
            {"type": "text", "text": "Let me search for that."},
            {
                "type": "tool_use",
                "id": "toolu_abc",
                "name": "search",
                "input": {"q": "rust programming"}
            }
        ],
        "usage": {"input_tokens": 20, "output_tokens": 15}
    });

    let resp = model.formatter.parse_response(raw).unwrap();
    assert!(resp.has_tool_calls());
    assert_eq!(resp.get_text(), "Let me search for that.");
    let calls = resp.get_tool_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "toolu_abc");
    assert_eq!(calls[0].name, "search");
    assert_eq!(calls[0].input["q"], "rust programming");
}

#[test]
fn test_anthropic_extract_system_from_formatted() {
    let messages = vec![
        json!({"role": "system", "content": "Be helpful."}),
        json!({"role": "user", "content": [{"type": "text", "text": "Hi"}]}),
    ];

    let (system, filtered) = AnthropicChatModel::extract_system_from_formatted(&messages);
    assert_eq!(system.as_deref(), Some("Be helpful."));
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0]["role"], "user");
}

#[test]
fn test_anthropic_extract_system_from_formatted_array_content() {
    let messages = vec![
        json!({"role": "system", "content": [{"type": "text", "text": "System prompt"}]}),
        json!({"role": "user", "content": [{"type": "text", "text": "Hi"}]}),
    ];

    let (system, filtered) = AnthropicChatModel::extract_system_from_formatted(&messages);
    assert_eq!(system.as_deref(), Some("System prompt"));
    assert_eq!(filtered.len(), 1);
}

#[test]
fn test_anthropic_extract_system_no_system() {
    let messages =
        vec![json!({"role": "user", "content": [{"type": "text", "text": "Hi"}]})];

    let (system, filtered) = AnthropicChatModel::extract_system_from_formatted(&messages);
    assert!(system.is_none());
    assert_eq!(filtered.len(), 1);
}

#[test]
fn test_anthropic_with_base_url() {
    let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514")
        .with_base_url("https://my-proxy.com");
    assert_eq!(model.endpoint(), "https://my-proxy.com/v1/messages");
}

#[test]
fn test_anthropic_default_options() {
    let default_opts = ChatOptions {
        temperature: Some(0.3),
        max_tokens: Some(8192),
        ..Default::default()
    };
    let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514")
        .with_default_options(default_opts);

    let messages =
        vec![json!({"role": "user", "content": [{"type": "text", "text": "hi"}]})];
    let body = model.build_request_body(&messages, None, &ChatOptions::default());

    assert_eq!(body["temperature"], 0.3);
    assert_eq!(body["max_tokens"], 8192);
}

#[test]
fn test_anthropic_tool_choice_specific() {
    let model = AnthropicChatModel::new("sk-ant-test", "claude-sonnet-4-20250514");
    let opts = ChatOptions {
        tools: Some(vec![json!({"name": "my_tool"})]),
        tool_choice: Some(ToolChoice::Specific("my_tool".into())),
        ..Default::default()
    };
    let messages =
        vec![json!({"role": "user", "content": [{"type": "text", "text": "hi"}]})];
    let body = model.build_request_body(&messages, None, &opts);
    assert_eq!(body["tool_choice"]["type"], "tool");
    assert_eq!(body["tool_choice"]["name"], "my_tool");
}