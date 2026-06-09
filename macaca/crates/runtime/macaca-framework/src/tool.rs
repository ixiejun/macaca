//! Tool system — registration, middleware, and execution.
//!
//! Provides a toolkit abstraction for managing agent tools:
//! - `ToolHandler` trait for implementing tools
//! - `ToolMiddleware` for cross-cutting concerns (logging, rate limiting, etc.)
//! - `ToolGroup` for activating/deactivating sets of tools
//! - `Toolkit` as the central registry that wires everything together

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::message::{ContentBlock, TextBlock};

#[path = "tool_schema.rs"]
mod tool_schema_impl;

pub use crate::tool_schema;

/// Provider-neutral trace event emitted during delegated tool execution.
///
/// Framework owns this DTO so `Toolkit` can route trace evidence without
/// depending on concrete tool-service crates. Shell or runtime-host adapters
/// are responsible for converting service-specific trace events into this
/// bounded, sanitized shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolTraceEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

// ---------------------------------------------------------------------------
// ToolResponse
// ---------------------------------------------------------------------------

/// The result returned by a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResponse {
    /// Content blocks produced by the tool.
    pub content: Vec<ContentBlock>,
    /// Optional arbitrary metadata (e.g. token counts, latency).
    pub metadata: Option<Value>,
    /// Whether the response is a streaming chunk.
    pub is_stream: bool,
    /// Whether this is the final chunk of a stream.
    pub is_last: bool,
    /// Whether the execution was interrupted mid-stream.
    pub is_interrupted: bool,
}

impl ToolResponse {
    /// Construct a plain-text response.
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text(TextBlock { text: s.into() })],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        }
    }

    /// Construct an error response (text block, marks the call as failed).
    pub fn error(s: impl Into<String>) -> Self {
        // We use the same TextBlock representation; callers distinguish errors
        // via the Err variant of `call_tool`. This constructor is for cases
        // where the handler wants to return a structured error message instead
        // of propagating a `ToolError`.
        Self {
            content: vec![ContentBlock::Text(TextBlock {
                text: format!("Error: {}", s.into()),
            })],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        }
    }

    /// Construct a response whose text is the JSON-serialized form of `value`.
    pub fn json(value: Value) -> Self {
        Self {
            content: vec![ContentBlock::Text(TextBlock {
                text: value.to_string(),
            })],
            metadata: None,
            is_stream: false,
            is_last: true,
            is_interrupted: false,
        }
    }
}

// ---------------------------------------------------------------------------
// ToolError
// ---------------------------------------------------------------------------

/// Errors that can occur during tool lookup or execution.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout")]
    Timeout,
}

// ---------------------------------------------------------------------------
// ToolHandler trait
// ---------------------------------------------------------------------------

/// The core trait every tool must implement.
///
/// Implementations must be `Send + Sync` so they can be stored behind a
/// shared reference and invoked from async contexts.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with the given arguments.
    async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError>;

    /// Execute with streaming trace support.
    ///
    /// Default behavior delegates to `execute`. Service-backed adapters can
    /// override this to bridge provider-specific trace events into the
    /// provider-neutral `ToolTraceEvent` stream.
    async fn execute_streaming(
        &self,
        args: Value,
        event_tx: Option<tokio::sync::mpsc::UnboundedSender<ToolTraceEvent>>,
    ) -> Result<ToolResponse, ToolError> {
        let _ = event_tx;
        self.execute(args).await
    }

    /// The unique tool name (used for lookup and LLM definitions).
    fn name(&self) -> &str;

    /// Human-readable description exposed to the LLM.
    fn description(&self) -> &str;

    /// JSON Schema describing the `args` object expected by `execute`.
    fn schema(&self) -> Value;
}

// ---------------------------------------------------------------------------
// ToolMiddleware trait
// ---------------------------------------------------------------------------

/// A middleware that wraps every tool invocation.
///
/// `before` runs after argument preparation but before `handler.execute`.
/// `after` runs after `handler.execute` returns successfully.
#[async_trait]
pub trait ToolMiddleware: Send + Sync {
    /// Called before the tool handler executes.
    ///
    /// Implementations may mutate `args` in place (e.g. inject defaults).
    async fn before(&self, name: &str, args: &mut Value) -> Result<(), ToolError>;

