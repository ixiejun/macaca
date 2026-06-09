// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;
use crate::message::ContentBlock;

#[test]
fn mcp_binary_content_omits_raw_payload() {
    let converted = convert_mcp_content(vec![McpContentValue::Binary {
        mime_type: Some("application/octet-stream".to_string()),
        size_bytes: 42,
        evidence_ref: Some("evidence://mcp/binary".to_string()),
    }]);

    let ContentBlock::Data(block) = &converted.blocks[0] else {
        panic!("binary mcp content should convert to a data block");
    };
    assert_eq!(block.data["payload_omitted"], serde_json::json!(true));
    assert_eq!(block.data["size_bytes"], serde_json::json!(42));
}

#[test]
fn mcp_text_content_is_bounded() {
    let converted = convert_mcp_content(vec![McpContentValue::Text {
        text: "x".repeat(128 * 1024),
    }]);

    assert!(!converted.diagnostics.is_empty());
    let ContentBlock::Text(block) = &converted.blocks[0] else {
        panic!("text mcp content should convert to a text block");
    };
    assert!(block.text.contains("[mcp text truncated]"));
}
