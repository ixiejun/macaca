mod tests {
    use super::super::*;
    use crate::message::{ContentBlock, MsgContent, TextBlock, ThinkingBlock, ToolUseBlock};

    // Helper: build a minimal AgentCard.
    fn sample_card() -> AgentCard {
        AgentCard {
            name: "test-agent".into(),
            url: "http://localhost:9000".into(),
            version: "1.0.0".into(),
            description: Some("A test agent".into()),
            capabilities: AgentCapabilities::default(),
            default_input_modes: vec!["text/plain".into()],
            default_output_modes: vec!["text/plain".into()],
            skills: vec![AgentSkillInfo {
                name: "echo".into(),
                description: Some("Echoes back the input".into()),
            }],
        }
    }

    // 1. AgentCard JSON round-trip
    #[test]
    fn test_agent_card_serialization() {
        let card = sample_card();
        let json = serde_json::to_string(&card).unwrap();
        let back: AgentCard = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, card.name);
        assert_eq!(back.url, card.url);
        assert_eq!(back.version, card.version);
        assert_eq!(back.description, card.description);
        assert_eq!(back.capabilities.streaming, true);
        assert_eq!(back.capabilities.push_notifications, false);
        assert_eq!(back.default_input_modes, card.default_input_modes);
        assert_eq!(back.skills.len(), 1);
        assert_eq!(back.skills[0].name, "echo");
    }

    // 2. A2AMessage JSON round-trip
    #[test]
    fn test_a2a_message_serialization() {
        let msg = A2AMessage {
            message_id: "msg-001".into(),
            role: A2ARole::User,
            parts: vec![
                A2APart::Text {
                    text: "Hello!".into(),
                },
                A2APart::Data {
                    data: serde_json::json!({"key": "value"}),
                },
            ],
            context_id: Some("ctx-1".into()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: A2AMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.message_id, "msg-001");
        assert_eq!(back.role, A2ARole::User);
        assert_eq!(back.parts.len(), 2);
        assert_eq!(back.context_id, Some("ctx-1".into()));
    }

    // 3. Task states serialization
    #[test]
    fn test_a2a_task_states() {
        let states = [
            A2ATaskState::Submitted,
            A2ATaskState::Working,
            A2ATaskState::Completed,
            A2ATaskState::Failed,
            A2ATaskState::Canceled,
        ];
        let expected_strs = ["submitted", "working", "completed", "failed", "canceled"];
        for (state, expected) in states.iter().zip(expected_strs.iter()) {
            let json = serde_json::to_string(state).unwrap();
            assert_eq!(json, format!("\"{}\"", expected));
            let back: A2ATaskState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, *state);
        }

        // Full task round-trip
        let task = A2ATask {
            id: "task-1".into(),
            context_id: None,
            status: A2ATaskStatus {
                state: A2ATaskState::Working,
                message: None,
            },
            artifacts: vec![],
        };
        let json = serde_json::to_string(&task).unwrap();
        let back: A2ATask = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "task-1");
        assert_eq!(back.status.state, A2ATaskState::Working);
    }

    // 4. Formatter: text Msg → A2AMessage
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

    // 5. Formatter: tool-use Msg → A2AMessage (Data part)
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

    // 6. Formatter: A2AMessage → Msg (reverse)
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

    // 7. Formatter: ThinkingBlock is stripped
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
        // Only the text part survives; thinking is stripped.
        assert_eq!(a2a.parts.len(), 1);
        match &a2a.parts[0] {
            A2APart::Text { text } => assert_eq!(text, "Visible answer."),
            other => panic!("Expected text part, got {:?}", other),
        }
    }

    // 8. FileCardResolver reads a JSON file
    #[tokio::test]
    async fn test_file_card_resolver() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        let card_json = serde_json::to_string(&sample_card()).unwrap();
        tmp.write_all(card_json.as_bytes()).unwrap();

        let resolver = FileCardResolver::new(tmp.path());
        let card = resolver.resolve().await.unwrap();
        assert_eq!(card.name, "test-agent");
        assert_eq!(card.url, "http://localhost:9000");
        assert_eq!(card.skills.len(), 1);
    }

    // 9. SendMessageRequest serialization
    #[test]
    fn test_send_message_request() {
        let req = SendMessageRequest {
            id: "req-001".into(),
            message: A2AMessage {
                message_id: "msg-002".into(),
                role: A2ARole::User,
                parts: vec![A2APart::Text {
                    text: "Do something".into(),
                }],
                context_id: None,
            },
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: SendMessageRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "req-001");
        assert_eq!(back.message.message_id, "msg-002");
        assert_eq!(back.message.role, A2ARole::User);
        match &back.message.parts[0] {
            A2APart::Text { text } => assert_eq!(text, "Do something"),
            other => panic!("Expected text, got {:?}", other),
        }
    }

    // 10. to_a2a → from_a2a roundtrip for plain text
    #[test]
    fn test_to_a2a_from_a2a_roundtrip_text() {
        let msg = Msg::user("alice", "Hello, round trip!");
        let a2a = A2AFormatter::to_a2a(&msg);
        let back = A2AFormatter::from_a2a("alice", &a2a).unwrap();

        assert_eq!(back.get_text(), "Hello, round trip!");
        assert_eq!(back.role, Role::User);
    }

    // 11. ThinkingBlock is stripped in to_a2a (explicit roundtrip check)
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

        // Only one part (text), thinking stripped.
        assert_eq!(a2a.parts.len(), 1);
        match &a2a.parts[0] {
            A2APart::Text { text } => assert_eq!(text, "public answer"),
            other => panic!("Expected text, got {:?}", other),
        }
    }

    // 12. ToolUse roundtrip through A2A
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

    // 13. AgentCard full serde roundtrip with all fields
    #[test]
    fn test_agent_card_serde() {
        let card = AgentCard {
            name: "multi-skill".into(),
            url: "https://example.com/a2a".into(),
            version: "2.1.0".into(),
            description: Some("An agent with many skills".into()),
            capabilities: AgentCapabilities {
                streaming: true,
                push_notifications: true,
                state_transition_history: true,
            },
            default_input_modes: vec!["text/plain".into(), "application/json".into()],
            default_output_modes: vec!["text/plain".into()],
            skills: vec![
                AgentSkillInfo {
                    name: "search".into(),
                    description: Some("Search the web".into()),
                },
                AgentSkillInfo {
                    name: "compute".into(),
                    description: None,
                },
            ],
        };
        let json = serde_json::to_string(&card).unwrap();
        let back: AgentCard = serde_json::from_str(&json).unwrap();

        assert_eq!(back.name, "multi-skill");
        assert_eq!(back.version, "2.1.0");
        assert_eq!(back.description, Some("An agent with many skills".into()));
        assert!(back.capabilities.push_notifications);
        assert!(back.capabilities.state_transition_history);
        assert_eq!(back.default_input_modes.len(), 2);
        assert_eq!(back.skills.len(), 2);
        assert_eq!(back.skills[1].name, "compute");
        assert!(back.skills[1].description.is_none());
    }

    // 14. All A2ATaskState variants serde roundtrip
    #[test]
    fn test_a2a_task_state_variants() {
        let all_states = vec![
            A2ATaskState::Submitted,
            A2ATaskState::Working,
            A2ATaskState::Completed,
            A2ATaskState::Failed,
            A2ATaskState::Canceled,
        ];
        for state in &all_states {
            let json = serde_json::to_value(state).unwrap();
            let back: A2ATaskState = serde_json::from_value(json).unwrap();
            assert_eq!(&back, state);
        }
    }

    // 15. Empty parts A2AMessage doesn't panic
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

        // from_a2a with empty parts should not panic.
        let internal = A2AFormatter::from_a2a("agent", &msg).unwrap();
        // Blocks variant with empty vec.
        match &internal.content {
            MsgContent::Blocks(b) => assert!(b.is_empty()),
            _ => panic!("Expected empty Blocks content"),
        }
    }

    // 16. Image roundtrip through A2A
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

        // from_a2a should reconstruct the ImageBlock.
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

    // 17. from_a2a rejects unknown data type
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

    // 18. from_a2a rejects tool_use missing name
    #[test]
    fn test_from_a2a_missing_tool_use_fields() {
        let a2a = A2AMessage {
            message_id: "m-no-name".into(),
            role: A2ARole::Agent,
            parts: vec![A2APart::Data {
                data: serde_json::json!({ "kind": "tool_use", "id": "call_1", "input": {} }),
                // "name" is missing
            }],
            context_id: None,
        };
        let result = A2AFormatter::from_a2a("agent", &a2a);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), A2AError::MissingField(_)));
    }

    // 19. from_a2a rejects tool_result missing tool_use_id
    #[test]
    fn test_from_a2a_missing_tool_result_fields() {
        let a2a = A2AMessage {
            message_id: "m-no-tuid".into(),
            role: A2ARole::Agent,
            parts: vec![A2APart::Data {
                data: serde_json::json!({ "kind": "tool_result", "output": "ok" }),
                // "tool_use_id" is missing
            }],
            context_id: None,
        };
        let result = A2AFormatter::from_a2a("agent", &a2a);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), A2AError::MissingField(_)));
    }
}