    /// Called after the tool handler executes successfully.
    ///
    /// Implementations may mutate `response` in place (e.g. append metadata).
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

// ---------------------------------------------------------------------------
// ToolGroup
// ---------------------------------------------------------------------------

/// A named group of tools that can be activated or deactivated as a unit.
#[derive(Debug, Clone)]
pub struct ToolGroup {
    /// Group identifier (e.g. "basic", "file_io", "web").
    pub name: String,
    /// Names of tools belonging to this group.
    pub tool_names: Vec<String>,
    /// Whether this group is currently active.
    pub active: bool,
}

// ---------------------------------------------------------------------------
// RegisteredTool
// ---------------------------------------------------------------------------

/// A tool stored inside the `Toolkit`, together with its group and preset args.
pub struct RegisteredTool {
    /// The concrete handler.
    pub handler: Box<dyn ToolHandler>,
    /// The group this tool belongs to.
    pub group: String,
    /// Arguments merged into every call before middleware runs.
    ///
    /// Preset args have lower priority than caller-supplied args — the
    /// caller's values win on key conflicts.
    pub preset_args: Value,
}

// ---------------------------------------------------------------------------
// Toolkit
// ---------------------------------------------------------------------------

/// Central registry for tools, groups, and middleware.
///
/// # Group semantics
///
/// Every tool belongs to exactly one group. The built-in `"basic"` group is
/// **always active** and cannot be deactivated. Other groups start active by
/// default but can be toggled with [`set_group_active`].
///
/// # Execution order
///
/// `call_tool` runs:
/// 1. Merge `preset_args` into caller args
/// 2. All `middleware.before()` in insertion order
/// 3. `handler.execute(args)`
/// 4. All `middleware.after()` in insertion order
/// 5. Return `ToolResponse`
pub struct Toolkit {
    tools: HashMap<String, RegisteredTool>,
    groups: HashMap<String, ToolGroup>,
    middlewares: Vec<Box<dyn ToolMiddleware>>,
    resources: Vec<Box<dyn ToolkitResource>>,
    /// Optional provider-neutral event channel for streaming tool traces.
    event_tx: Option<tokio::sync::mpsc::UnboundedSender<ToolTraceEvent>>,
}

impl Toolkit {
    /// Create an empty toolkit.  The `"basic"` group is pre-registered and
    /// always active.
    pub fn new() -> Self {
        let mut groups = HashMap::new();
        groups.insert(
            "basic".to_string(),
            ToolGroup {
                name: "basic".to_string(),
                tool_names: Vec::new(),
                active: true,
            },
        );
        Self {
            tools: HashMap::new(),
            groups,
            middlewares: Vec::new(),
            resources: Vec::new(),
            event_tx: None,
        }
    }

    // -----------------------------------------------------------------------
    // Registration
    // -----------------------------------------------------------------------

    /// Register a tool into the given group (defaults to `"basic"`).
    ///
    /// If the group does not yet exist it is created and marked active.
    /// If a tool with the same name already exists it is replaced.
    pub fn register(&mut self, handler: Box<dyn ToolHandler>, group: Option<&str>) {
        let group_name = group.unwrap_or("basic").to_string();
        let tool_name = handler.name().to_string();

        // Ensure the group exists.
        self.groups
            .entry(group_name.clone())
            .or_insert_with(|| ToolGroup {
                name: group_name.clone(),
                tool_names: Vec::new(),
                active: true,
            })
            .tool_names
            .push(tool_name.clone());

        self.tools.insert(
            tool_name,
            RegisteredTool {
                handler,
                group: group_name,
                preset_args: Value::Object(Default::default()),
            },
        );
    }

