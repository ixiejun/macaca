//! Contract tests for the message module tree (serde roundtrips and boundaries).

use super::{
    AudioBlock, ContentBlock, ImageBlock, Msg, MsgContent, Role, TextBlock, ThinkingBlock,
    ToolResultBlock, ToolUseBlock, VideoBlock,
};

#[test]
fn test_msg_user_text() {
    let msg = Msg::user("alice", "hello world");
    assert_eq!(msg.name, "alice");
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.get_text(), "hello world");
    assert!(!msg.content.has_tool_calls());
}

#[test]
fn test_msg_with_blocks() {
    let blocks = vec![
        ContentBlock::Text(TextBlock {
            text: "Let me help.".into(),
        }),
        ContentBlock::ToolUse(ToolUseBlock {
            id: "call_1".into(),
            name: "search".into(),
            input: serde_json::json!({"query": "rust"}),
            raw_input: None,
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    assert_eq!(msg.get_text(), "Let me help.");
    assert!(msg.content.has_tool_calls());
    assert_eq!(msg.get_tool_calls().len(), 1);
    assert_eq!(msg.get_tool_calls()[0].name, "search");
}

#[test]
fn test_strip_thinking() {
    let blocks = vec![
        ContentBlock::Thinking(ThinkingBlock {
            thinking: "hmm...".into(),
        }),
        ContentBlock::Text(TextBlock {
            text: "Here's the answer.".into(),
        }),
    ];
    let content = MsgContent::Blocks(blocks);
    let stripped = content.strip_thinking();
    match &stripped {
        MsgContent::Blocks(b) => {
            assert_eq!(b.len(), 1);
            assert!(matches!(&b[0], ContentBlock::Text(_)));
        }
        _ => panic!("Expected blocks"),
    }
}

#[test]
fn test_tool_result_msg() {
    let msg = Msg::tool_result("call_1", "search", "found 5 results", false);
    assert_eq!(msg.role, Role::Tool);
    assert_eq!(msg.name, "search");
    match &msg.content {
        MsgContent::Blocks(blocks) => {
            assert_eq!(blocks.len(), 1);
            match &blocks[0] {
                ContentBlock::ToolResult(r) => {
                    assert_eq!(r.tool_use_id, "call_1");
                    assert!(!r.is_error);
                }
                _ => panic!("Expected ToolResult"),
            }
        }
        _ => panic!("Expected Blocks"),
    }
}

#[test]
fn test_msg_serialization_roundtrip() {
    let msg = Msg::user("alice", "hello");
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Msg = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "alice");
    assert_eq!(deserialized.get_text(), "hello");
    assert_eq!(deserialized.role, Role::User);
}

#[test]
fn test_content_block_serialization() {
    let block = ContentBlock::ToolUse(ToolUseBlock {
        id: "c1".into(),
        name: "file_read".into(),
        input: serde_json::json!({"path": "/tmp/test.txt"}),
        raw_input: None,
    });
    let json = serde_json::to_string(&block).unwrap();
    assert!(json.contains("\"type\":\"tool_use\""));
    let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(block, deserialized);
}

#[test]
fn test_msg_content_from_string() {
    let content: MsgContent = "hello".into();
    assert_eq!(content.get_text(), "hello");
    assert!(!content.has_tool_calls());
}

#[test]
fn test_empty_content() {
    assert!(MsgContent::Text(String::new()).is_empty());
    assert!(MsgContent::Blocks(vec![]).is_empty());
    assert!(!MsgContent::Text("hi".into()).is_empty());
}

#[test]
fn test_strip_thinking_all_thinking() {
    let blocks = vec![ContentBlock::Thinking(ThinkingBlock {
        thinking: "thought".into(),
    })];
    let stripped = MsgContent::Blocks(blocks).strip_thinking();
    assert!(matches!(stripped, MsgContent::Text(s) if s.is_empty()));
}

#[test]
fn test_empty_content_msg_serde_roundtrip() {
    let msg = Msg::user("alice", "");
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: Msg = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.get_text(), "");
    assert_eq!(deserialized.name, "alice");
    assert_eq!(deserialized.role, Role::User);
    assert!(deserialized.content.is_empty());
}

#[test]
fn test_all_seven_content_blocks_mixed() {
    let blocks = vec![
        ContentBlock::Text(TextBlock {
            text: "hello".into(),
        }),
        ContentBlock::Thinking(ThinkingBlock {
            thinking: "hmm".into(),
        }),
        ContentBlock::ToolUse(ToolUseBlock {
            id: "t1".into(),
            name: "search".into(),
            input: serde_json::json!({}),
            raw_input: None,
        }),
        ContentBlock::ToolResult(ToolResultBlock {
            tool_use_id: "t1".into(),
            output: "done".into(),
            name: Some("search".into()),
            is_error: false,
        }),
        ContentBlock::Image(ImageBlock {
            data: Some("base64data".into()),
            url: None,
            mime_type: Some("image/png".into()),
        }),
        ContentBlock::Audio(AudioBlock {
            data: None,
            url: Some("https://example.com/audio.mp3".into()),
            mime_type: Some("audio/mp3".into()),
        }),
        ContentBlock::Video(VideoBlock {
            data: None,
            url: Some("https://example.com/video.mp4".into()),
            mime_type: Some("video/mp4".into()),
        }),
    ];
    let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));
    match &msg.content {
        MsgContent::Blocks(b) => assert_eq!(b.len(), 7),
        _ => panic!("Expected Blocks"),
    }
    let json = serde_json::to_string(&msg).unwrap();
    let de: Msg = serde_json::from_str(&json).unwrap();
    match &de.content {
        MsgContent::Blocks(b) => assert_eq!(b.len(), 7),
        _ => panic!("Expected Blocks after roundtrip"),
    }
}

