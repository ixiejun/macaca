// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Provider-neutral model formatter, transport, and error contracts.
//!
//! The framework can describe model-family formatting and transport outcomes,
//! but concrete HTTP/WebSocket clients, credentials, endpoint selection, retry
//! budgets, and pricing remain LLM-service or runtime-host responsibilities.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::message::Msg;
use crate::runtime_context::RuntimeContext;

/// Provider-neutral formatter family reference.
///
/// The value is an opaque capability id supplied by provider configuration.
/// Framework code must not branch on specific provider or model names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatterFamilyRef {
    pub family_id: String,
    pub parser_id: String,
}

impl FormatterFamilyRef {
    pub fn new(family_id: impl Into<String>, parser_id: impl Into<String>) -> Self {
        Self {
            family_id: family_id.into(),
            parser_id: parser_id.into(),
        }
    }
}

/// Command for formatting model input through a Strategy implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFormatCommand {
    pub runtime: RuntimeContext,
    pub family: FormatterFamilyRef,
    pub messages: Vec<Msg>,
    #[serde(default)]
    pub options: BTreeMap<String, serde_json::Value>,
}

/// Result of formatter strategy execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelFormatResult {
    pub request_body: serde_json::Value,
    pub formatter_evidence_ref: Option<String>,
}

/// Transport kind requested by a model service adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTransportKind {
    Http,
    WebSocket,
    StreamingHttp,
    ServiceBus,
    Unavailable,
}

/// Typed model transport command. It carries a sanitized request body and an
/// opaque route reference, not credentials or concrete provider clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelTransportCommand {
    pub runtime: RuntimeContext,
    pub route_ref: String,
    pub transport: ModelTransportKind,
    pub request_body: serde_json::Value,
}

/// Stream event returned by a delegated model transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ModelTransportStreamEvent {
    Started { request_id: String },
    Delta { value: serde_json::Value },
    Completed { usage: Option<serde_json::Value> },
    Failed { error: ModelException },
}

/// Provider-neutral model transport result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ModelTransportResult {
    Completed {
        response_body: serde_json::Value,
        usage: Option<serde_json::Value>,
    },
    Streaming {
        events: Vec<ModelTransportStreamEvent>,
    },
    Failed {
        error: ModelException,
    },
}

/// Model exception taxonomy used across formatter, transport, and adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModelException {
    #[error("model authentication failed: {reason}")]
    Authentication { reason: String },
    #[error("model bad request: {reason}")]
    BadRequest { reason: String },
    #[error("model rate limited: {reason}")]
    RateLimit { reason: String },
    #[error("model route not found: {reason}")]
    NotFound { reason: String },
    #[error("model permission denied: {reason}")]
    Permission { reason: String },
    #[error("model timed out: {reason}")]
    Timeout { reason: String },
    #[error("model provider unavailable: {reason}")]
    Unavailable { reason: String },
    #[error("model internal error: {reason}")]
    Internal { reason: String },
    #[error("model provider failure: {reason}")]
    ProviderFailure { reason: String },
    #[error("model capability unsupported: {reason}")]
    Unsupported { reason: String },
}

impl ModelException {
    /// Return a sanitized reason string suitable for logs and snapshots.
    pub fn sanitized_reason(&self) -> &str {
        match self {
            ModelException::Authentication { reason }
            | ModelException::BadRequest { reason }
            | ModelException::RateLimit { reason }
            | ModelException::NotFound { reason }
            | ModelException::Permission { reason }
            | ModelException::Timeout { reason }
            | ModelException::Unavailable { reason }
            | ModelException::Internal { reason }
            | ModelException::ProviderFailure { reason }
            | ModelException::Unsupported { reason } => reason,
        }
    }
}

#[cfg(test)]
#[path = "model_transport_contract_tests.rs"]
mod model_transport_contract_tests;