    /// Remove a tool by name.  Also removes it from its group's `tool_names`.
    pub fn unregister(&mut self, name: &str) {
        if let Some(tool) = self.tools.remove(name) {
            if let Some(group) = self.groups.get_mut(&tool.group) {
                group.tool_names.retain(|n| n != name);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Lookup
    // -----------------------------------------------------------------------

    /// Look up a registered tool by name.
    pub fn get_tool(&self, name: &str) -> Option<&RegisteredTool> {
        self.tools.get(name)
    }

    /// Total number of registered tools (active or not).
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    // -----------------------------------------------------------------------
    // Middleware
    // -----------------------------------------------------------------------

    /// Append a middleware to the chain.
    pub fn add_middleware(&mut self, middleware: Box<dyn ToolMiddleware>) {
        self.middlewares.push(middleware);
    }

    /// Set the event channel for streaming tool execution.
    pub fn set_event_tx(&mut self, tx: tokio::sync::mpsc::UnboundedSender<ToolTraceEvent>) {
        self.event_tx = Some(tx);
    }

    /// Track an external resource for cleanup when this toolkit is dropped.
    pub fn add_resource(&mut self, resource: Box<dyn ToolkitResource>) {
        self.resources.push(resource);
    }

    // -----------------------------------------------------------------------
    // Group management
    // -----------------------------------------------------------------------

    /// Register a group definition.  Existing group entries are preserved.
    pub fn register_group(&mut self, group: ToolGroup) {
        self.groups.insert(group.name.clone(), group);
    }

    /// Activate or deactivate a group.
    ///
    /// The `"basic"` group **cannot** be deactivated; calls targeting it are
    /// silently ignored.
    pub fn set_group_active(&mut self, group_name: &str, active: bool) {
        if group_name == "basic" {
            return;
        }
        if let Some(group) = self.groups.get_mut(group_name) {
            group.active = active;
        }
    }

    // -----------------------------------------------------------------------
    // Execution
    // -----------------------------------------------------------------------

    /// Invoke a tool by name.
    ///
    /// Returns `ToolError::NotFound` if the tool does not exist.
    /// Returns `ToolError::PermissionDenied` if the tool's group is inactive.
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<ToolResponse, ToolError> {
        // 1. Look up the tool.
        let registered = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        // 2. Check group active state ("basic" is always active).
        let group_active = if registered.group == "basic" {
            true
        } else {
            self.groups
                .get(&registered.group)
                .map(|g| g.active)
                .unwrap_or(false)
        };
        if !group_active {
            return Err(ToolError::PermissionDenied(format!(
                "tool '{}' belongs to inactive group '{}'",
                name, registered.group
            )));
        }

        // 3. Merge preset_args (lower priority) with caller args (higher priority).
        let mut merged = match registered.preset_args.clone() {
            Value::Object(map) => map,
            _ => Default::default(),
        };
        if let Value::Object(caller_map) = args {
            for (k, v) in caller_map {
                merged.insert(k, v);
            }
        }
        let mut effective_args = Value::Object(merged);

        // 4. Run before-middleware.
        for mw in &self.middlewares {
            mw.before(name, &mut effective_args).await?;
        }

        // 5. Execute handler. The default streaming implementation delegates
        // to `execute`; service-backed adapters may bridge concrete trace
        // events into the provider-neutral `ToolTraceEvent` channel.
        let mut response = {
            registered
                .handler
                .execute_streaming(effective_args, self.event_tx.clone())
                .await?
        };

        // 6. Run after-middleware.
        for mw in &self.middlewares {
            mw.after(name, &mut response).await?;
        }

        Ok(response)
    }

    // -----------------------------------------------------------------------
    // LLM definitions
    // -----------------------------------------------------------------------

    /// Return tool definitions for all active tools in all active groups.
    ///
    /// Format: `[{"name": "...", "description": "...", "parameters": {schema}}]`
    pub fn get_definitions(&self) -> Vec<Value> {
        self.tools
            .values()
            .filter(|t| {
                if t.group == "basic" {
                    true
                } else {
                    self.groups.get(&t.group).map(|g| g.active).unwrap_or(false)
                }
            })
            .map(|t| {
                serde_json::json!({
                    "name": t.handler.name(),
                    "description": t.handler.description(),
                    "parameters": t.handler.schema(),
                })
            })
            .collect()
    }
}

impl Default for Toolkit {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Toolkit {
    fn drop(&mut self) {
        for resource in self.resources.drain(..) {
            resource.close();
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "tool_tests.rs"]
mod tool_tests;

#[cfg(test)]
#[path = "tool_schema_tests.rs"]
mod tool_schema_tests;