#[test]
fn test_strip_thinking_idempotent() {
    let blocks = vec![
        ContentBlock::Text(TextBlock { text: "a".into() }),
        ContentBlock::ToolUse(ToolUseBlock {
            id: "t1".into(),
            name: "x".into(),
            input: serde_json::json!(null),
            raw_input: None,
        }),
    ];
    let content = MsgContent::Blocks(blocks.clone());
    let stripped = content.strip_thinking();
    assert_eq!(stripped, MsgContent::Blocks(blocks));
}

#[test]
fn test_get_tool_calls_text_only() {
    let msg = Msg::user("alice", "just text");
    let calls = msg.get_tool_calls();
    assert!(calls.is_empty());

    let blocks = vec![ContentBlock::Text(TextBlock { text: "hi".into() })];
    let content = MsgContent::Blocks(blocks);
    assert!(content.get_tool_calls().is_empty());
}

#[test]
fn test_large_text_content() {
    let large_text = "A".repeat(1_100_000);
    let msg = Msg::user("alice", large_text.as_str());
    let json = serde_json::to_string(&msg).unwrap();
    let de: Msg = serde_json::from_str(&json).unwrap();
    assert_eq!(de.get_text().len(), 1_100_000);
    assert_eq!(de.get_text(), large_text);
}

#[test]
fn test_metadata_nested_json() {
    let nested = serde_json::json!({
        "level1": {
            "level2": {
                "key": "value",
                "numbers": [1, 2, 3]
            }
        },
        "tags": ["a", "b", "c"],
        "flag": true,
        "count": 42
    });
    let msg = Msg::user("alice", "hi").with_metadata(nested.clone());
    let json = serde_json::to_string(&msg).unwrap();
    let de: Msg = serde_json::from_str(&json).unwrap();
    assert_eq!(de.metadata, nested);
}

#[test]
fn test_special_characters_unicode() {
    let text = "Hello 🎉 世界 \u{200B} café ñ ü \t\n emoji: 🚀🌍";
    let msg = Msg::user("alice", text);
    assert_eq!(msg.get_text(), text);
    let json = serde_json::to_string(&msg).unwrap();
    let de: Msg = serde_json::from_str(&json).unwrap();
    assert_eq!(de.get_text(), text);
}

#[test]
fn test_tool_result_construction() {
    let msg = Msg::tool_result("call_42", "my_tool", "output data", true);
    assert_eq!(msg.role, Role::Tool);
    assert_eq!(msg.name, "my_tool");
    match &msg.content {
        MsgContent::Blocks(blocks) => {
            assert_eq!(blocks.len(), 1);
            match &blocks[0] {
                ContentBlock::ToolResult(r) => {
                    assert_eq!(r.tool_use_id, "call_42");
                    assert_eq!(r.output, "output data");
                    assert_eq!(r.name, Some("my_tool".into()));
                    assert!(r.is_error);
                }
                _ => panic!("Expected ToolResult block"),
            }
        }
        _ => panic!("Expected Blocks content"),
    }
}

#[test]
fn test_msg_content_text_vs_blocks_text() {
    let text_content = MsgContent::Text("hello".into());
    let blocks_content = MsgContent::Blocks(vec![ContentBlock::Text(TextBlock {
        text: "hello".into(),
    })]);
    assert_eq!(text_content.get_text(), blocks_content.get_text());
    assert_eq!(text_content.get_text(), "hello");
}
