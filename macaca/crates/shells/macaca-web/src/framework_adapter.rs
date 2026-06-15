//! Shell-owned adapters from service SDK contracts into `macaca-framework`.
//!
//! These adapters intentionally live in `macaca-web`, not `macaca-framework`.
//! The framework exposes provider-neutral `ChatModel`, `ToolHandler`, and
//! `ToolTraceEvent` contracts; the shell owns concrete SDK/service conversion
//! while runtime-host and services own provider construction and side effects.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_host_composition::framework::message::{
    ContentBlock, TextBlock, ThinkingBlock, ToolUseBlock,
};
use macaca_host_composition::framework::model::{
    ChatModel, ChatOptions, ChatResponse, ChatUsage, ModelError,
};
use macaca_host_composition::framework::tool::{
    ToolError, ToolHandler, ToolResponse, ToolTraceEvent, Toolkit,
};
use macaca_proto::{LlmChatCommand, LlmServiceScope};

/// Convert an OS-level LLM response into the framework `ChatResponse` shape.
///
/// The adapter performs only DTO conversion. Routing, credentials, rate limits,
/// budget, and provider choice remain inside the LLM service boundary.
fn chat_response_from_llm(response: macaca_proto::LlmResponse) -> ChatResponse {
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
        for call in tool_calls {
            content_blocks.push(ContentBlock::ToolUse(ToolUseBlock {
                id: call.id.clone(),
                name: call.name.clone(),
                input: call.arguments.clone(),
                raw_input: None,
            }));
        }
    }
    ChatResponse {
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
    }
}

/// Bridges the serviceized SDK LLM client to the framework's `ChatModel` trait.
///
/// This is a Facade/Adapter pair at the shell boundary. The shell converts
/// framework messages into service commands; the service remains the authority
/// for provider routing, policy, credentials, and failure handling.
pub(crate) struct ServiceChatModelAdapter {
    client: Arc<dyn macaca_sdk::SystemLlmClient>,
    scope: LlmServiceScope,
}

impl ServiceChatModelAdapter {
    pub(crate) fn new(
        client: Arc<dyn macaca_sdk::SystemLlmClient>,
        scope: LlmServiceScope,
    ) -> Self {
        Self { client, scope }
    }
}

#[async_trait]
impl ChatModel for ServiceChatModelAdapter {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        let llm_messages =
            macaca_host_composition::framework::llm_wire::messages_from_json_values(&messages);
        let mut llm_options = macaca_proto::LlmOptions {
            model: options.model.clone().unwrap_or_default(),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            ..Default::default()
        };
        if let Some(ref tools) = options.tools {
            llm_options.tools = Some(
                tools
                    .iter()
                    .map(|tool| macaca_proto::ToolDefinition {
                        name: tool["name"].as_str().unwrap_or("").to_string(),
                        description: tool["description"].as_str().unwrap_or("").to_string(),
                        parameters: tool["parameters"].clone(),
                    })
                    .collect(),
            );
        }
        let mut trace = macaca_proto::TraceContext::new(uuid::Uuid::new_v4().to_string());
        trace.session_id = Some(self.scope.session_id.clone());
        trace.agent = Some(self.scope.agent_name.clone());
        tracing::info!(
            trace_id = %trace.trace_id,
            session_id = %self.scope.session_id,
            agent = %self.scope.agent_name,
            "web framework adapter dispatching chat through LLM service"
        );
        let command = LlmChatCommand::new(self.scope.clone(), trace, llm_messages, llm_options)
            .map(|command| {
                if let Some(model) = options.model.as_ref().filter(|model| !model.is_empty()) {
                    command.model_hint(model.clone())
                } else {
                    command
                }
            })
            .map_err(|error| ModelError::Api(error.to_string()))?;
        let result = self
            .client
            .chat(command)
            .await
            .map_err(|error| ModelError::Api(error.to_string()))?;
        Ok(chat_response_from_llm(result.response))
    }

    fn name(&self) -> &str {
        "service-llm"
    }
}

/// Wraps one SDK tool as a framework `ToolHandler`.
pub(crate) struct SingleToolAdapter {
    tool: Box<dyn macaca_host_composition::tools::Tool>,
}

impl SingleToolAdapter {
    pub(crate) fn new(tool: Box<dyn macaca_host_composition::tools::Tool>) -> Self {
        Self { tool }
    }
}

