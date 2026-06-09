mod tests {
    use super::super::*;

    // -----------------------------------------------------------------------
    // Robustness / fault-tolerance tests
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
    fn test_dashscope_native_response_format() {
        let raw = json!({
            "request_id": "ds-native-test",
            "output": {
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "native ok"
                    }
                }]
            },
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5,
                "total_tokens": 15
            }
        });
        let fmt = DashScopeFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.id, "ds-native-test");
        assert_eq!(resp.get_text(), "native ok");
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 5);
        assert_eq!(resp.usage.total_tokens, 15);
    }

    #[test]
    fn test_dashscope_openai_compat_format() {
        let raw = json!({
            "id": "ds-compat-test",
            "created": 1700000000,
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "compat ok"
                }
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 6,
                "total_tokens": 18
            }
        });
        let fmt = DashScopeFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.id, "ds-compat-test");
        assert_eq!(resp.get_text(), "compat ok");
        assert_eq!(resp.usage.input_tokens, 12);
        assert_eq!(resp.usage.output_tokens, 6);
    }

    #[test]
    fn test_format_mixed_content_blocks() {
        // A message with Text + ToolUse + Image blocks.
        let blocks = vec![
            ContentBlock::Text(TextBlock {
                text: "Here is the result.".into(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "tu_mix".into(),
                name: "search".into(),
                input: json!({"q": "test"}),
                raw_input: None,
            }),
            ContentBlock::Image(ImageBlock {
                url: Some("https://img.example.com/pic.png".into()),
                data: None,
                mime_type: None,
            }),
        ];
        let msg = Msg::assistant("bot", MsgContent::Blocks(blocks));

        // OpenAI: tool_calls present, text present, image not in tool_calls path
        let openai = OpenAiFormatter;
        let openai_result = openai.format(&[msg.clone()]);
        assert_eq!(openai_result[0]["role"], "assistant");
        let tool_calls = openai_result[0]["tool_calls"].as_array().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["function"]["name"], "search");

        // Anthropic: all blocks appear in content array
        let anthropic = AnthropicFormatter;
        let anthropic_result = anthropic.format(&[msg.clone()]);
        let content = anthropic_result[0]["content"].as_array().unwrap();
        let types: Vec<&str> = content
            .iter()
            .map(|c| c["type"].as_str().unwrap())
            .collect();
        assert!(types.contains(&"text"));
        assert!(types.contains(&"tool_use"));
        assert!(types.contains(&"image"));

        // DashScope: same as OpenAI
        let dashscope = DashScopeFormatter;
        let ds_result = dashscope.format(&[msg]);
        assert_eq!(ds_result[0]["role"], "assistant");
        assert!(ds_result[0]["tool_calls"].as_array().unwrap().len() == 1);
    }

    // -----------------------------------------------------------------------
    // Additional robustness tests
    // -----------------------------------------------------------------------

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

    #[test]
    fn test_dashscope_parse_missing_usage() {
        // DashScope native format without usage field.
        let raw = json!({
            "request_id": "ds-no-usage",
            "output": {
                "choices": [{
                    "message": {"role": "assistant", "content": "no usage here"}
                }]
            }
        });
        let fmt = DashScopeFormatter;
        let resp = fmt.parse_response(raw).unwrap();
        assert_eq!(resp.get_text(), "no usage here");
        // Usage should default to zeros.
        assert_eq!(resp.usage.input_tokens, 0);
        assert_eq!(resp.usage.output_tokens, 0);
        assert_eq!(resp.usage.total_tokens, 0);
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
}
