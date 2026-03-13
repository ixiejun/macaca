//! Tool and ToolSet trait definitions.

use async_trait::async_trait;
use macaca_proto::{MacacaResult, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

/// A trace event from tool execution (e.g., Claude Code internal steps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub tool_name: Option<String>,
    pub tool_input: Option<Value>,
    pub tool_result: Option<String>,
    pub thinking: Option<String>,
    pub text: Option<String>,
    pub is_error: Option<bool>,
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
        self.tools().iter().find(|t| t.name() == name).map(|t| t.as_ref())
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
