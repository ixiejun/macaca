use std::{collections::HashMap, sync::Arc};
use macaca_proto::{
    error::{MacacaError, MacacaResult},
    types::{LlmMessage, LlmOptions, LlmResponse},
    config::LlmConfig,
};
use crate::provider::LlmProvider;
use crate::openai::OpenAiProvider;
use crate::anthropic::AnthropicProvider;
use crate::dashscope::DashScopeProvider;
use crate::openai_compatible::OpenAiCompatibleProvider;

/// Routes LLM requests to the correct backend based on the model name prefix.
///
/// Built-in rules:
/// - `gpt-*`    → provider registered as "openai"
/// - `claude-*` → provider registered as "anthropic"
///
/// Additional providers can be registered with [`LlmRouter::register`].
pub struct LlmRouter {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
}

impl LlmRouter {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider under the given name.
    pub fn register(&mut self, name: impl Into<String>, provider: Arc<dyn LlmProvider>) {
        self.providers.insert(name.into(), provider);
    }

    /// Build a router from config, creating providers for each configured entry.
    ///
    /// Known provider names (`openai`, `anthropic`, `dashscope`) get their native
    /// implementation; everything else is treated as an OpenAI-compatible endpoint.
    ///
    /// Providers whose API key cannot be resolved (e.g. env var not set) are
    /// skipped with a warning instead of failing the entire router.
    pub fn from_config(config: &LlmConfig) -> MacacaResult<Self> {
        let mut router = Self::new();
        for (name, provider_config) in &config.providers {
            let api_key = match provider_config.resolve_api_key() {
                Ok(key) => key,
                Err(_) => {
                    tracing::warn!(
                        provider = %name,
                        "Skipping provider: API key not available"
                    );
                    continue;
                }
            };
            let base_url = &provider_config.base_url;

            let provider: Arc<dyn LlmProvider> = match name.as_str() {
                "openai" => Arc::new(
                    OpenAiProvider::new(&api_key).with_base_url(base_url),
                ),
                "anthropic" => Arc::new(
                    AnthropicProvider::new(&api_key).with_base_url(base_url),
                ),
                "dashscope" => Arc::new(
                    DashScopeProvider::new(&api_key).with_base_url(base_url),
                ),
                other => Arc::new(
                    OpenAiCompatibleProvider::new(other, base_url, &api_key),
                ),
            };
            router.register(name, provider);
        }
        Ok(router)
    }

    /// Resolve the provider name from the model string.
    ///
    /// Built-in routing rules:
    /// - `gpt-*`, `o1*`, `o3*` → `"openai"`
    /// - `claude-*` → `"anthropic"`
    /// - `qwen*` → `"dashscope"` (covers qwen-*, qwen2-*, qwen3-*, etc.)
    /// - `deepseek-*` → `"deepseek"`
    /// - anything else → uses the model string as the provider key
    fn resolve_provider_name(model: &str) -> &str {
        // Models with "/" separator are from aggregator platforms (e.g. OpenRouter).
        // Format: "provider/model-name" or "provider/model:variant"
        // Examples: "qwen/qwen3.6-plus:free", "openai/gpt-4o", "anthropic/claude-3.5-sonnet"
        // Must check BEFORE bare-prefix rules so "qwen/..." goes to openrouter, not dashscope.
        if model.contains('/') {
            "openrouter"
        } else if model.starts_with("gpt-") || model.starts_with("o1") || model.starts_with("o3") {
            "openai"
        } else if model.starts_with("claude-") {
            "anthropic"
        } else if model.starts_with("qwen") {
            "dashscope"
        } else if model.starts_with("deepseek-") {
            "deepseek"
        } else {
            // Fall back to using the model string itself as the provider key,
            // allowing callers to register arbitrary names.
            model
        }
    }

    /// Send a chat request, auto-routing based on `options.model`.
    pub async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        let provider_name = Self::resolve_provider_name(&options.model);
        let provider = self.providers.get(provider_name).ok_or_else(|| {
            MacacaError::Llm(format!(
                "No provider registered for model '{}' (resolved to '{}')",
                options.model, provider_name
            ))
        })?;
        provider.chat(messages, options).await
    }
}

impl Default for LlmRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use macaca_proto::types::{LlmRole, TokenUsage};

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
            let content = messages.last().map(|m| m.content.clone()).unwrap_or_default();
            Ok(LlmResponse {
                content,
                model: options.model.clone(),
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    #[test]
    fn resolve_provider_name_gpt() {
        assert_eq!(LlmRouter::resolve_provider_name("gpt-4o"), "openai");
        assert_eq!(LlmRouter::resolve_provider_name("gpt-4-turbo"), "openai");
    }

    #[test]
    fn resolve_provider_name_claude() {
        assert_eq!(
            LlmRouter::resolve_provider_name("claude-3-5-sonnet-20241022"),
            "anthropic"
        );
    }

    #[test]
    fn resolve_provider_name_qwen() {
        assert_eq!(LlmRouter::resolve_provider_name("qwen-turbo"), "dashscope");
        assert_eq!(LlmRouter::resolve_provider_name("qwen-max"), "dashscope");
        assert_eq!(LlmRouter::resolve_provider_name("qwen-plus"), "dashscope");
        // Qwen2/3 series
        assert_eq!(LlmRouter::resolve_provider_name("qwen3-max"), "dashscope");
        assert_eq!(LlmRouter::resolve_provider_name("qwen2.5-coder-32b"), "dashscope");
    }

    #[test]
    fn resolve_provider_name_deepseek() {
        assert_eq!(LlmRouter::resolve_provider_name("deepseek-chat"), "deepseek");
        assert_eq!(LlmRouter::resolve_provider_name("deepseek-coder"), "deepseek");
    }

    #[test]
    fn resolve_provider_name_openrouter() {
        // Models with "/" are routed to openrouter (aggregator platform)
        assert_eq!(LlmRouter::resolve_provider_name("qwen/qwen3.6-plus:free"), "openrouter");
        assert_eq!(LlmRouter::resolve_provider_name("openai/gpt-4o"), "openrouter");
        assert_eq!(LlmRouter::resolve_provider_name("anthropic/claude-3.5-sonnet"), "openrouter");
        assert_eq!(LlmRouter::resolve_provider_name("meta-llama/llama-3-70b"), "openrouter");
    }

    #[test]
    fn resolve_provider_name_unknown_falls_back_to_model() {
        assert_eq!(LlmRouter::resolve_provider_name("llama-3"), "llama-3");
    }

    #[tokio::test]
    async fn router_dispatches_to_registered_provider() {
        let mut router = LlmRouter::new();
        router.register("openai", Arc::new(EchoProvider { name: "openai".into() }));

        let messages = vec![LlmMessage::user("hello")];
        let options = LlmOptions {
            model: "gpt-4o".into(),
            ..Default::default()
        };

        let resp = router.chat(messages, &options).await.unwrap();
        assert_eq!(resp.content, "hello");
    }

    #[tokio::test]
    async fn router_returns_error_for_missing_provider() {
        let router = LlmRouter::new();
        let messages = vec![LlmMessage::user("hi")];
        let options = LlmOptions {
            model: "gpt-4".into(),
            ..Default::default()
        };
        assert!(router.chat(messages, &options).await.is_err());
    }
}
