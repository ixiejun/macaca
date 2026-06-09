// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;

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
fn test_data_and_hint_blocks_serialization() {
    let blocks = vec![
        ContentBlock::Data(DataBlock {
            id: Some("data-1".into()),
            data: serde_json::json!({"answer": 42}),
            name: Some("structured_output".into()),
        }),
        ContentBlock::Hint(HintBlock {
            id: Some("hint-1".into()),
            hint: "keep response concise".into(),
            category: Some("middleware".into()),
        }),
    ];
    let json = serde_json::to_string(&blocks).unwrap();
    let deserialized: Vec<ContentBlock> = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.len(), 2);
    assert!(matches!(&deserialized[0], ContentBlock::Data(_)));
    assert!(matches!(&deserialized[1], ContentBlock::Hint(_)));
}

#[test]
fn test_stable_block_ids_are_available_for_all_block_kinds() {
    let text = ContentBlock::Text(TextBlock {
        text: "stable".into(),
    });
    let same_text = ContentBlock::Text(TextBlock {
        text: "stable".into(),
    });
    let tool = ContentBlock::ToolUse(ToolUseBlock {
        id: "call-explicit".into(),
        name: "tool".into(),
        input: serde_json::json!({"ok": true}),
        raw_input: None,
    });
    let data = ContentBlock::Data(DataBlock {
        id: Some("data-explicit".into()),
        data: serde_json::json!({"value": 1}),
        name: None,
    });

    assert_eq!(text.stable_block_id(), same_text.stable_block_id());
    assert!(text.stable_block_id().starts_with("text:"));
    assert_eq!(tool.stable_block_id(), "call-explicit");
    assert_eq!(data.stable_block_id(), "data-explicit");
}

#[test]
fn test_tool_state_projection_and_message_usage_metadata() {
    let tool_use = ContentBlock::ToolUse(ToolUseBlock {
        id: "call-1".into(),
        name: "search".into(),
        input: serde_json::json!({}),
        raw_input: None,
    });
    let tool_result = ContentBlock::ToolResult(ToolResultBlock {
        tool_use_id: "call-1".into(),
        output: "failed".into(),
        name: Some("search".into()),
        is_error: true,
    });
    let usage = MessageUsage {
        input_tokens: 3,
        output_tokens: 5,
        total_tokens: 8,
        duration_ms: Some(12),
    };
    let msg = Msg::assistant("agent", "done")
        .with_generate_reason(GenerateReason::EndTurn)
        .with_usage(usage.clone());

    assert_eq!(tool_use.tool_call_state(), Some(ToolCallState::Completed));
    assert_eq!(
        tool_result.tool_result_state(),
        Some(ToolResultState::Error)
    );
    assert_eq!(msg.generate_reason(), Some(GenerateReason::EndTurn));
    assert_eq!(msg.usage(), Some(usage));
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
fn test_framework_validation_rejects_user_tool_use() {
    let msg = Msg::user(
        "alice",
        MsgContent::Blocks(vec![ContentBlock::ToolUse(ToolUseBlock {
            id: "call-1".into(),
            name: "generic_tool".into(),
            input: serde_json::json!({}),
            raw_input: None,
        })]),
    );

    assert!(matches!(
        msg.validate_for_framework().unwrap_err(),
        MsgValidationError::UserToolUse
    ));
}

#[test]
fn test_framework_validation_rejects_system_non_text() {
    let msg = Msg::new(
        "system",
        MsgContent::Blocks(vec![ContentBlock::Hint(HintBlock {
            id: None,
            hint: "not system text".into(),
            category: None,
        })]),
        Role::System,
    );

    assert!(matches!(
        msg.validate_for_framework().unwrap_err(),
        MsgValidationError::SystemNonText
    ));
}

#[test]
fn test_framework_try_new_accepts_assistant_blocks() {
    let msg = Msg::try_new(
        "assistant",
        MsgContent::Blocks(vec![ContentBlock::Data(DataBlock {
            id: Some("data-1".into()),
            data: serde_json::json!({"ok": true}),
            name: None,
        })]),
        Role::Assistant,
    )
    .unwrap();

    assert_eq!(msg.role, Role::Assistant);
}

#[test]
fn test_strip_thinking_all_thinking() {
    let blocks = vec![ContentBlock::Thinking(ThinkingBlock {
        thinking: "thought".into(),
    })];
    let stripped = MsgContent::Blocks(blocks).strip_thinking();
    assert!(matches!(stripped, MsgContent::Text(s) if s.is_empty()));
}

// -----------------------------------------------------------------------
// Boundary tests
// -----------------------------------------------------------------------

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
    // Verify all 7 blocks are preserved
    match &msg.content {
        MsgContent::Blocks(b) => assert_eq!(b.len(), 7),
        _ => panic!("Expected Blocks"),
    }
    // Serde roundtrip preserves all blocks
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
    // No thinking blocks, so content should be unchanged
    assert_eq!(stripped, MsgContent::Blocks(blocks));
}

#[test]
fn test_get_tool_calls_text_only() {
    let msg = Msg::user("alice", "just text");
    let calls = msg.get_tool_calls();
    assert!(calls.is_empty());

    // Also test Blocks variant with no ToolUse
    let blocks = vec![ContentBlock::Text(TextBlock { text: "hi".into() })];
    let content = MsgContent::Blocks(blocks);
    assert!(content.get_tool_calls().is_empty());
}

#[test]
fn test_large_text_content() {
    // Create >1MB text
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
    // Serde roundtrip preserves special chars
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
    // Both should produce the same text via get_text()
    assert_eq!(text_content.get_text(), blocks_content.get_text());
    assert_eq!(text_content.get_text(), "hello");
}
