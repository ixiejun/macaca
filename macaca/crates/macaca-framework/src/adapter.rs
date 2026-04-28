//! Adapters bridging macaca-framework with existing macaca-llm and macaca-tools crates.
//!
//! These adapters let `ReActAgent` use the existing LLM providers and tool
//! implementations without rewriting them.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_llm::LlmProvider;

use crate::message::{ContentBlock, TextBlock, ThinkingBlock, ToolUseBlock};
use crate::model::{ChatModel, ChatOptions, ChatResponse, ChatUsage, ModelError};
use crate::tool::{ToolError, ToolHandler, ToolResponse, Toolkit};

// ---------------------------------------------------------------------------
// LlmProviderAdapter: macaca_llm::LlmProvider → ChatModel
// ---------------------------------------------------------------------------

/// Bridges `macaca_llm::LlmProvider` to the framework's `ChatModel` trait.
pub struct LlmProviderAdapter {
    provider: Arc<dyn macaca_llm::LlmProvider>,
}

impl LlmProviderAdapter {
    pub fn new(provider: Arc<dyn macaca_llm::LlmProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl ChatModel for LlmProviderAdapter {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        let llm_messages = messages_from_json(&messages);

        let mut llm_options = macaca_proto::LlmOptions {
            model: options.model.clone().unwrap_or_default(),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            ..Default::default()
        };

        // Convert tool definitions
        if let Some(ref tools) = options.tools {
            llm_options.tools = Some(
                tools
                    .iter()
                    .map(|t| macaca_proto::ToolDefinition {
                        name: t["name"].as_str().unwrap_or("").to_string(),
                        description: t["description"].as_str().unwrap_or("").to_string(),
                        parameters: t["parameters"].clone(),
                    })
                    .collect(),
            );
        }

        let response = self
            .provider
            .chat(llm_messages, &llm_options)
            .await
            .map_err(|e| ModelError::Api(e.to_string()))?;

        // Build content blocks from response
        let mut content_blocks = Vec::new();

        if let Some(ref reasoning_content) = response.reasoning_content {
            if !reasoning_content.is_empty() {
                content_blocks.push(ContentBlock::Thinking(ThinkingBlock {
                    thinking: reasoning_content.clone(),
                }));
            }
        }

        if !response.content.is_empty() {
            content_blocks.push(ContentBlock::Text(TextBlock {
                text: response.content.clone(),
            }));
        }

        if let Some(ref tool_calls) = response.tool_calls {
            for tc in tool_calls {
                content_blocks.push(ContentBlock::ToolUse(ToolUseBlock {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: tc.arguments.clone(),
                    raw_input: None,
                }));
            }
        }

        Ok(ChatResponse {
            content: content_blocks,
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            usage: ChatUsage {
                input_tokens: response.usage.prompt_tokens,
                output_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
                duration_ms: None,
            },
            metadata: None,
        })
    }

    fn name(&self) -> &str {
        self.provider.name()
    }
}

/// Bridges `macaca_llm::LlmRouter` to the framework's `ChatModel` trait with
/// a pre-resolved default route plan.
pub struct RoutedLlmAdapter {
    router: Arc<macaca_llm::LlmRouter>,
    default_selection: macaca_llm::ModelSelection,
}

impl RoutedLlmAdapter {
    pub fn new(
        router: Arc<macaca_llm::LlmRouter>,
        default_selection: macaca_llm::ModelSelection,
    ) -> Self {
        Self {
            router,
            default_selection,
        }
    }
}

#[async_trait]
impl ChatModel for RoutedLlmAdapter {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        let llm_messages = messages_from_json(&messages);

        let mut llm_options = macaca_proto::LlmOptions {
            model: options
                .model
                .clone()
                .unwrap_or_else(|| self.default_selection.primary.reference()),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            ..Default::default()
        };

        if let Some(ref tools) = options.tools {
            llm_options.tools = Some(
                tools
                    .iter()
                    .map(|t| macaca_proto::ToolDefinition {
                        name: t["name"].as_str().unwrap_or("").to_string(),
                        description: t["description"].as_str().unwrap_or("").to_string(),
                        parameters: t["parameters"].clone(),
                    })
                    .collect(),
            );
        }

        let response = if options
            .model
            .as_deref()
            .is_some_and(|m| !m.is_empty() && m != self.default_selection.primary.reference())
        {
            self.router
                .chat(llm_messages, &llm_options)
                .await
                .map_err(|e| ModelError::Api(e.to_string()))?
        } else {
            llm_options.model = self.default_selection.primary.model.clone();
            self.router
                .chat_with_selection(llm_messages, &llm_options, &self.default_selection)
                .await
                .map_err(|e| ModelError::Api(e.to_string()))?
        };

        let mut content_blocks = Vec::new();

        if let Some(ref reasoning_content) = response.reasoning_content {
            if !reasoning_content.is_empty() {
                content_blocks.push(ContentBlock::Thinking(ThinkingBlock {
                    thinking: reasoning_content.clone(),
                }));
            }
        }