#[async_trait]
impl ToolHandler for SingleToolAdapter {
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResponse, ToolError> {
        let result = macaca_host_composition::tools::ToolCommandExecutor::execute_command(
            self.tool.as_ref(),
            macaca_host_composition::tools::ToolCommand::new(args),
        )
        .await
        .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        Ok(ToolResponse::text(
            serde_json::to_string(&result).unwrap_or_else(|_| result.to_string()),
        ))
    }

    async fn execute_streaming(
        &self,
        args: serde_json::Value,
        event_tx: Option<tokio::sync::mpsc::UnboundedSender<ToolTraceEvent>>,
    ) -> Result<ToolResponse, ToolError> {
        let command = if let Some(event_tx) = event_tx {
            let (sdk_tx, mut sdk_rx) = tokio::sync::mpsc::unbounded_channel::<
                macaca_host_composition::tools::TraceEvent,
            >();
            tokio::spawn(async move {
                while let Some(event) = sdk_rx.recv().await {
                    let _ = event_tx.send(tool_trace_from_sdk(event));
                }
            });
            macaca_host_composition::tools::ToolCommand::with_context(
                args,
                macaca_host_composition::tools::ToolCommandContext {
                    event_tx: Some(sdk_tx),
                    ..Default::default()
                },
            )
        } else {
            macaca_host_composition::tools::ToolCommand::new(args)
        };
        let result = macaca_host_composition::tools::ToolCommandExecutor::execute_command(
            self.tool.as_ref(),
            command,
        )
        .await
        .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        Ok(ToolResponse::text(
            serde_json::to_string(&result).unwrap_or_else(|_| result.to_string()),
        ))
    }

    fn name(&self) -> &str {
        self.tool.name()
    }

    fn description(&self) -> &str {
        self.tool.description()
    }

    fn schema(&self) -> serde_json::Value {
        macaca_host_composition::tools::ToolSchemaProvider::tool_schema(self.tool.as_ref())
    }
}

/// Converts all tools from an SDK catalog into a framework `Toolkit`.
pub(crate) struct ToolSetBridge;

impl ToolSetBridge {
    pub(crate) fn from_tool_set(
        tool_set: Arc<dyn macaca_host_composition::tools::ToolCatalog>,
    ) -> Toolkit {
        let mut toolkit = Toolkit::new();
        for definition in
            macaca_host_composition::tools::ToolCatalog::definitions(tool_set.as_ref())
        {
            toolkit.register(
                Box::new(CatalogToolAdapter::new(
                    definition.name.clone(),
                    definition.description.clone(),
                    definition.parameters.clone(),
                    Arc::clone(&tool_set),
                )),
                None,
            );
        }
        toolkit
    }
}

struct CatalogToolAdapter {
    tool_name: String,
    tool_description: String,
    tool_schema: serde_json::Value,
    tool_set: Arc<dyn macaca_host_composition::tools::ToolCatalog>,
}

impl CatalogToolAdapter {
    fn new(
        name: String,
        description: String,
        schema: serde_json::Value,
        tool_set: Arc<dyn macaca_host_composition::tools::ToolCatalog>,
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
impl ToolHandler for CatalogToolAdapter {
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResponse, ToolError> {
        let tool = macaca_host_composition::tools::ToolCatalog::find_tool(
            self.tool_set.as_ref(),
            &self.tool_name,
        )
        .ok_or_else(|| ToolError::NotFound(self.tool_name.clone()))?;
        let result = macaca_host_composition::tools::ToolCommandExecutor::execute_command(
            tool,
            macaca_host_composition::tools::ToolCommand::new(args),
        )
        .await
        .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        Ok(ToolResponse::text(
            serde_json::to_string(&result).unwrap_or_else(|_| result.to_string()),
        ))
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

fn tool_trace_from_sdk(event: macaca_host_composition::tools::TraceEvent) -> ToolTraceEvent {
    ToolTraceEvent {
        event_type: event.event_type,
        driver_id: event.driver_id,
        timestamp: event.timestamp,
        correlation_id: event.correlation_id,
        title: event.title,
        content: event.content,
        tool_name: event.tool_name,
        tool_input: event.tool_input,
        tool_output: event.tool_output,
        is_error: event.is_error,
        metadata: event.metadata,
    }
}
