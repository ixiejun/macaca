//! Shared mock tools and middleware for toolkit contract tests.
//!
//! Fixtures are application-agnostic: they exercise registry, middleware, and group
//! policy semantics without referencing product-specific tool names or workflows.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;

use crate::message::ContentBlock;
use crate::tool::{ToolError, ToolHandler, ToolMiddleware, ToolResponse};

/// EchoTool — returns its input arguments as JSON text.
pub(crate) struct EchoTool;

#[async_trait]
impl ToolHandler for EchoTool {
    async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError> {
        Ok(ToolResponse::json(args))
    }
    fn name(&self) -> &str {
        "echo"
    }
    fn description(&self) -> &str {
        "Echoes its input arguments as JSON."
    }
    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            }
        })
    }
}

/// AddTool — expects `{"a": number, "b": number}`, returns their sum as text.
pub(crate) struct AddTool;

#[async_trait]
impl ToolHandler for AddTool {
    async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError> {
        let a = args["a"]
            .as_f64()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'a'".into()))?;
        let b = args["b"]
            .as_f64()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'b'".into()))?;
        Ok(ToolResponse::text(format!("{}", a + b)))
    }
    fn name(&self) -> &str {
        "add"
    }
    fn description(&self) -> &str {
        "Adds two numbers."
    }
    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "a": { "type": "number" },
                "b": { "type": "number" }
            },
            "required": ["a", "b"]
        })
    }
}

/// Records tool names seen in before/after middleware hooks.
pub(crate) struct RecordingMiddleware {
    pub(crate) before_calls: Arc<Mutex<Vec<String>>>,
    pub(crate) after_calls: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ToolMiddleware for RecordingMiddleware {
    async fn before(&self, name: &str, _args: &mut Value) -> Result<(), ToolError> {
        self.before_calls.lock().unwrap().push(name.to_string());
        Ok(())
    }
    async fn after(&self, name: &str, _response: &mut ToolResponse) -> Result<(), ToolError> {
        self.after_calls.lock().unwrap().push(name.to_string());
        Ok(())
    }
}

/// A tool whose name is configurable, so tests can register duplicates.
pub(crate) struct NamedEchoTool {
    pub(crate) tool_name: String,
}

#[async_trait]
impl ToolHandler for NamedEchoTool {
    async fn execute(&self, args: Value) -> Result<ToolResponse, ToolError> {
        Ok(ToolResponse::json(args))
    }
    fn name(&self) -> &str {
        &self.tool_name
    }
    fn description(&self) -> &str {
        "A named echo tool."
    }
    fn schema(&self) -> Value {
        serde_json::json!({"type": "object"})
    }
}

/// Middleware that always fails in `before`, used to verify short-circuit semantics.
pub(crate) struct FailBeforeMiddleware;

#[async_trait]
impl ToolMiddleware for FailBeforeMiddleware {
    async fn before(&self, _name: &str, _args: &mut Value) -> Result<(), ToolError> {
        Err(ToolError::ExecutionFailed("before blocked".into()))
    }
    async fn after(&self, _name: &str, _response: &mut ToolResponse) -> Result<(), ToolError> {
        Ok(())
    }
}

/// Middleware that replaces response text in `after`.
pub(crate) struct ModifyAfterMiddleware;

#[async_trait]
impl ToolMiddleware for ModifyAfterMiddleware {
    async fn before(&self, _name: &str, _args: &mut Value) -> Result<(), ToolError> {
        Ok(())
    }
    async fn after(&self, _name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        response.content = vec![ContentBlock::Text(crate::message::TextBlock {
            text: "modified_by_after".into(),
        })];
        Ok(())
    }
}

/// Labeled middleware that records its label into shared vecs for ordering assertions.
pub(crate) struct LabeledMiddleware {
    pub(crate) label: String,
    pub(crate) before_log: Arc<Mutex<Vec<String>>>,
    pub(crate) after_log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ToolMiddleware for LabeledMiddleware {
    async fn before(&self, _name: &str, _args: &mut Value) -> Result<(), ToolError> {
        self.before_log.lock().unwrap().push(self.label.clone());
        Ok(())
    }
    async fn after(&self, _name: &str, _response: &mut ToolResponse) -> Result<(), ToolError> {
        self.after_log.lock().unwrap().push(self.label.clone());
        Ok(())
    }
}

/// A tool that tracks whether execute was called.
pub(crate) struct TrackingTool {
    pub(crate) called: Arc<Mutex<bool>>,
}

#[async_trait]
impl ToolHandler for TrackingTool {
    async fn execute(&self, _args: Value) -> Result<ToolResponse, ToolError> {
        *self.called.lock().unwrap() = true;
        Ok(ToolResponse::text("ok"))
    }
    fn name(&self) -> &str {
        "tracking"
    }
    fn description(&self) -> &str {
        "Tracks calls."
    }
    fn schema(&self) -> Value {
        serde_json::json!({"type": "object"})
    }
}
