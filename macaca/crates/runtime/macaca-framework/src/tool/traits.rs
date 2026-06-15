// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Tool extension contracts.
//!
//! These traits form the narrow Adapter boundary between framework agents and
//! concrete tool implementations. Tool providers can be local, service-backed,
//! MCP-backed, or test doubles, but the framework only sees these contracts.

use async_trait::async_trait;
use serde_json::Value;

use super::{ToolError, ToolResponse, ToolTraceEvent};

/// The core trait every tool must implement.
///
/// Implementations must be `Send + Sync` so they can be stored behind a shared
/// reference and invoked from async contexts without leaking provider-specific
/// ownership into the framework.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with the given arguments.
    async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError>;

    /// Execute with streaming trace support.
    ///
    /// Default behavior delegates to `execute`. Service-backed adapters can
    /// override this method to bridge provider-specific trace events into the
    /// provider-neutral `ToolTraceEvent` stream.
    async fn execute_streaming(
        &self,
        args: Value,
        event_tx: Option<tokio::sync::mpsc::UnboundedSender<ToolTraceEvent>>,
    ) -> Result<ToolResponse, ToolError> {
        let _ = event_tx;
        self.execute(args).await
    }

    /// The unique tool name used for lookup and LLM definitions.
    fn name(&self) -> &str;

    /// Human-readable description exposed to the LLM.
    fn description(&self) -> &str;

    /// JSON Schema describing the `args` object expected by `execute`.
    fn schema(&self) -> Value;
}

/// A middleware that wraps every tool invocation.
///
/// `before` runs after argument preparation but before `handler.execute`.
/// `after` runs after `handler.execute` returns successfully. This is the
/// Decorator extension point for cross-cutting behavior such as logging,
/// execution-control adapters, or sanitized event emission.
#[async_trait]
pub trait ToolMiddleware: Send + Sync {
    /// Called before the tool handler executes.
    ///
    /// Implementations may mutate `args` in place, for example to inject safe
    /// defaults. They must not inject application-specific behavior.
    async fn before(&self, name: &str, args: &mut Value) -> Result<(), ToolError>;

    /// Called after the tool handler executes successfully.
    ///
    /// Implementations may mutate `response` in place, for example to append
    /// bounded metadata or emit sanitized trace information.
    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError>;
}

/// Resource owned by a toolkit and cleaned up when the toolkit is dropped.
///
/// Tool handlers can hold shared references to external runtimes such as MCP
/// subprocesses. Registering a cleanup resource makes the ownership explicit:
/// once the agent toolkit goes away, the external runtime is closed too.
pub trait ToolkitResource: Send + Sync {
    fn close(self: Box<Self>);
}
