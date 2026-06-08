//! Contract tests for the chat-completions API adapter.

use serde_json::json;

use crate::formatter::Formatter;
use crate::model::{ChatOptions, ToolChoice};

use super::super::openai::OpenAiChatModel;


#[test]
fn test_openai_build_request_body() {
    let model = OpenAiChatModel::new("sk-test", "gpt-4");
    let messages = vec![
        json!({"role": "system", "content": "You are helpful."}),
        json!({"role": "user", "content": "Hello"}),
    ];
    let opts = ChatOptions {
        temperature: Some(0.7),
        max_tokens: Some(1024),
        ..Default::default()
    };

    let body = model.build_request_body(&messages, &opts);

    assert_eq!(body["model"], "gpt-4");
    assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    assert_eq!(body["temperature"], 0.7);
    assert_eq!(body["max_tokens"], 1024);
    assert!(body.get("tools").is_none());
}

#[test]
fn test_openai_build_request_body_with_tools() {
    let model = OpenAiChatModel::new("sk-test", "gpt-4");
    let messages = vec![json!({"role": "user", "content": "Search for rust"})];
    let tools = vec![json!({
        "type": "function",
        "function": {
            "name": "search",
            "description": "Search the web",
            "parameters": {"type": "object", "properties": {"q": {"type": "string"}}}
        }
    })];
    let opts = ChatOptions {
        tools: Some(tools),
        tool_choice: Some(ToolChoice::Auto),
        ..Default::default()
    };

    let body = model.build_request_body(&messages, &opts);

    assert!(body["tools"].as_array().unwrap().len() == 1);
    assert_eq!(body["tool_choice"], "auto");
}

#[test]
fn test_openai_parse_success_response() {
    let model = OpenAiChatModel::new("sk-test", "gpt-4");
    let raw = json!({
        "id": "chatcmpl-abc123",
        "created": 1700000000,
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "Hello! How can I help you?"
            }
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 8,
            "total_tokens": 18
        }
    });

    let resp = model.formatter.parse_response(raw).unwrap();
    assert_eq!(resp.id, "chatcmpl-abc123");
    assert_eq!(resp.get_text(), "Hello! How can I help you?");
    assert_eq!(resp.usage.input_tokens, 10);
    assert_eq!(resp.usage.output_tokens, 8);
    assert_eq!(resp.usage.total_tokens, 18);
    assert!(!resp.has_tool_calls());
}

#[test]
fn test_openai_parse_tool_call_response() {
    let model = OpenAiChatModel::new("sk-test", "gpt-4");
    let raw = json!({
        "id": "chatcmpl-tools",
        "created": 1700000001,
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_abc",
                    "type": "function",
                    "function": {
                        "name": "search",
                        "arguments": "{\"q\": \"rust programming\"}"
                    }
                }]
            }
        }],
        "usage": {"prompt_tokens": 20, "completion_tokens": 15, "total_tokens": 35}
    });

    let resp = model.formatter.parse_response(raw).unwrap();
    assert!(resp.has_tool_calls());
    let calls = resp.get_tool_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "call_abc");
    assert_eq!(calls[0].name, "search");
    assert_eq!(calls[0].input["q"], "rust programming");
}

#[test]
fn test_openai_error_response() {
    let model = OpenAiChatModel::new("sk-test", "gpt-4");
    // Simulate an API error JSON — formatter returns empty content
    let raw = json!({
        "error": {
            "message": "Invalid API key",
            "type": "invalid_request_error",
            "code": "invalid_api_key"
        }
    });

    // The formatter gracefully handles missing choices with empty response
    let result = model.formatter.parse_response(raw).unwrap();
    assert!(result.content.is_empty());
    assert_eq!(result.get_text(), "");
}

#[test]
fn test_openai_with_base_url() {
    let model = OpenAiChatModel::new("sk-test", "gpt-4")
        .with_base_url("https://api.deepseek.com/v1");
    assert_eq!(
        model.endpoint(),
        "https://api.deepseek.com/v1/chat/completions"
    );
}

#[test]
fn test_openai_with_base_url_trailing_slash() {
    let model = OpenAiChatModel::new("sk-test", "gpt-4")
        .with_base_url("https://api.deepseek.com/v1/");
    assert_eq!(
        model.endpoint(),
        "https://api.deepseek.com/v1/chat/completions"
    );
}

#[test]
fn test_openai_default_options() {
    let default_opts = ChatOptions {
        temperature: Some(0.5),
        max_tokens: Some(2048),
        ..Default::default()
    };
    let model = OpenAiChatModel::new("sk-test", "gpt-4").with_default_options(default_opts);

    // Call with empty options → defaults should apply
    let messages = vec![json!({"role": "user", "content": "hi"})];
    let body = model.build_request_body(&messages, &ChatOptions::default());

    assert_eq!(body["temperature"], 0.5);
    assert_eq!(body["max_tokens"], 2048);
}

#[test]
fn test_openai_options_override_defaults() {
    let default_opts = ChatOptions {
        temperature: Some(0.5),
        max_tokens: Some(2048),
        ..Default::default()
    };
    let model = OpenAiChatModel::new("sk-test", "gpt-4").with_default_options(default_opts);

    let override_opts = ChatOptions {
        temperature: Some(1.0),
        ..Default::default()
    };
    let messages = vec![json!({"role": "user", "content": "hi"})];
    let body = model.build_request_body(&messages, &override_opts);

    // temperature overridden, max_tokens from default
    assert_eq!(body["temperature"], 1.0);
    assert_eq!(body["max_tokens"], 2048);
}

#[test]
fn test_openai_model_name_override() {
    let model = OpenAiChatModel::new("sk-test", "gpt-4");
    let opts = ChatOptions {
        model: Some("gpt-4o".into()),
        ..Default::default()
    };
    let messages = vec![json!({"role": "user", "content": "hi"})];
    let body = model.build_request_body(&messages, &opts);
    assert_eq!(body["model"], "gpt-4o");
}

#[test]
fn test_openai_tool_choice_specific() {
    let model = OpenAiChatModel::new("sk-test", "gpt-4");
    let opts = ChatOptions {
        tools: Some(vec![
            json!({"type": "function", "function": {"name": "my_tool"}}),
        ]),
        tool_choice: Some(ToolChoice::Specific("my_tool".into())),
        ..Default::default()
    };
    let messages = vec![json!({"role": "user", "content": "hi"})];
    let body = model.build_request_body(&messages, &opts);
    assert_eq!(body["tool_choice"]["type"], "function");
    assert_eq!(body["tool_choice"]["function"]["name"], "my_tool");
}