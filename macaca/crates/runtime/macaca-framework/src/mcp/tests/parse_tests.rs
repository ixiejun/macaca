//! `tools/call` result parsing contract tests.

use crate::message::ContentBlock;

use super::super::parse::parse_call_result;

#[test]
fn test_parse_call_result_basic() {
    let result = serde_json::json!({
        "content": [
            {"type": "text", "text": "hello"},
            {"type": "text", "text": " world"}
        ],
        "isError": false
    });
    let parsed = parse_call_result(&result).unwrap();
    assert!(!parsed.is_error);
    assert_eq!(parsed.content.len(), 2);
}

#[test]
fn test_parse_call_result_error() {
    let result = serde_json::json!({
        "content": [{"type": "text", "text": "something went wrong"}],
        "isError": true
    });
    let parsed = parse_call_result(&result).unwrap();
    assert!(parsed.is_error);
}

#[test]
fn test_parse_call_result_multimodal_and_resource_fallback() {
    let result = serde_json::json!({
        "content": [
            {"type": "image", "data": "abc", "mimeType": "image/png"},
            {"type": "audio", "data": "def", "mimeType": "audio/wav"},
            {"type": "resource", "resource": {"uri": "file://tmp.txt", "text": "resource text"}},
            {"type": "unknown", "value": 1}
        ],
        "isError": false,
        "_meta": {"server": "test"}
    });
    let parsed = parse_call_result(&result).unwrap();
    assert!(!parsed.is_error);
    assert_eq!(parsed.content.len(), 4);
    assert!(matches!(parsed.content[0], ContentBlock::Image(_)));
    assert!(matches!(parsed.content[1], ContentBlock::Audio(_)));
    match &parsed.content[2] {
        ContentBlock::Text(text) => assert_eq!(text.text, "resource text"),
        _ => panic!("expected text resource fallback"),
    }
    match &parsed.content[3] {
        ContentBlock::Text(text) => assert!(text.text.contains("\"unknown\"")),
        _ => panic!("expected json text fallback"),
    }
    assert_eq!(parsed.metadata, Some(serde_json::json!({"server": "test"})));
}
