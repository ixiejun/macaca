//! Contract tests for [`super::anthropic::AnthropicFormatter`].

use serde_json::json;

use crate::formatter::{AnthropicFormatter, Formatter};
use crate::message::{
    ContentBlock, ImageBlock, Msg, MsgContent, Role, TextBlock, ThinkingBlock, ToolResultBlock,
    ToolUseBlock,
};

#[test]
fn test_anthropic_extract_system() {
    let msgs = vec![Msg::system("Be helpful."), Msg::user("alice", "hi")];
    let sys = AnthropicFormatter::extract_system(&msgs);
    assert_eq!(sys.as_deref(), Some("Be helpful."));
}

#[test]
fn test_anthropic_format_excludes_system() {
    let msgs = vec![Msg::system("Be helpful."), Msg::user("alice", "hi")];
    let fmt = AnthropicFormatter;
    let result = fmt.format(&msgs);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["role"], "user");
}

#[test]
fn test_anthropic_format_user_text() {
    let msg = Msg::user("alice", "hello");
    let fmt = AnthropicFormatter;
    let result = fmt.format(&[msg]);
    assert_eq!(result[0]["role"], "user");
    let content = result[0]["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[0]["text"], "hello");
}

#[test]
fn test_anthropic_format_tool_use() {
    let blocks = vec![ContentBlock::ToolUse(ToolUseBlock {
        id: "tu_1".into(),
        name: "calculator".into(),
        input: json!({"expr": "1+1"}),
        raw_input: None,
    })];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    let fmt = AnthropicFormatter;
    let result = fmt.format(&[msg]);
    let content = result[0]["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "tool_use");
    assert_eq!(content[0]["id"], "tu_1");
    assert_eq!(content[0]["name"], "calculator");
}

#[test]
fn test_anthropic_format_tool_result() {
    let blocks = vec![ContentBlock::ToolResult(ToolResultBlock {
        tool_use_id: "tu_1".into(),
        output: "2".into(),
        name: Some("calculator".into()),
        is_error: false,
    })];
    let msg = Msg::new("calculator", MsgContent::Blocks(blocks), Role::Tool);
    let fmt = AnthropicFormatter;
    let result = fmt.format(&[msg]);
    let content = result[0]["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "tool_result");
    assert_eq!(content[0]["tool_use_id"], "tu_1");
    assert_eq!(content[0]["content"], "2");
}

#[test]
fn test_anthropic_format_thinking_preserved() {
    let blocks = vec![
        ContentBlock::Thinking(ThinkingBlock {
            thinking: "hmm".into(),
        }),
        ContentBlock::Text(TextBlock {
            text: "answer".into(),
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    let fmt = AnthropicFormatter;
    let result = fmt.format(&[msg]);
    let content = result[0]["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["type"], "thinking");
    assert_eq!(content[1]["type"], "text");
}

#[test]
fn test_anthropic_parse_response_text() {
    let raw = json!({
        "id": "msg-123",
        "content": [{"type": "text", "text": "Hello from Claude"}],
        "usage": {"input_tokens": 10, "output_tokens": 4}
    });
    let fmt = AnthropicFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    assert_eq!(resp.id, "msg-123");
    assert_eq!(resp.get_text(), "Hello from Claude");
    assert_eq!(resp.usage.input_tokens, 10);
    assert_eq!(resp.usage.output_tokens, 4);
    assert_eq!(resp.usage.total_tokens, 14);
}

#[test]
fn test_anthropic_parse_response_tool_use() {
    let raw = json!({
        "id": "msg-456",
        "content": [{
            "type": "tool_use",
            "id": "toolu_1",
            "name": "search",
            "input": {"query": "rust lang"}
        }],
        "usage": {"input_tokens": 20, "output_tokens": 8}
    });
    let fmt = AnthropicFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    assert!(resp.has_tool_calls());
    let calls = resp.get_tool_calls();
    assert_eq!(calls[0].id, "toolu_1");
    assert_eq!(calls[0].name, "search");
}

#[test]
fn test_anthropic_parse_response_thinking() {
    let raw = json!({
        "id": "msg-789",
        "content": [
            {"type": "thinking", "thinking": "let me think"},
            {"type": "text", "text": "result"}
        ],
        "usage": {"input_tokens": 5, "output_tokens": 3}
    });
    let fmt = AnthropicFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    let thinking = resp.get_thinking();
    assert_eq!(thinking.len(), 1);
    assert_eq!(thinking[0].thinking, "let me think");
    assert_eq!(resp.get_text(), "result");
}

// -----------------------------------------------------------------------
// Robustness / fault-tolerance tests
// -----------------------------------------------------------------------

#[test]
fn test_anthropic_extract_system_none() {
    // No system message present.
    let msgs = vec![Msg::user("alice", "hello"), Msg::assistant("bot", "hi")];
    let sys = AnthropicFormatter::extract_system(&msgs);
    assert!(sys.is_none());
}

#[test]
fn test_anthropic_thinking_preserved() {
    let blocks = vec![
        ContentBlock::Thinking(ThinkingBlock {
            thinking: "deep thoughts".into(),
        }),
        ContentBlock::Text(TextBlock {
            text: "final answer".into(),
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    let fmt = AnthropicFormatter;
    let result = fmt.format(&[msg]);
    let content = result[0]["content"].as_array().unwrap();
    // ThinkingBlock must be preserved in Anthropic format.
    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["type"], "thinking");
    assert_eq!(content[0]["thinking"], "deep thoughts");
    assert_eq!(content[1]["type"], "text");
    assert_eq!(content[1]["text"], "final answer");
}

#[test]
fn test_anthropic_image_base64_and_url() {
    // Base64 image
    let base64_blocks = vec![ContentBlock::Image(ImageBlock {
        data: Some("aGVsbG8=".into()),
        url: None,
        mime_type: Some("image/jpeg".into()),
    })];
    let msg_b64 = Msg::user("alice", MsgContent::Blocks(base64_blocks));
    let fmt = AnthropicFormatter;
    let result = fmt.format(&[msg_b64]);
    let content = result[0]["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "image");
    assert_eq!(content[0]["source"]["type"], "base64");
    assert_eq!(content[0]["source"]["media_type"], "image/jpeg");
    assert_eq!(content[0]["source"]["data"], "aGVsbG8=");

    // URL image
    let url_blocks = vec![ContentBlock::Image(ImageBlock {
        data: None,
        url: Some("https://example.com/photo.png".into()),
        mime_type: None,
    })];
    let msg_url = Msg::user("alice", MsgContent::Blocks(url_blocks));
    let result = fmt.format(&[msg_url]);
    let content = result[0]["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "image");
    assert_eq!(content[0]["source"]["type"], "url");
    assert_eq!(content[0]["source"]["url"], "https://example.com/photo.png");
}

#[test]
fn test_anthropic_parse_unknown_content_type() {
    // Content array has an unknown type — it should be skipped.
    let raw = json!({
        "id": "msg-unknown-type",
        "content": [
            {"type": "text", "text": "Hello"},
            {"type": "server_tool_use", "id": "st_1", "name": "web"},
            {"type": "text", "text": " World"}
        ],
        "usage": {"input_tokens": 5, "output_tokens": 3}
    });
    let fmt = AnthropicFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    // Only the two text blocks survive; the unknown type is skipped.
    assert_eq!(resp.content.len(), 2);
    assert_eq!(resp.get_text(), "Hello World");
}

#[test]
fn test_anthropic_parse_missing_usage() {
    // Anthropic response without usage field.
    let raw = json!({
        "id": "msg-no-usage",
        "content": [{"type": "text", "text": "ok"}]
    });
    let fmt = AnthropicFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    assert_eq!(resp.get_text(), "ok");
    assert_eq!(resp.usage.input_tokens, 0);
    assert_eq!(resp.usage.output_tokens, 0);
}

#[test]
fn test_anthropic_parse_missing_stop_reason() {
    // stop_reason field is absent — should not panic.
    let raw = json!({
        "id": "msg-no-stop",
        "content": [{"type": "text", "text": "partial"}],
        "usage": {"input_tokens": 1, "output_tokens": 1}
    });
    let fmt = AnthropicFormatter;
    let resp = fmt.parse_response(raw).unwrap();
    assert_eq!(resp.get_text(), "partial");
}
