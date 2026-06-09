// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.
//
// This implementation is adapted for Macaca Agent OS. It defines provider-
// neutral middleware contracts for the canonical AgentScope 2.0-style runtime.

//! AgentScope 2.0-style five-stage middleware contract.
//!
//! This module defines the middleware ABI used by the canonical framework
//! provider. The ReActAgent event-stream loop consumes this pipeline directly.
//!
//! Design patterns:
//! - **Chain of Responsibility**: `AgentMiddlewareChain` executes middleware in
//!   deterministic registration order.
//! - **Decorator**: trace, policy, resource, entitlement, and metering layers
//!   can wrap model/tool side-effect stages through this contract.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, warn};

use crate::agent::{AgentError, AgentResult};
use crate::message::Msg;
use crate::runtime_context::{AgentState, RuntimeContext};

/// AgentScope 2.0 middleware stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MiddlewareStage {
    Agent,
    Reasoning,
    Acting,
    ModelCall,
    SystemPrompt,
}

/// Whether middleware is running before or after the stage body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MiddlewarePhase {
    Before,
    After,
}

/// Mutable context passed through middleware.
///
/// The context carries a `RuntimeContext` reference and optional durable state
/// snapshot. Middleware may rewrite the message, but it must not execute model,
/// tool, filesystem, skill, MCP, or other side effects unless it is itself a
/// service decorator enforcing trace/policy/resource boundaries.
#[derive(Debug, Clone)]
pub struct MiddlewareContext {
    pub stage: MiddlewareStage,
    pub phase: MiddlewarePhase,
    pub runtime: RuntimeContext,
    pub message: Msg,
    pub agent_state: Option<AgentState>,
}

impl MiddlewareContext {
    /// Build a context for one middleware stage transition.
    pub fn new(
        stage: MiddlewareStage,
        phase: MiddlewarePhase,
        runtime: RuntimeContext,
        message: Msg,
    ) -> Self {
        Self {
            stage,
            phase,
            runtime,
            message,
            agent_state: None,
        }
    }

    /// Attach durable state when the caller needs state-aware middleware.
    pub fn with_agent_state(mut self, agent_state: AgentState) -> Self {
        self.agent_state = Some(agent_state);
        self
    }
}

/// Middleware implementation for one or more framework stages.
#[async_trait]
pub trait AgentMiddleware: Send + Sync {
    /// Stable middleware name used in trace logs and diagnostics.
    fn name(&self) -> &str;

    /// Return true when this middleware should run for the context.
    fn supports(&self, _stage: MiddlewareStage, _phase: MiddlewarePhase) -> bool {
        true
    }

    /// Apply middleware to the context. Implementations should return the
    /// updated context instead of mutating external execution state.
    async fn handle(&self, context: MiddlewareContext) -> AgentResult<MiddlewareContext>;
}

/// Deterministic middleware chain.
#[derive(Default)]
pub struct AgentMiddlewareChain {
    middleware: Vec<Arc<dyn AgentMiddleware>>,
}

impl AgentMiddlewareChain {
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    /// Register middleware at the end of the chain.
    pub fn push(&mut self, middleware: Arc<dyn AgentMiddleware>) {
        debug!(
            middleware = middleware.name(),
            "agent middleware registered"
        );
        self.middleware.push(middleware);
    }

    /// Run every middleware that supports the requested stage and phase.
    pub async fn run(&self, mut context: MiddlewareContext) -> AgentResult<MiddlewareContext> {
        for middleware in &self.middleware {
            if !middleware.supports(context.stage, context.phase) {
                continue;
            }
            debug!(
                middleware = middleware.name(),
                stage = ?context.stage,
                phase = ?context.phase,
                trace_id = %context.runtime.trace_id,
                "agent middleware executing"
            );
            context = middleware.handle(context).await?;
        }
        Ok(context)
    }
}

/// Middleware that explicitly fails closed when a required stage is unavailable.
pub struct UnavailableMiddleware {
    name: String,
    reason: String,
}

impl UnavailableMiddleware {
    pub fn new(name: impl Into<String>, reason: impl Into<String>) -> Self {
        let name = name.into();
        let reason = reason.into();
        warn!(
            middleware = %name,
            reason = %reason,
            "unavailable middleware initialized"
        );
        Self { name, reason }
    }
}

#[async_trait]
impl AgentMiddleware for UnavailableMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    async fn handle(&self, _context: MiddlewareContext) -> AgentResult<MiddlewareContext> {
        Err(AgentError::Other(format!(
            "middleware unavailable: {}",
            self.reason
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct RecordingMiddleware {
        name: String,
        records: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl AgentMiddleware for RecordingMiddleware {
        fn name(&self) -> &str {
            &self.name
        }

        async fn handle(&self, context: MiddlewareContext) -> AgentResult<MiddlewareContext> {
            self.records.lock().unwrap().push(self.name.clone());
            Ok(context)
        }
    }

    #[tokio::test]
    async fn middleware_chain_runs_in_registration_order() {
        let records = Arc::new(Mutex::new(Vec::new()));
        let mut chain = AgentMiddlewareChain::new();
        chain.push(Arc::new(RecordingMiddleware {
            name: "first".to_string(),
            records: records.clone(),
        }));
        chain.push(Arc::new(RecordingMiddleware {
            name: "second".to_string(),
            records: records.clone(),
        }));

        let context = MiddlewareContext::new(
            MiddlewareStage::Reasoning,
            MiddlewarePhase::Before,
            RuntimeContext::new("trace-middleware"),
            Msg::user("user", "hello"),
        );
        chain.run(context).await.unwrap();

        assert_eq!(*records.lock().unwrap(), vec!["first", "second"]);
    }

    #[tokio::test]
    async fn unavailable_middleware_fails_closed() {
        let middleware = UnavailableMiddleware::new("required", "missing provider");
        let context = MiddlewareContext::new(
            MiddlewareStage::ModelCall,
            MiddlewarePhase::Before,
            RuntimeContext::new("trace-unavailable"),
            Msg::user("user", "hello"),
        );

        assert!(middleware.handle(context).await.is_err());
    }
}
