//! Adapters bridging macaca-framework with existing macaca-llm and macaca-tools crates.
//!
//! These adapters let `ReActAgent` use the existing LLM providers and tool
//! implementations without rewriting them.

use std::sync::Arc;

#[cfg(feature = "service-clients")]
pub use crate::adapter_llm::ServiceChatModelAdapter;
#[allow(deprecated)]
pub use crate::adapter_llm::{LlmProviderAdapter, RoutedLlmAdapter};
use async_trait::async_trait;

use crate::tool::{ToolError, ToolHandler, ToolResponse, Toolkit};

// LegacyToolHandler: macaca_tools::Tool → ToolHandler
// ---------------------------------------------------------------------------

/// Wraps a single tool from `macaca_tools::ToolCatalog` as a framework `ToolHandler`.
pub struct LegacyToolHandler {
    tool_name: String,
    tool_description: String,
    tool_schema: serde_json::Value,
    tool_set: Arc<dyn macaca_tools::ToolCatalog>,
}

impl LegacyToolHandler {
    pub fn new(
        name: String,
        description: String,
        schema: serde_json::Value,
        tool_set: Arc<dyn macaca_tools::ToolCatalog>,
    ) -> Self {
        Self {
            tool_name: name,
            tool_description: description,
            tool_schema: schema,
            tool_set,
        }
    }
}

#[async_trait]
impl ToolHandler for LegacyToolHandler {
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResponse, ToolError> {
        let tool = macaca_tools::ToolCatalog::find_tool(self.tool_set.as_ref(), &self.tool_name)
            .ok_or_else(|| ToolError::NotFound(self.tool_name.clone()))?;

        let result = macaca_tools::ToolCommandExecutor::execute_command(
            tool,
            macaca_tools::ToolCommand::new(args),
        )
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let text = serde_json::to_string(&result).unwrap_or_else(|_| result.to_string());
        Ok(ToolResponse::text(text))
    }

    fn name(&self) -> &str {
        &self.tool_name
    }
    fn description(&self) -> &str {
        &self.tool_description
    }
    fn schema(&self) -> serde_json::Value {
        self.tool_schema.clone()
    }
}

// ---------------------------------------------------------------------------
// ToolSetBridge: macaca_tools::ToolCatalog → Toolkit
// ---------------------------------------------------------------------------

/// Converts a `macaca_tools::ToolCatalog` into a framework `Toolkit`.
pub struct ToolSetBridge;

impl ToolSetBridge {
    /// Register all tools from a `macaca_tools::ToolCatalog` into a new `Toolkit`.
    pub fn from_tool_set(tool_set: Arc<dyn macaca_tools::ToolCatalog>) -> Toolkit {
        let mut toolkit = Toolkit::new();
        let definitions = macaca_tools::ToolCatalog::definitions(tool_set.as_ref());
        for def in definitions {
            let handler = LegacyToolHandler::new(
                def.name.clone(),
                def.description.clone(),
                def.parameters.clone(),
                Arc::clone(&tool_set),
            );
            toolkit.register(Box::new(handler), None);
        }
        toolkit
    }
}

// ---------------------------------------------------------------------------
// SingleToolAdapter: individual macaca_tools::Tool → ToolHandler
// ---------------------------------------------------------------------------

/// Wraps a single `macaca_tools::Tool` as a framework `ToolHandler`.
///
/// Unlike `LegacyToolHandler` (which requires a `ToolSet`), this adapter
/// owns the tool directly — useful for per-agent tools that are not part
/// of a shared `ToolSet` (e.g. todo/task board tools).
pub struct SingleToolAdapter {
    tool: Box<dyn macaca_tools::Tool>,
}

impl SingleToolAdapter {
    pub fn new(tool: Box<dyn macaca_tools::Tool>) -> Self {
        Self { tool }
    }
}

#[async_trait]
impl ToolHandler for SingleToolAdapter {
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResponse, ToolError> {
        let result = macaca_tools::ToolCommandExecutor::execute_command(
            self.tool.as_ref(),
            macaca_tools::ToolCommand::new(args),
        )
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let text = serde_json::to_string(&result).unwrap_or_else(|_| result.to_string());
        Ok(ToolResponse::text(text))
    }

    #[cfg(feature = "macaca-compat")]
    async fn execute_streaming(
        &self,
        args: serde_json::Value,
        event_tx: Option<tokio::sync::mpsc::UnboundedSender<macaca_tools::TraceEvent>>,
    ) -> Result<ToolResponse, ToolError> {
        let result = macaca_tools::ToolCommandExecutor::execute_command(
            self.tool.as_ref(),
            macaca_tools::ToolCommand::with_context(
                args,
                macaca_tools::ToolCommandContext {
                    event_tx,
                    ..Default::default()
                },
            ),
        )
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let text = serde_json::to_string(&result).unwrap_or_else(|_| result.to_string());
        Ok(ToolResponse::text(text))
    }

    fn name(&self) -> &str {
        self.tool.name()
    }

    fn description(&self) -> &str {
        self.tool.description()
    }

    fn schema(&self) -> serde_json::Value {
        macaca_tools::ToolSchemaProvider::tool_schema(self.tool.as_ref())
    }
}

// ---------------------------------------------------------------------------
// Tool bridge tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_messages_from_json_user() {
        let msgs = vec![serde_json::json!({"role": "user", "content": "hello"})];
        let result = crate::llm_wire::messages_from_json_values(&msgs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, macaca_proto::LlmRole::User);
        assert_eq!(result[0].content, "hello");
    }

    #[test]
    fn test_messages_from_json_system() {
        let msgs = vec![serde_json::json!({"role": "system", "content": "You are helpful."})];
        let result = crate::llm_wire::messages_from_json_values(&msgs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, macaca_proto::LlmRole::System);
    }

    #[test]
    fn test_messages_from_json_assistant_with_tools() {
        let msgs = vec![serde_json::json!({
            "role": "assistant",
            "content": "",
            "tool_calls": [{
                "id": "call_1",
                "function": {
                    "name": "search",
                    "arguments": "{\"query\": \"rust\"}"
                }
            }]
        })];
        let result = crate::llm_wire::messages_from_json_values(&msgs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, macaca_proto::LlmRole::Assistant);
        let tcs = result[0].tool_calls.as_ref().unwrap();
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0].name, "search");
    }

    #[test]
    fn test_messages_from_json_tool_result() {
        let msgs = vec![serde_json::json!({
            "role": "tool",
            "tool_call_id": "call_1",
            "content": "found 5 results"
        })];
        let result = crate::llm_wire::messages_from_json_values(&msgs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, macaca_proto::LlmRole::Tool);
    }

    #[test]
    fn test_messages_from_json_mixed() {
        let msgs = vec![
            serde_json::json!({"role": "system", "content": "sys"}),
            serde_json::json!({"role": "user", "content": "hi"}),
            serde_json::json!({"role": "assistant", "content": "hello"}),
            serde_json::json!({"role": "invalid", "content": "skip"}),
        ];
        let result = crate::llm_wire::messages_from_json_values(&msgs);
        assert_eq!(result.len(), 3); // invalid role skipped
    }
}
