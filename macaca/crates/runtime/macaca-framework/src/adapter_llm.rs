//! LLM adapters for `macaca-framework`.
//!
//! This module is intentionally separate from tool adapters so the framework
//! compatibility layer stays small and reviewable.  It contains the deprecated
//! provider-backed adapters plus the Route C service-backed adapter.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_llm::LlmProvider;

use crate::message::{ContentBlock, TextBlock, ThinkingBlock, ToolUseBlock};
use crate::model::{ChatModel, ChatOptions, ChatResponse, ChatUsage, ModelError};

/// Convert an OS-level LLM response into the framework `ChatResponse` shape.
///
/// Multiple adapters need this conversion while the framework migrates from
/// provider-backed dispatch to service-backed dispatch.  Keeping the mapping in
/// one helper prevents behavioral drift between compatibility and Route C
/// service paths.
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
        for tc in tool_calls {
            content_blocks.push(ContentBlock::ToolUse(ToolUseBlock {
                id: tc.id.clone(),
                name: tc.name.clone(),
                input: tc.arguments.clone(),
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

/// Bridges `macaca_llm::LlmProvider` to the framework's `ChatModel` trait.
#[deprecated(note = "Use ServiceChatModelAdapter over macaca_sdk::SystemLlmClient for new code")]
pub struct LlmProviderAdapter {
    provider: Arc<dyn macaca_llm::LlmProvider>,
}

#[allow(deprecated)]
impl LlmProviderAdapter {
    pub fn new(provider: Arc<dyn macaca_llm::LlmProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
#[allow(deprecated)]
impl ChatModel for LlmProviderAdapter {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        let llm_messages = crate::llm_wire::messages_from_json_values(&messages);
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
        Ok(chat_response_from_llm(response))
    }

    fn name(&self) -> &str {
        self.provider.name()
    }
}

/// Bridges `macaca_llm::LlmRouter` to the framework's `ChatModel` trait with
/// a pre-resolved default route plan.
#[deprecated(note = "Use ServiceChatModelAdapter over macaca_sdk::SystemLlmClient for new code")]
pub struct RoutedLlmAdapter {
    router: Arc<macaca_llm::LlmRouter>,
    default_selection: macaca_llm::ModelSelection,
}

#[allow(deprecated)]
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
#[allow(deprecated)]
impl ChatModel for RoutedLlmAdapter {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        let llm_messages = crate::llm_wire::messages_from_json_values(&messages);
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
        Ok(chat_response_from_llm(response))
    }

    fn name(&self) -> &str {
        self.router.name()
    }
}

/// Bridges the serviceized SDK LLM client to the framework's `ChatModel` trait.
///
/// This adapter is the preferred Route C path.  It uses the Facade pattern:
/// framework agents keep depending on `ChatModel`, while model dispatch flows
/// through `SystemLlmClient` and the LLM Service boundary.  The adapter only
/// performs message/options conversion and never constructs concrete providers.
#[cfg(feature = "service-clients")]
pub struct ServiceChatModelAdapter {
    client: Arc<dyn macaca_sdk::SystemLlmClient>,
    scope: macaca_llm::LlmServiceScope,
}

#[cfg(feature = "service-clients")]
impl ServiceChatModelAdapter {
    /// Create a service-backed chat model for one application/session/agent scope.
    pub fn new(
        client: Arc<dyn macaca_sdk::SystemLlmClient>,
        scope: macaca_llm::LlmServiceScope,
    ) -> Self {
        Self { client, scope }
    }
}

#[cfg(feature = "service-clients")]
#[async_trait]
impl ChatModel for ServiceChatModelAdapter {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        let llm_messages = crate::llm_wire::messages_from_json_values(&messages);
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
                    .map(|t| macaca_proto::ToolDefinition {
                        name: t["name"].as_str().unwrap_or("").to_string(),
                        description: t["description"].as_str().unwrap_or("").to_string(),
                        parameters: t["parameters"].clone(),
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
            "framework service chat model dispatching through LLM service"
        );
        let command =
            macaca_llm::LlmChatCommand::new(self.scope.clone(), trace, llm_messages, llm_options)
                .map_err(|err| ModelError::Api(err.to_string()))?;
        let result = self
            .client
            .chat(command)
            .await
            .map_err(|err| ModelError::Api(err.to_string()))?;
        Ok(chat_response_from_llm(result.response))
    }

    fn name(&self) -> &str {
        "service-llm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::types::TokenUsage;
    use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaError, MacacaResult};

    struct EchoProvider {
        name: String,
    }

    #[async_trait]
    impl LlmProvider for EchoProvider {
        fn name(&self) -> &str {
            &self.name
        }

        async fn chat(
            &self,
            messages: Vec<LlmMessage>,
            options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            let content = messages
                .last()
                .map(|message| message.content.clone())
                .unwrap_or_default();
            Ok(LlmResponse {
                content,
                reasoning_content: None,
                model: options.model.clone(),
                usage: TokenUsage::default(),
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    struct FailingProvider;

    #[async_trait]
    impl LlmProvider for FailingProvider {
        fn name(&self) -> &str {
            "failing"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            Err(MacacaError::Llm("forced failure".into()))
        }
    }

    fn text_content(response: &ChatResponse) -> String {
        response
            .content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    #[tokio::test]
    async fn routed_adapter_uses_default_selection_and_fallbacks_without_model_override() {
        let mut router = macaca_llm::LlmRouter::new();
        router.register("openai", Arc::new(FailingProvider));
        router.register(
            "anthropic",
            Arc::new(EchoProvider {
                name: "anthropic".into(),
            }),
        );
        let selection = macaca_llm::ModelSelection {
            primary: macaca_llm::ModelTarget {
                provider: "openai".into(),
                model: "gpt-4o".into(),
            },
            fallbacks: vec![macaca_llm::ModelTarget {
                provider: "anthropic".into(),
                model: "claude-sonnet-4".into(),
            }],
            source: "test",
        };
        let adapter = RoutedLlmAdapter::new(Arc::new(router), selection);
        let response = adapter
            .chat(
                vec![serde_json::json!({"role": "user", "content": "hello"})],
                &ChatOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(text_content(&response), "hello");
    }

    #[tokio::test]
    async fn routed_adapter_explicit_model_override_uses_router_resolution() {
        let mut router = macaca_llm::LlmRouter::new();
        router.register("openai", Arc::new(FailingProvider));
        router.register(
            "anthropic",
            Arc::new(EchoProvider {
                name: "anthropic".into(),
            }),
        );
        let selection = macaca_llm::ModelSelection {
            primary: macaca_llm::ModelTarget {
                provider: "openai".into(),
                model: "gpt-4o".into(),
            },
            fallbacks: Vec::new(),
            source: "test",
        };
        let adapter = RoutedLlmAdapter::new(Arc::new(router), selection);
        let response = adapter
            .chat(
                vec![serde_json::json!({"role": "user", "content": "override"})],
                &ChatOptions {
                    model: Some("anthropic:claude-sonnet-4".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(text_content(&response), "override");
    }
}
