//! Contract tests for [`A2AFormatter`] (Adapter: internal `Msg` ↔ A2A wire types).

use crate::a2a::{A2AError, A2AFormatter, A2AMessage, A2APart, A2ARole};
use crate::message::{
    ContentBlock, ImageBlock, Msg, MsgContent, Role, TextBlock, ThinkingBlock, ToolUseBlock,
};

#[test]
fn test_formatter_text_to_a2a() {
    let msg = Msg::user("alice", "Hello, agent!");
    let a2a = A2AFormatter::to_a2a(&msg);
    assert_eq!(a2a.message_id, msg.id);
    assert_eq!(a2a.role, A2ARole::User);
    assert_eq!(a2a.parts.len(), 1);
    match &a2a.parts[0] {
        A2APart::Text { text } => assert_eq!(text, "Hello, agent!"),
        other => panic!("Expected text part, got {:?}", other),
    }
}

#[test]
fn test_formatter_tool_use_to_a2a() {
    let blocks = vec![
        ContentBlock::Text(TextBlock {
            text: "Calling tool.".into(),
        }),
        ContentBlock::ToolUse(ToolUseBlock {
            id: "call_42".into(),
            name: "search".into(),
            input: serde_json::json!({"query": "rust a2a"}),
            raw_input: None,
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    let a2a = A2AFormatter::to_a2a(&msg);
    assert_eq!(a2a.role, A2ARole::Agent);
    assert_eq!(a2a.parts.len(), 2);
    match &a2a.parts[1] {
        A2APart::Data { data } => {
            assert_eq!(data["kind"], "tool_use");
            assert_eq!(data["id"], "call_42");
            assert_eq!(data["name"], "search");
        }
        other => panic!("Expected Data part, got {:?}", other),
    }
}

#[test]
fn test_formatter_a2a_to_msg() {
    let a2a = A2AMessage {
        message_id: "m-1".into(),
        role: A2ARole::Agent,
        parts: vec![A2APart::Text {
            text: "I found it.".into(),
        }],
        context_id: None,
    };
    let msg = A2AFormatter::from_a2a("remote-agent", &a2a).unwrap();
    assert_eq!(msg.name, "remote-agent");
    assert_eq!(msg.role, Role::Assistant);
    assert_eq!(msg.get_text(), "I found it.");
}

#[test]
fn test_formatter_strips_thinking() {
    let blocks = vec![
        ContentBlock::Thinking(ThinkingBlock {
            thinking: "internal thought".into(),
        }),
        ContentBlock::Text(TextBlock {
            text: "Visible answer.".into(),
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    let a2a = A2AFormatter::to_a2a(&msg);
    assert_eq!(a2a.parts.len(), 1);
    match &a2a.parts[0] {
        A2APart::Text { text } => assert_eq!(text, "Visible answer."),
        other => panic!("Expected text part, got {:?}", other),
    }
}

#[test]
fn test_to_a2a_from_a2a_roundtrip_text() {
    let msg = Msg::user("alice", "Hello, round trip!");
    let a2a = A2AFormatter::to_a2a(&msg);
    let back = A2AFormatter::from_a2a("alice", &a2a).unwrap();

    assert_eq!(back.get_text(), "Hello, round trip!");
    assert_eq!(back.role, Role::User);
}

#[test]
fn test_to_a2a_strips_thinking() {
    let blocks = vec![
        ContentBlock::Thinking(ThinkingBlock {
            thinking: "secret reasoning".into(),
        }),
        ContentBlock::Text(TextBlock {
            text: "public answer".into(),
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    let a2a = A2AFormatter::to_a2a(&msg);

    assert_eq!(a2a.parts.len(), 1);
    match &a2a.parts[0] {
        A2APart::Text { text } => assert_eq!(text, "public answer"),
        other => panic!("Expected text, got {:?}", other),
    }
}

#[test]
fn test_to_a2a_from_a2a_tool_use() {
    let blocks = vec![
        ContentBlock::Text(TextBlock {
            text: "Invoking tool.".into(),
        }),
        ContentBlock::ToolUse(ToolUseBlock {
            id: "call_99".into(),
            name: "code_search".into(),
            input: serde_json::json!({"pattern": "fn main"}),
            raw_input: None,
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    let a2a = A2AFormatter::to_a2a(&msg);
    let back = A2AFormatter::from_a2a("bot", &a2a).unwrap();

    assert_eq!(back.role, Role::Assistant);
    let tool_calls = back.get_tool_calls();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id, "call_99");
    assert_eq!(tool_calls[0].name, "code_search");
    assert_eq!(
        tool_calls[0].input,
        serde_json::json!({"pattern": "fn main"})
    );
}

#[test]
fn test_empty_parts_a2a_message() {
    let msg = A2AMessage {
        message_id: "empty-msg".into(),
        role: A2ARole::Agent,
        parts: vec![],
        context_id: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: A2AMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(back.parts.len(), 0);

    let internal = A2AFormatter::from_a2a("agent", &msg).unwrap();
    match &internal.content {
        MsgContent::Blocks(b) => assert!(b.is_empty()),
        _ => panic!("Expected empty Blocks content"),
    }
}

#[test]
fn test_a2a_roundtrip_with_image() {
    let blocks = vec![ContentBlock::Image(ImageBlock {
        data: Some("iVBORw0KGgo=".into()),
        url: Some("https://example.com/img.png".into()),
        mime_type: Some("image/png".into()),
    })];
    let msg = Msg::user("alice", MsgContent::Blocks(blocks));
    let a2a = A2AFormatter::to_a2a(&msg);

    assert_eq!(a2a.parts.len(), 1);
    match &a2a.parts[0] {
        A2APart::File { file } => {
            assert_eq!(file.uri.as_deref(), Some("https://example.com/img.png"));
            assert_eq!(file.bytes.as_deref(), Some("iVBORw0KGgo="));
            assert_eq!(file.mime_type.as_deref(), Some("image/png"));
        }
        other => panic!("Expected File part, got {:?}", other),
    }

    let back = A2AFormatter::from_a2a("alice", &a2a).unwrap();
    match &back.content {
        MsgContent::Blocks(blocks) => {
            assert_eq!(blocks.len(), 1);
            match &blocks[0] {
                ContentBlock::Image(img) => {
                    assert_eq!(img.url.as_deref(), Some("https://example.com/img.png"));
                    assert_eq!(img.data.as_deref(), Some("iVBORw0KGgo="));
                    assert_eq!(img.mime_type.as_deref(), Some("image/png"));
                }
                other => panic!("Expected ImageBlock, got {:?}", other),
            }
        }
        other => panic!("Expected Blocks, got {:?}", other),
    }
}

#[test]
fn test_from_a2a_invalid_data_type() {
    let a2a = A2AMessage {
        message_id: "m-bad-type".into(),
        role: A2ARole::Agent,
        parts: vec![A2APart::Data {
            data: serde_json::json!({ "kind": "unknown", "foo": "bar" }),
        }],
        context_id: None,
    };
    let result = A2AFormatter::from_a2a("agent", &a2a);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, A2AError::InvalidDataType(_)));
    assert!(err.to_string().contains("unknown"));
}

#[test]
fn test_from_a2a_missing_tool_use_fields() {
    let a2a = A2AMessage {
        message_id: "m-no-name".into(),
        role: A2ARole::Agent,
        parts: vec![A2APart::Data {
            data: serde_json::json!({ "kind": "tool_use", "id": "call_1", "input": {} }),
        }],
        context_id: None,
    };
    let result = A2AFormatter::from_a2a("agent", &a2a);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), A2AError::MissingField(_)));
}

#[test]
fn test_from_a2a_missing_tool_result_fields() {
    let a2a = A2AMessage {
        message_id: "m-no-tuid".into(),
        role: A2ARole::Agent,
        parts: vec![A2APart::Data {
            data: serde_json::json!({ "kind": "tool_result", "output": "ok" }),
        }],
        context_id: None,
    };
    let result = A2AFormatter::from_a2a("agent", &a2a);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), A2AError::MissingField(_)));
}
