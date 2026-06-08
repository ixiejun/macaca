//! Contract tests for [`super::dashscope::DashScopeFormatter`].

use serde_json::json;

use crate::formatter::{DashScopeFormatter, Formatter, OpenAiFormatter};
use crate::message::{ContentBlock, Msg, MsgContent, TextBlock, ToolUseBlock};

#[test]
fn test_dashscope_format_matches_openai() {
    let msg = Msg::user("user", "hi");
    let openai = OpenAiFormatter;
    let dash = DashScopeFormatter;
    assert_eq!(openai.format(&[msg.clone()]), dash.format(&[msg]));
}

#[test]
fn test_dashscope_parse_openai_compatible() {
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
