//! Contract tests for [`super::openai::OpenAiFormatter`].

use serde_json::json;

use crate::formatter::{
    Formatter, FormatterError, OpenAiFormatter,
};
use crate::message::{ContentBlock, ImageBlock, Msg, MsgContent, Role, TextBlock, ThinkingBlock, ToolUseBlock};
use crate::model::ChatResponse;

#[test]
fn test_openai_format_system_message() {
    let msg = Msg::system("You are a helpful assistant.");
    let fmt = OpenAiFormatter;
    let result = fmt.format(&[msg]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["role"], "system");
    assert_eq!(result[0]["content"], "You are a helpful assistant.");
}

#[test]
fn test_openai_format_user_text() {
    let msg = Msg::user("alice", "hello");
    let fmt = OpenAiFormatter;
    let result = fmt.format(&[msg]);
    assert_eq!(result[0]["role"], "user");
    assert_eq!(result[0]["content"], "hello");
}

#[test]
fn test_openai_format_assistant_with_tool_calls() {
    let blocks = vec![
        ContentBlock::Text(TextBlock {
            text: "Let me search.".into(),
        }),
        ContentBlock::ToolUse(ToolUseBlock {
            id: "call_1".into(),
            name: "web_search".into(),
            input: json!({"query": "rust"}),
            raw_input: None,
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    let fmt = OpenAiFormatter;
    let result = fmt.format(&[msg]);

    assert_eq!(result[0]["role"], "assistant");
    let tool_calls = result[0]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["id"], "call_1");
    assert_eq!(tool_calls[0]["function"]["name"], "web_search");
}

#[test]
fn test_openai_format_tool_result() {
    let msg = Msg::tool_result("call_1", "search", "5 results found", false);
    let fmt = OpenAiFormatter;
    let result = fmt.format(&[msg]);
    assert_eq!(result[0]["role"], "tool");
    assert_eq!(result[0]["tool_call_id"], "call_1");
    assert_eq!(result[0]["content"], "5 results found");
}

#[test]
fn test_openai_format_image_url() {
    let blocks = vec![ContentBlock::Image(ImageBlock {
        url: Some("https://example.com/img.png".into()),
        data: None,
        mime_type: None,
    })];
    let msg = Msg::user("alice", MsgContent::Blocks(blocks));
    let fmt = OpenAiFormatter;
    let result = fmt.format(&[msg]);
    let content = result[0]["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "image_url");
    assert_eq!(
        content[0]["image_url"]["url"],
        "https://example.com/img.png"
    );
}

#[test]
fn test_openai_thinking_omitted() {
    let blocks = vec![
        ContentBlock::Thinking(ThinkingBlock {
            thinking: "internal".into(),
        }),
        ContentBlock::Text(TextBlock {
            text: "visible".into(),
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    let fmt = OpenAiFormatter;
    let result = fmt.format(&[msg]);
    // content should only contain "visible"
    assert_eq!(result[0]["content"], "visible");
}

#[test]
fn test_openai_assistant_reasoning_content_roundtrip() {
    let blocks = vec![
        ContentBlock::Thinking(ThinkingBlock {
            thinking: "reasoning".into(),
        }),
        ContentBlock::Text(TextBlock {
            text: "answer".into(),
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    let fmt = OpenAiFormatter;
    let formatted = fmt.format(&[msg]);
    assert_eq!(formatted[0]["reasoning_content"], "reasoning");
    assert_eq!(formatted[0]["content"], "answer");

    let parsed = fmt
        .parse_response(json!({
            "id": "chatcmpl-1",
            "created": 1,
            "choices": [{
                "message": {
                    "role": "assistant",
                    "reasoning_content": "reasoning",
                    "content": "answer"
                }
            }]
        }))
        .unwrap();
    assert_eq!(parsed.get_thinking()[0].thinking, "reasoning");
    assert_eq!(parsed.get_text(), "answer");
}

#[test]
fn test_openai_parse_response_text() {
    let raw = json!({
        "id": "chatcmpl-abc",
        "created": 1700000000,
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "Hello!"
            }
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    });
    let fmt = OpenAiFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    assert_eq!(resp.id, "chatcmpl-abc");
    assert_eq!(resp.get_text(), "Hello!");
    assert_eq!(resp.usage.input_tokens, 10);
    assert_eq!(resp.usage.output_tokens, 5);
    assert_eq!(resp.usage.total_tokens, 15);
}

#[test]
fn test_openai_parse_response_tool_calls() {
    let raw = json!({
        "id": "chatcmpl-xyz",
        "created": 1700000001,
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "my_tool",
                        "arguments": "{\"x\": 1}"
                    }
                }]
            }
        }],
        "usage": {"prompt_tokens": 20, "completion_tokens": 10, "total_tokens": 30}
    });
    let fmt = OpenAiFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    assert!(resp.has_tool_calls());
    let calls = resp.get_tool_calls();
    assert_eq!(calls[0].name, "my_tool");
    assert_eq!(calls[0].input["x"], 1);
}

// -----------------------------------------------------------------------
// DashScopeFormatter
// -----------------------------------------------------------------------

#[test]
fn test_openai_parse_missing_choices() {
    // Response JSON has no "choices" field at all — returns empty content.
    let raw = json!({
        "id": "chatcmpl-no-choices",
        "created": 1700000000,
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    });
    let fmt = OpenAiFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    assert!(resp.content.is_empty());
    assert_eq!(resp.id, "chatcmpl-no-choices");
}

#[test]
fn test_openai_parse_invalid_tool_args() {
    // tool_calls[0].function.arguments is not valid JSON.
    let raw = json!({
        "id": "chatcmpl-bad-args",
        "created": 1700000000,
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_bad",
                    "type": "function",
                    "function": {
                        "name": "broken_tool",
                        "arguments": "NOT VALID JSON {{{{"
                    }
                }]
            }
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    });
    let fmt = OpenAiFormatter;
    // Should not panic — gracefully falls back to empty object.
    let resp = fmt.parse_response(raw).unwrap();
    assert!(resp.has_tool_calls());
    let calls = resp.get_tool_calls();
    assert_eq!(calls[0].name, "broken_tool");
    // input falls back to {"_raw": "<original string>"}
    assert_eq!(calls[0].input, json!({"_raw": "NOT VALID JSON {{{{"}));
    // raw_input preserves original string
    assert_eq!(calls[0].raw_input.as_deref(), Some("NOT VALID JSON {{{{"));
}

#[test]
fn test_openai_format_empty_messages() {
    let fmt = OpenAiFormatter;
    let result = fmt.format(&[]);
    assert!(result.is_empty());
}

#[test]
fn test_openai_parse_null_content_with_tool_calls() {
    // OpenAI sends content=null when there are tool_calls.
    let raw = json!({
        "id": "chatcmpl-null-content",
        "created": 1700000000,
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_100",
                    "type": "function",
                    "function": {
                        "name": "my_tool",
                        "arguments": "{\"key\": \"value\"}"
                    }
                }]
            }
        }],
        "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
    });
    let fmt = OpenAiFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    // No text block (content was null).
    assert_eq!(resp.get_text(), "");
    // Tool call should be present.
    assert!(resp.has_tool_calls());
    let calls = resp.get_tool_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "my_tool");
    assert_eq!(calls[0].input["key"], "value");
}

#[test]
fn test_openai_parse_empty_choices_array() {
    // choices exists but is empty — returns empty content.
    let raw = json!({
        "id": "chatcmpl-empty-choices",
        "created": 1700000000,
        "choices": [],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    });
    let fmt = OpenAiFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    assert!(resp.content.is_empty());
}