        if !response.content.is_empty() {
            content_blocks.push(ContentBlock::Text(TextBlock {
                text: response.content.clone(),
            }));
        }

        if let Some(ref tool_calls) = response.tool_calls {
            for tc in tool_calls {
                content_blocks.push(ContentBlock::ToolUse(ToolUseBlock {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: tc.arguments.clone(),
                    raw_input: None,
                }));
            }
        }

        Ok(ChatResponse {
            content: content_blocks,
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            usage: ChatUsage {
                input_tokens: response.usage.prompt_tokens,
                output_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
                duration_ms: None,
            },
            metadata: None,
        })
    }

    fn name(&self) -> &str {
        self.router.name()
    }
}

/// Convert JSON messages (as produced by Formatter) back to LlmMessage.
fn messages_from_json(messages: &[serde_json::Value]) -> Vec<macaca_proto::LlmMessage> {
    messages
        .iter()
        .filter_map(|msg| {
            let role_str = msg["role"].as_str()?;
            let content = msg["content"].as_str().unwrap_or("").to_string();
            let reasoning_content = msg["reasoning_content"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_string);

            match role_str {
                "system" => Some(macaca_proto::LlmMessage::system(content)),
                "user" => Some(macaca_proto::LlmMessage::user(content)),
                "assistant" => {
                    if let Some(tool_calls) = msg["tool_calls"].as_array() {
                        let tcs: Vec<macaca_proto::ToolCall> = tool_calls
                            .iter()
                            .filter_map(|tc| {
                                let name = tc["function"]["name"]
                                    .as_str()
                                    .or_else(|| tc["name"].as_str())?
                                    .to_string();
                                let args = if let Some(fa) = tc["function"].get("arguments") {
                                    if let Some(s) = fa.as_str() {
                                        serde_json::from_str(s).unwrap_or(serde_json::json!({}))
                                    } else {
                                        fa.clone()
                                    }
                                } else {
                                    tc.get("input").cloned().unwrap_or(serde_json::json!({}))
                                };
                                Some(macaca_proto::ToolCall {
                                    id: tc["id"].as_str().unwrap_or("").to_string(),
                                    name,
                                    arguments: args,
                                })
                            })
                            .collect();
                        if tcs.is_empty() {
                            let mut message = macaca_proto::LlmMessage::assistant(content);
                            message.reasoning_content = reasoning_content;
                            Some(message)
                        } else {
                            let mut message =
                                macaca_proto::LlmMessage::assistant_with_tool_calls(content, tcs);
                            message.reasoning_content = reasoning_content;
                            Some(message)
                        }
                    } else {
                        let mut message = macaca_proto::LlmMessage::assistant(content);
                        message.reasoning_content = reasoning_content;
                        Some(message)
                    }
                }
                "tool" => {
                    let tool_call_id = msg["tool_call_id"].as_str().unwrap_or("");
                    Some(macaca_proto::LlmMessage::tool_result(tool_call_id, content))
                }
                _ => None,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// LegacyToolHandler: macaca_tools::Tool → ToolHandler
// ---------------------------------------------------------------------------

/// Wraps a single tool from `macaca_tools::ToolSet` as a framework `ToolHandler`.
pub struct LegacyToolHandler {
    tool_name: String,
    tool_description: String,
    tool_schema: serde_json::Value,
    tool_set: Arc<dyn macaca_tools::ToolSet>,
}

impl LegacyToolHandler {
    pub fn new(
        name: String,
        description: String,
        schema: serde_json::Value,
        tool_set: Arc<dyn macaca_tools::ToolSet>,
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
        let tool = self
            .tool_set
            .get_tool(&self.tool_name)
            .ok_or_else(|| ToolError::NotFound(self.tool_name.clone()))?;

        let result = tool
            .execute(args)
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
// ToolSetBridge: macaca_tools::ToolSet → Toolkit
// ---------------------------------------------------------------------------

/// Converts a `macaca_tools::ToolSet` into a framework `Toolkit`.
pub struct ToolSetBridge;

impl ToolSetBridge {
    /// Register all tools from a `macaca_tools::ToolSet` into a new `Toolkit`.
    pub fn from_tool_set(tool_set: Arc<dyn macaca_tools::ToolSet>) -> Toolkit {
        let mut toolkit = Toolkit::new();
        let definitions = tool_set.to_definitions();
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
        let result = self
            .tool
            .execute(args)
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
        let result = self
            .tool
            .execute_streaming(args, event_tx)
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
        self.tool.parameters_schema()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_messages_from_json_user() {
        let msgs = vec![serde_json::json!({"role": "user", "content": "hello"})];
        let result = messages_from_json(&msgs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, macaca_proto::LlmRole::User);
        assert_eq!(result[0].content, "hello");
    }

    #[test]
    fn test_messages_from_json_system() {
        let msgs = vec![serde_json::json!({"role": "system", "content": "You are helpful."})];
        let result = messages_from_json(&msgs);
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
        let result = messages_from_json(&msgs);
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
        let result = messages_from_json(&msgs);
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
        let result = messages_from_json(&msgs);
        assert_eq!(result.len(), 3); // invalid role skipped
    }
}
