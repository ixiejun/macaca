// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.
//
// This implementation is adapted for Macaca Agent OS. It isolates the external
// `Agent` trait projection from the canonical AgentScope 2.0-style event loop.

//! External `Agent` trait adapter for the canonical ReAct runtime.
//!
//! The canonical implementation is event-stream-first. This module keeps the
//! historical `Agent` trait usable for existing consumers by projecting those
//! calls into `AgentRunCommand`. It is intentionally an Adapter layer, not a
//! retained runtime fallback path.

use async_trait::async_trait;
use macaca_proto::AgentId;
use tracing::info;

use crate::agent::{Agent, AgentResult};
use crate::message::Msg;
use crate::runtime_context::{AgentState, RuntimeContext};

use super::{AgentRunCommand, ReActAgent};

#[async_trait]
impl Agent for ReActAgent {
    /// Project the canonical typed event-stream execution into the historical
    /// `Agent::reply` shape.
    ///
    /// This adapter constructs a fresh provider-neutral `RuntimeContext`, runs
    /// the same `AgentRunCommand` path used by service execution, and returns
    /// the reconstructed final assistant message from the event accumulator.
    async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
        let trace_id = format!("framework-reply-{}", uuid::Uuid::new_v4());
        info!(
            agent = %self.name,
            trace_id = %trace_id,
            "projecting Agent::reply through canonical ReActAgent event stream"
        );
        self.run(AgentRunCommand {
            input: msg,
            runtime: RuntimeContext::new(trace_id),
            state: AgentState::empty(),
        })
        .await
        .map(|result| result.final_message)
    }

    /// `observe` does not append to hidden working memory because canonical
    /// execution receives durable state through `AgentRunCommand`. The log makes
    /// the no-op auditable for callers that still use the old trait surface.
    async fn observe(&self, msg: Msg) -> AgentResult<()> {
        info!(
            agent = %self.name,
            observed_message_id = %msg.id,
            "ReActAgent observe projected through canonical per-run state boundary"
        );
        Ok(())
    }

    async fn interrupt(&self, _msg: Msg) -> AgentResult<()> {
        self.request_graceful_shutdown();
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &AgentId {
        &self.id
    }
}
