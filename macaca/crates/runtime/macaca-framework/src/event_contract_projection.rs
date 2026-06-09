// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use crate::message::{
    ContentBlock, DataBlock, TextBlock, ThinkingBlock, ToolResultBlock, ToolUseBlock,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) enum BlockDraft {
    Text {
        block_id: String,
        text: String,
    },
    Thinking {
        block_id: String,
        thinking: String,
    },
    Data {
        block_id: String,
        data: serde_json::Value,
    },
    ToolUse {
        block_id: String,
        tool_call_id: String,
        name: String,
        input: serde_json::Value,
        raw_input: Option<String>,
    },
    ToolResult {
        block_id: String,
        tool_call_id: String,
        name: Option<String>,
        output: String,
        is_error: bool,
    },
}

impl BlockDraft {
    pub(super) fn block_id(&self) -> &str {
        match self {
            BlockDraft::Text { block_id, .. }
            | BlockDraft::Thinking { block_id, .. }
            | BlockDraft::Data { block_id, .. }
            | BlockDraft::ToolUse { block_id, .. }
            | BlockDraft::ToolResult { block_id, .. } => block_id,
        }
    }

    pub(super) fn into_content_block(self) -> ContentBlock {
        match self {
            BlockDraft::Text { text, .. } => ContentBlock::Text(TextBlock { text }),
            BlockDraft::Thinking { thinking, .. } => {
                ContentBlock::Thinking(ThinkingBlock { thinking })
            }
            BlockDraft::Data { block_id, data } => ContentBlock::Data(DataBlock {
                id: Some(block_id),
                data,
                name: None,
            }),
            BlockDraft::ToolUse {
                tool_call_id,
                name,
                input,
                raw_input,
                ..
            } => ContentBlock::ToolUse(ToolUseBlock {
                id: tool_call_id,
                name,
                input,
                raw_input,
            }),
            BlockDraft::ToolResult {
                tool_call_id,
                name,
                output,
                is_error,
                ..
            } => ContentBlock::ToolResult(ToolResultBlock {
                tool_use_id: tool_call_id,
                output,
                name,
                is_error,
            }),
        }
    }
}
