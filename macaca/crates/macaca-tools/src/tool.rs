//! Tool and ToolSet trait definitions.

use async_trait::async_trait;
use macaca_proto::{MacacaResult, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

/// A trace event from tool/driver execution.
///
/// Generic structure supporting any driver (Claude Code, custom drivers, etc.).
/// New fields are all `Option` + `skip_serializing_if` for backward compatibility.
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

    // --- Generic semantic fields ---

    /// Title/summary (main heading for frontend rendering)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Body content (thinking process, output text, logs, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "thinking", alias = "text")]
    pub content: Option<String>,

    /// Tool/operation name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,

    /// Tool/operation input (structured JSON)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<Value>,

    /// Tool/operation output (text)
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "tool_result")]
    pub tool_output: Option<String>,

    /// Whether this is an error event
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,

    /// Extension data (driver-specific non-standard fields)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// A single callable tool.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's input parameters.
    fn parameters_schema(&self) -> Value;

    /// Execute the tool and return the result.
    async fn execute(&self, input: Value) -> MacacaResult<Value>;

    /// Execute with streaming events via an unbounded sender.
    /// Default implementation just calls execute() without streaming.
    async fn execute_streaming(
        &self,
        input: Value,
        event_tx: Option<UnboundedSender<TraceEvent>>,
    ) -> MacacaResult<Value> {
        let _ = event_tx;
        self.execute(input).await
    }
}

/// A collection of tools, addressable by name.
pub trait ToolSet: Send + Sync {
    fn tools(&self) -> &[Box<dyn Tool>];

    fn get_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools()
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    /// Convert all tools to LLM-compatible tool definitions.
    fn to_definitions(&self) -> Vec<ToolDefinition> {
        self.tools()
            .iter()
            .map(|t| ToolDefinition {
                name: t.name().to_owned(),
                description: t.description().to_owned(),
                parameters: t.parameters_schema(),
            })
            .collect()
    }
}
