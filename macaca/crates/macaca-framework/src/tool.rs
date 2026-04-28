//! Tool system — registration, middleware, and execution.
//!
//! Provides a toolkit abstraction for managing agent tools:
//! - `ToolHandler` trait for implementing tools
//! - `ToolMiddleware` for cross-cutting concerns (logging, rate limiting, etc.)
//! - `ToolGroup` for activating/deactivating sets of tools
//! - `Toolkit` as the central registry that wires everything together

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::message::{ContentBlock, TextBlock};

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

    /// Execute with streaming events support.
    /// Default: delegates to execute()
    #[cfg(feature = "macaca-compat")]
    async fn execute_streaming(
        &self,
        args: Value,
        event_tx: Option<tokio::sync::mpsc::UnboundedSender<macaca_tools::TraceEvent>>,
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
    /// Optional event channel for streaming tool execution
    #[cfg(feature = "macaca-compat")]
    event_tx: Option<tokio::sync::mpsc::UnboundedSender<macaca_tools::TraceEvent>>,
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
            #[cfg(feature = "macaca-compat")]
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
    #[cfg(feature = "macaca-compat")]
    pub fn set_event_tx(&mut self, tx: tokio::sync::mpsc::UnboundedSender<macaca_tools::TraceEvent>) {
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

        // 5. Execute handler (streaming when event_tx is available).
        #[cfg(feature = "macaca-compat")]
        let mut response = {
            registered.handler
                .execute_streaming(effective_args, self.event_tx.clone())
                .await?
        };
        #[cfg(not(feature = "macaca-compat"))]
        let mut response = registered.handler.execute(effective_args).await?;

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
// tool_schema! macro
// ---------------------------------------------------------------------------

/// Macro to conveniently construct a JSON Schema object for tool parameters.
///
/// # Example
/// ```rust
/// use macaca_framework::tool_schema;
/// let schema = tool_schema!({
///     "query" => { "type": "string", "description": "Search query" },
///     "limit" => { "type": "integer", "description": "Max results", "default": 5 }
/// });
/// ```
#[macro_export]
macro_rules! tool_schema {
    ({ $( $name:tt => $props:tt ),* $(,)? }) => {{
        let mut properties = serde_json::Map::new();
        let mut required = Vec::<String>::new();
        $(
            let prop_val: serde_json::Value = serde_json::json!($props);
            // If no "default" key exists, the field is required
            if !prop_val.as_object().map_or(false, |o| o.contains_key("default")) {
                required.push($name.to_string());
            }
            properties.insert($name.to_string(), prop_val);
        )*
        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }};
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    // -----------------------------------------------------------------------
    // Mock tools
    // -----------------------------------------------------------------------

    /// EchoTool — returns its input arguments as JSON text.
    struct EchoTool;

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

    /// AddTool — expects `{"a": number, "b": number}`, returns their sum.
    struct AddTool;

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

    // -----------------------------------------------------------------------
    // Middleware mock
    // -----------------------------------------------------------------------

    use std::sync::{Arc, Mutex};

    struct RecordingMiddleware {
        before_calls: Arc<Mutex<Vec<String>>>,
        after_calls: Arc<Mutex<Vec<String>>>,
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

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_register_and_call() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(AddTool), None);

        let resp = kit
            .call_tool("add", serde_json::json!({"a": 3.0, "b": 4.0}))
            .await
            .unwrap();

        assert_eq!(resp.content.len(), 1);
        if let ContentBlock::Text(tb) = &resp.content[0] {
            assert_eq!(tb.text, "7");
        } else {
            panic!("expected TextBlock");
        }
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let kit = Toolkit::new();
        let err = kit
            .call_tool("nonexistent", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_preset_args() {
        // Register EchoTool, then manually set preset_args.
        let mut kit = Toolkit::new();
        kit.register(Box::new(EchoTool), None);

        // Inject a preset arg.
        kit.tools.get_mut("echo").unwrap().preset_args =
            serde_json::json!({"preset_key": "preset_val"});

        // Caller provides an additional key; caller wins on conflict.
        let resp = kit
            .call_tool("echo", serde_json::json!({"caller_key": "caller_val"}))
            .await
            .unwrap();

        if let ContentBlock::Text(tb) = &resp.content[0] {
            let v: Value = serde_json::from_str(&tb.text).unwrap();
            assert_eq!(v["preset_key"], "preset_val");
            assert_eq!(v["caller_key"], "caller_val");
        } else {
            panic!("expected TextBlock");
        }
    }

    #[tokio::test]
    async fn test_middleware_chain() {
        let before_calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let after_calls = Arc::new(Mutex::new(Vec::<String>::new()));

        let mw = RecordingMiddleware {
            before_calls: before_calls.clone(),
            after_calls: after_calls.clone(),
        };

        let mut kit = Toolkit::new();
        kit.add_middleware(Box::new(mw));
        kit.register(Box::new(AddTool), None);

        kit.call_tool("add", serde_json::json!({"a": 1.0, "b": 2.0}))
            .await
            .unwrap();

        assert_eq!(*before_calls.lock().unwrap(), vec!["add"]);
        assert_eq!(*after_calls.lock().unwrap(), vec!["add"]);
    }

    #[tokio::test]
    async fn test_group_active() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(EchoTool), Some("optional"));

        // Should work while group is active.
        kit.call_tool("echo", serde_json::json!({})).await.unwrap();

        // Deactivate the group.
        kit.set_group_active("optional", false);

        let err = kit
            .call_tool("echo", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));
    }

    #[tokio::test]
    async fn test_get_definitions() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(EchoTool), Some("group_a"));
        kit.register(Box::new(AddTool), Some("group_b"));

        // Both groups active → both definitions present.
        let defs = kit.get_definitions();
        assert_eq!(defs.len(), 2);

        // Deactivate group_a → only AddTool definition.
        kit.set_group_active("group_a", false);
        let defs = kit.get_definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0]["name"], "add");
    }

    #[tokio::test]
    async fn test_unregister() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(AddTool), None);
        assert_eq!(kit.tool_count(), 1);

        kit.unregister("add");
        assert_eq!(kit.tool_count(), 0);

        let err = kit
            .call_tool("add", serde_json::json!({"a": 1.0, "b": 2.0}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_basic_group_always_active() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(AddTool), None); // defaults to "basic"

        // Attempting to deactivate "basic" is a no-op.
        kit.set_group_active("basic", false);

        // Tool must still be callable.
        kit.call_tool("add", serde_json::json!({"a": 10.0, "b": 5.0}))
            .await
            .unwrap();

        // Definition must still appear.
        let defs = kit.get_definitions();
        assert_eq!(defs.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Robustness tests
    // -----------------------------------------------------------------------

    /// A tool whose name is configurable, so we can register duplicates.
    struct NamedEchoTool {
        tool_name: String,
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

    /// Middleware that always fails in `before`.
    struct FailBeforeMiddleware;

    #[async_trait]
    impl ToolMiddleware for FailBeforeMiddleware {
        async fn before(&self, _name: &str, _args: &mut Value) -> Result<(), ToolError> {
            Err(ToolError::ExecutionFailed("before blocked".into()))
        }
        async fn after(&self, _name: &str, _response: &mut ToolResponse) -> Result<(), ToolError> {
            Ok(())
        }
    }

    /// Middleware that modifies the response text in `after`.
    struct ModifyAfterMiddleware;

    #[async_trait]
    impl ToolMiddleware for ModifyAfterMiddleware {
        async fn before(&self, _name: &str, _args: &mut Value) -> Result<(), ToolError> {
            Ok(())
        }
        async fn after(&self, _name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
            response.content = vec![ContentBlock::Text(TextBlock {
                text: "modified_by_after".into(),
            })];
            Ok(())
        }
    }

    /// A labeled middleware that records its label into shared vecs.
    struct LabeledMiddleware {
        label: String,
        before_log: Arc<Mutex<Vec<String>>>,
        after_log: Arc<Mutex<Vec<String>>>,
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
    struct TrackingTool {
        called: Arc<Mutex<bool>>,
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

    #[tokio::test]
    async fn test_register_duplicate_tool_name() {
        let mut kit = Toolkit::new();
        kit.register(
            Box::new(NamedEchoTool {
                tool_name: "dup".into(),
            }),
            None,
        );
        kit.register(
            Box::new(NamedEchoTool {
                tool_name: "dup".into(),
            }),
            None,
        );
        // Second registration replaces the first; count is still 1.
        assert_eq!(kit.tool_count(), 1);
        // Tool is still callable.
        kit.call_tool("dup", serde_json::json!({})).await.unwrap();
    }

    #[tokio::test]
    async fn test_call_nonexistent_tool() {
        let kit = Toolkit::new();
        let err = kit
            .call_tool("does_not_exist", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
        assert!(err.to_string().contains("does_not_exist"));
    }

    #[tokio::test]
    async fn test_middleware_before_error_short_circuits() {
        let called = Arc::new(Mutex::new(false));
        let mut kit = Toolkit::new();
        kit.add_middleware(Box::new(FailBeforeMiddleware));
        kit.register(
            Box::new(TrackingTool {
                called: called.clone(),
            }),
            None,
        );

        let err = kit
            .call_tool("tracking", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
        // Handler must NOT have been called.
        assert!(!*called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_middleware_after_modifies_response() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(AddTool), None);
        kit.add_middleware(Box::new(ModifyAfterMiddleware));

        let resp = kit
            .call_tool("add", serde_json::json!({"a": 1.0, "b": 2.0}))
            .await
            .unwrap();

        if let ContentBlock::Text(tb) = &resp.content[0] {
            assert_eq!(tb.text, "modified_by_after");
        } else {
            panic!("expected TextBlock");
        }
    }

    #[tokio::test]
    async fn test_multiple_middlewares_chain() {
        let before_log = Arc::new(Mutex::new(Vec::<String>::new()));
        let after_log = Arc::new(Mutex::new(Vec::<String>::new()));

        let mut kit = Toolkit::new();
        for label in ["mw1", "mw2", "mw3"] {
            kit.add_middleware(Box::new(LabeledMiddleware {
                label: label.to_string(),
                before_log: before_log.clone(),
                after_log: after_log.clone(),
            }));
        }
        kit.register(Box::new(EchoTool), None);

        kit.call_tool("echo", serde_json::json!({})).await.unwrap();

        assert_eq!(*before_log.lock().unwrap(), vec!["mw1", "mw2", "mw3"]);
        assert_eq!(*after_log.lock().unwrap(), vec!["mw1", "mw2", "mw3"]);
    }

    #[tokio::test]
    async fn test_disabled_group_tool_rejected() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(EchoTool), Some("optional"));

        // Deactivate the group.
        kit.set_group_active("optional", false);

        let err = kit
            .call_tool("echo", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));
        assert!(err.to_string().contains("inactive group"));
    }

    #[tokio::test]
    async fn test_basic_group_cannot_disable() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(AddTool), None); // "basic" group

        // Attempt to disable basic group.
        kit.set_group_active("basic", false);

        // "basic" group must remain active.
        let group = kit.groups.get("basic").unwrap();
        assert!(group.active);

        // Tool still callable.
        kit.call_tool("add", serde_json::json!({"a": 1.0, "b": 1.0}))
            .await
            .unwrap();

        // Definition still present.
        assert_eq!(kit.get_definitions().len(), 1);
    }

    #[tokio::test]
    async fn test_empty_toolkit_definitions() {
        let kit = Toolkit::new();
        let defs = kit.get_definitions();
        assert!(defs.is_empty());
    }

    #[tokio::test]
    async fn test_preset_kwargs_merge() {
        let mut kit = Toolkit::new();
        kit.register(Box::new(EchoTool), None);

        // Set preset args.
        kit.tools.get_mut("echo").unwrap().preset_args =
            serde_json::json!({"preset_a": "from_preset", "shared": "preset_wins_not"});

        // Caller provides "shared" and an extra key — caller wins on conflict.
        let resp = kit
            .call_tool(
                "echo",
                serde_json::json!({"shared": "caller_wins", "caller_b": 42}),
            )
            .await
            .unwrap();

        if let ContentBlock::Text(tb) = &resp.content[0] {
            let v: Value = serde_json::from_str(&tb.text).unwrap();
            assert_eq!(v["preset_a"], "from_preset");
            assert_eq!(v["shared"], "caller_wins");
            assert_eq!(v["caller_b"], 42);
        } else {
            panic!("expected TextBlock");
        }
    }

    // -----------------------------------------------------------------------
    // tool_schema! macro tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tool_schema_macro_basic() {
        let schema = tool_schema!({
            "query" => { "type": "string", "description": "Search query" },
            "count" => { "type": "integer", "description": "Number of results" }
        });

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["query"]["type"], "string");
        assert_eq!(schema["properties"]["count"]["type"], "integer");

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("query")));
        assert!(required.contains(&serde_json::json!("count")));
        assert_eq!(required.len(), 2);
    }

    #[test]
    fn test_tool_schema_macro_with_defaults() {
        let schema = tool_schema!({
            "query" => { "type": "string", "description": "Search query" },
            "limit" => { "type": "integer", "description": "Max results", "default": 5 }
        });

        let required = schema["required"].as_array().unwrap();
        // "query" has no default → required
        assert!(required.contains(&serde_json::json!("query")));
        // "limit" has a default → NOT required
        assert!(!required.contains(&serde_json::json!("limit")));
        assert_eq!(required.len(), 1);

        // Both properties should exist
        assert_eq!(schema["properties"]["limit"]["default"], 5);
    }

    #[test]
    fn test_tool_schema_macro_empty() {
        let schema = tool_schema!({});

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"], serde_json::json!({}));
        assert_eq!(schema["required"], serde_json::json!([]));
    }
}
