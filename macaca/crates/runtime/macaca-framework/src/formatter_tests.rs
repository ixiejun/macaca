mod tests {
    use super::super::*;
    use crate::message::{
        ContentBlock, ImageBlock, Msg, MsgContent, TextBlock, ThinkingBlock, ToolResultBlock,
        ToolUseBlock,
    };

    // -----------------------------------------------------------------------
    // OpenAiFormatter
    // -----------------------------------------------------------------------

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
    fn test_dashscope_format_matches_openai() {
        let msg = Msg::user("user", "hi");
        let openai = OpenAiFormatter;
        let dash = DashScopeFormatter;
        assert_eq!(openai.format(&[msg.clone()]), dash.format(&[msg]));
    }

    #[test]
    fn test_dashscope_parse_openai_wire_shape() {
        let raw = json!({
            "id": "ds-1",
            "created": 1700000002,
            "choices": [{
                "message": {"role": "assistant", "content": "ok"}
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7}
        });
        let fmt = DashScopeFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.get_text(), "ok");
    }

    #[test]
    fn test_dashscope_parse_native_format() {
        let raw = json!({
            "request_id": "ds-native-1",
            "output": {
                "choices": [{
                    "message": {"role": "assistant", "content": "native response"}
                }]
            },
            "usage": {
                "input_tokens": 8,
                "output_tokens": 3,
                "total_tokens": 11
            }
        });
        let fmt = DashScopeFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.id, "ds-native-1");
        assert_eq!(resp.get_text(), "native response");
        assert_eq!(resp.usage.input_tokens, 8);
    }

    // -----------------------------------------------------------------------
    // AnthropicFormatter
    // -----------------------------------------------------------------------

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
}
