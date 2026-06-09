// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::*;
use crate::message::{TextBlock, ToolUseBlock};
use crate::model::{ChatUsage, ModelError};
use crate::permission_contract::ToolPermissionDecision;
use crate::tool::{ToolError, ToolHandler, ToolResponse};

pub(super) struct FakeModel {
    pub(super) responses: Mutex<Vec<ChatResponse>>,
    pub(super) call_count: AtomicUsize,
}

#[async_trait]
impl ChatModel for FakeModel {
    async fn chat(
        &self,
        _messages: Vec<serde_json::Value>,
        _options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        let index = self.call_count.fetch_add(1, Ordering::SeqCst);
        self.responses
            .lock()
            .await
            .get(index)
            .cloned()
            .ok_or_else(|| ModelError::Other("missing fake response".to_string()))
    }

    fn name(&self) -> &str {
        "fake-model"
    }
}

pub(super) struct RetryModel {
    pub(super) fail_until: usize,
    pub(super) call_count: AtomicUsize,
}

#[async_trait]
impl ChatModel for RetryModel {
    async fn chat(
        &self,
        _messages: Vec<serde_json::Value>,
        _options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        let index = self.call_count.fetch_add(1, Ordering::SeqCst);
        if index < self.fail_until {
            Err(ModelError::Timeout)
        } else {
            Ok(text_response("retried"))
        }
    }

    fn name(&self) -> &str {
        "retry-model"
    }
}

pub(super) struct CaptureModel {
    pub(super) captured_messages: Mutex<Vec<serde_json::Value>>,
    pub(super) captured_options: Mutex<Option<ChatOptions>>,
}

#[async_trait]
impl ChatModel for CaptureModel {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        *self.captured_messages.lock().await = messages;
        *self.captured_options.lock().await = Some(options.clone());
        Ok(text_response("captured"))
    }

    fn name(&self) -> &str {
        "capture-model"
    }
}

pub(super) struct EchoTool;

#[async_trait]
impl ToolHandler for EchoTool {
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResponse, ToolError> {
        Ok(ToolResponse::text(args.to_string()))
    }

    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo arguments"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

pub(super) struct CountingTool {
    pub(super) executions: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolHandler for CountingTool {
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResponse, ToolError> {
        self.executions.fetch_add(1, Ordering::SeqCst);
        Ok(ToolResponse::text(args.to_string()))
    }

    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Counting echo arguments"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

pub(super) struct StaticPermissionPolicy {
    pub(super) decision: ToolPermissionDecision,
}

#[async_trait]
impl ToolPermissionPolicy for StaticPermissionPolicy {
    fn name(&self) -> &str {
        "static"
    }

    async fn decide(
        &self,
        _command: ToolPermissionCommand<'_>,
    ) -> AgentResult<ToolPermissionDecision> {
        Ok(self.decision.clone())
    }
}

pub(super) fn fake_model(responses: Vec<ChatResponse>) -> Arc<FakeModel> {
    Arc::new(FakeModel {
        responses: Mutex::new(responses),
        call_count: AtomicUsize::new(0),
    })
}

pub(super) fn retry_model(fail_until: usize) -> Arc<RetryModel> {
    Arc::new(RetryModel {
        fail_until,
        call_count: AtomicUsize::new(0),
    })
}

pub(super) fn text_response(text: &str) -> ChatResponse {
    ChatResponse {
        content: vec![ContentBlock::Text(TextBlock {
            text: text.to_string(),
        })],
        id: "response-text".to_string(),
        created_at: String::new(),
        usage: ChatUsage::default(),
        metadata: None,
    }
}

pub(super) fn tool_response() -> ChatResponse {
    ChatResponse {
        content: vec![ContentBlock::ToolUse(ToolUseBlock {
            id: "call-1".to_string(),
            name: "echo".to_string(),
            input: serde_json::json!({"value": 1}),
            raw_input: None,
        })],
        id: "response-tool".to_string(),
        created_at: String::new(),
        usage: ChatUsage::default(),
        metadata: None,
    }
}
