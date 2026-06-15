//! Tool, command, schema, middleware, and toolset abstractions.

use async_trait::async_trait;
use macaca_proto::{MacacaResult, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

/// A trace event from tool/driver execution.
///
/// Generic structure supporting any driver or service-backed tool provider.
/// Optional fields let providers add trace dimensions without breaking serde
/// decoding for records that do not carry those dimensions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Event semantic type (driver-defined, e.g. "thinking", "tool_call", "compilation", "file_read")
    #[serde(rename = "type")]
    pub event_type: String,

    /// Source driver identifier (auto-injected by framework, drivers don't need to set this)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_id: Option<String>,

    /// Event timestamp in milliseconds (auto-injected by framework)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,

    /// Correlation ID (shared by multiple events in the same logical operation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Title/summary (main heading for frontend rendering)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Body content (thinking process, output text, logs, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Tool/operation name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,

    /// Tool/operation input (structured JSON)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<Value>,

    /// Tool/operation output (text)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<String>,

    /// Whether this is an error event
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,

    /// Extension data (driver-specific non-standard fields)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Execution context carried alongside a tool command.
#[derive(Clone, Default)]
pub struct ToolCommandContext {
    /// Optional trace event sink for standard tool events.
    pub event_tx: Option<UnboundedSender<TraceEvent>>,
    /// Optional correlation id shared across related events.
    pub correlation_id: Option<String>,
    /// Optional extension metadata for middleware.
    pub metadata: Option<Value>,
}

impl ToolCommandContext {
    pub fn with_event_tx(mut self, event_tx: UnboundedSender<TraceEvent>) -> Self {
        self.event_tx = Some(event_tx);
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Canonical tool execution request.
#[derive(Clone)]
pub struct ToolCommand {
    pub input: Value,
    pub context: ToolCommandContext,
}

impl ToolCommand {
    pub fn new(input: Value) -> Self {
        Self {
            input,
            context: ToolCommandContext::default(),
        }
    }

    pub fn with_context(input: Value, context: ToolCommandContext) -> Self {
        Self { input, context }
    }
}

/// Provides canonical schema access for tools.
pub trait ToolSchemaProvider: Send + Sync {
    fn tool_schema(&self) -> Value;
}

/// Canonical command execution interface for tools.
#[async_trait]
pub trait ToolCommandExecutor: Send + Sync {
    async fn execute_command(&self, command: ToolCommand) -> MacacaResult<Value>;
}

/// Middleware around canonical tool command execution.
#[async_trait]
pub trait ToolCommandMiddleware: Send + Sync {
    async fn before(&self, tool_name: &str, command: &ToolCommand) -> MacacaResult<()>;

    async fn after(
        &self,
        tool_name: &str,
        command: &ToolCommand,
        result: &MacacaResult<Value>,
    ) -> MacacaResult<()>;
}

/// Canonical tool command execution chain.
pub struct ToolCommandPipeline {
    middlewares: Vec<Box<dyn ToolCommandMiddleware>>,
}

impl ToolCommandPipeline {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn with_default_trace() -> Self {
        let mut pipeline = Self::new();
        pipeline.add_middleware(Box::new(TraceToolCommandMiddleware));
        pipeline
    }

    pub fn add_middleware(&mut self, middleware: Box<dyn ToolCommandMiddleware>) {
        self.middlewares.push(middleware);
    }

    pub async fn execute<T>(&self, tool: &T, command: ToolCommand) -> MacacaResult<Value>
    where
        T: Tool + ?Sized,
    {
        for middleware in &self.middlewares {
            middleware.before(tool.name(), &command).await?;
        }

        let result = tool.invoke(command.clone()).await;

        for middleware in &self.middlewares {
            middleware.after(tool.name(), &command, &result).await?;
        }

        result
    }
}

impl Default for ToolCommandPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard trace middleware that emits tool_call/tool_result events.
pub struct TraceToolCommandMiddleware;

#[async_trait]
impl ToolCommandMiddleware for TraceToolCommandMiddleware {
    async fn before(&self, tool_name: &str, command: &ToolCommand) -> MacacaResult<()> {
        if let Some(event_tx) = &command.context.event_tx {
            let _ = event_tx.send(TraceEvent {
                event_type: "tool_call".into(),
                correlation_id: command.context.correlation_id.clone(),
                tool_name: Some(tool_name.to_string()),
                tool_input: Some(command.input.clone()),
                metadata: command.context.metadata.clone(),
                ..Default::default()
            });
        }
        Ok(())
    }

    async fn after(
        &self,
        tool_name: &str,
        command: &ToolCommand,
        result: &MacacaResult<Value>,
    ) -> MacacaResult<()> {
        if let Some(event_tx) = &command.context.event_tx {
            let (tool_output, is_error) = match result {
                Ok(value) => (Some(serialize_tool_output(value)), Some(false)),
                Err(error) => (Some(error.to_string()), Some(true)),
            };
            let _ = event_tx.send(TraceEvent {
                event_type: "tool_result".into(),
                correlation_id: command.context.correlation_id.clone(),
                tool_name: Some(tool_name.to_string()),
                tool_output,
                is_error,
                metadata: command.context.metadata.clone(),
                ..Default::default()
            });
        }
        Ok(())
    }
}

fn serialize_tool_output(output: &Value) -> String {
    serde_json::to_string(output).unwrap_or_else(|_| output.to_string())
}

/// A single callable tool.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;

    /// JSON Schema describing the tool input parameters.
    fn tool_schema(&self) -> Value;

    /// Execute the tool once after the command pipeline has emitted trace and
    /// middleware hooks. Implementations read business input from
    /// [`ToolCommand::input`] and may use [`ToolCommand::context`] for
    /// provider-neutral metadata or event sinks.
    async fn invoke(&self, command: ToolCommand) -> MacacaResult<Value>;
}

impl<T> ToolSchemaProvider for T
where
    T: Tool + ?Sized,
{
    fn tool_schema(&self) -> Value {
        Tool::tool_schema(self)
    }
}

#[async_trait]
impl<T> ToolCommandExecutor for T
where
    T: Tool + ?Sized,
{
    async fn execute_command(&self, command: ToolCommand) -> MacacaResult<Value> {
        ToolCommandPipeline::with_default_trace()
            .execute(self, command)
            .await
    }
}

/// Canonical tool catalog query surface.
pub trait ToolCatalog: Send + Sync {
    fn all_tools(&self) -> &[Box<dyn Tool>];

    fn find_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.all_tools()
            .iter()
            .find(|tool| tool.name() == name)
            .map(|tool| tool.as_ref())
    }

    fn definitions(&self) -> Vec<ToolDefinition> {
        self.all_tools()
            .iter()
            .map(|tool| ToolDefinition {
                name: tool.name().to_owned(),
                description: tool.description().to_owned(),
                parameters: tool.tool_schema(),
            })
            .collect()
    }
}

/// Standard composite toolset primitive for combining multiple tool groups.
pub struct CompositeToolSet {
    tools: Vec<Box<dyn Tool>>,
}

impl CompositeToolSet {
    pub fn new(tools: Vec<Box<dyn Tool>>) -> Self {
        Self { tools }
    }

    pub fn empty() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn from_groups(groups: Vec<Vec<Box<dyn Tool>>>) -> Self {
        let mut tools = Vec::new();
        for group in groups {
            tools.extend(group);
        }
        Self { tools }
    }

    pub fn push_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn extend_tools(&mut self, tools: Vec<Box<dyn Tool>>) {
        self.tools.extend(tools);
    }
}

impl Default for CompositeToolSet {
    fn default() -> Self {
        Self::empty()
    }
}

impl ToolCatalog for CompositeToolSet {
    fn all_tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::MacacaResult;
    use serde_json::json;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echo the payload"
        }

        fn tool_schema(&self) -> Value {
            json!({"type": "object"})
        }

        async fn invoke(&self, command: ToolCommand) -> MacacaResult<Value> {
            Ok(command.input)
        }
    }

    #[tokio::test]
    async fn command_executor_emits_standard_trace_events() {
        let tool = EchoTool;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let command = ToolCommand::with_context(
            json!({"message": "hello"}),
            ToolCommandContext::default()
                .with_event_tx(tx)
                .with_correlation_id("corr-1"),
        );

        let output = tool.execute_command(command).await.expect("execute");
        assert_eq!(output["message"], "hello");

        let first = rx.recv().await.expect("tool_call event");
        assert_eq!(first.event_type, "tool_call");
        assert_eq!(first.tool_name.as_deref(), Some("echo"));

        let second = rx.recv().await.expect("tool_result event");
        assert_eq!(second.event_type, "tool_result");
        assert_eq!(second.tool_name.as_deref(), Some("echo"));
        assert_eq!(second.is_error, Some(false));
    }

    #[test]
    fn composite_toolset_flattens_groups() {
        let composite = CompositeToolSet::from_groups(vec![
            vec![Box::new(EchoTool) as Box<dyn Tool>],
            vec![Box::new(EchoTool) as Box<dyn Tool>],
        ]);

        assert_eq!(composite.all_tools().len(), 2);
        assert_eq!(composite.definitions().len(), 2);
    }
}
