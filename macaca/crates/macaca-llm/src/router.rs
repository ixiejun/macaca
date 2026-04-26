use crate::anthropic::AnthropicProvider;
use crate::coding_plans::normalize_openai_compatible_base;
use crate::dashscope::DashScopeProvider;
use crate::openai::OpenAiProvider;
use crate::openai_compatible::OpenAiCompatibleProvider;
use crate::provider::LlmProvider;
use crate::{CostTracker, RateLimiter, ResilientConfig, ResilientLlmWrapper};
use macaca_proto::{
    config::LlmConfig,
    error::{MacacaError, MacacaResult},
    types::{LlmMessage, LlmOptions, LlmResponse},
};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTarget {
    pub provider: String,
    pub model: String,
}

impl ModelTarget {
    pub fn reference(&self) -> String {
        format!("{}:{}", self.provider, self.model)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSelection {
    pub primary: ModelTarget,
    pub fallbacks: Vec<ModelTarget>,
    pub source: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelSelectionRequest {
    pub request_model: Option<String>,
    pub agent_model: Option<String>,
    pub app_model: Option<String>,
    pub app_provider: Option<String>,
    pub system_model: Option<String>,
    pub fallbacks: Vec<String>,
}

/// Routes LLM requests to the correct backend based on the model name prefix.
///
/// Built-in rules:
/// - `gpt-*`    → provider registered as "openai"
/// - `claude-*` → provider registered as "anthropic"
///
/// Additional providers can be registered with [`LlmRouter::register`].
pub struct LlmRouter {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    default_provider: Option<String>,
    default_model: Option<String>,
    provider_defaults: HashMap<String, String>,
}

impl LlmRouter {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: None,
            default_model: None,
            provider_defaults: HashMap::new(),
        }
    }

    /// Register a provider under the given name.
    pub fn register(&mut self, name: impl Into<String>, provider: Arc<dyn LlmProvider>) {
        self.providers.insert(name.into(), provider);
    }

    pub fn set_default_provider(&mut self, provider: impl Into<String>) {
        self.default_provider = Some(provider.into());
    }

    pub fn set_default_model(&mut self, model: impl Into<String>) {
        self.default_model = Some(model.into());
    }

    pub fn set_provider_default_model(
        &mut self,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) {
        self.provider_defaults.insert(provider.into(), model.into());
    }

    pub fn default_model_reference(&self) -> String {
        self.default_target()
            .map(|target| target.reference())
            .unwrap_or_default()
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
        router.set_default_provider(config.default_provider.clone());
        if let Some(default_model) = config.default_model.as_ref().filter(|m| !m.is_empty()) {
            router.set_default_model(default_model.clone());
        }

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
            let base_url = normalize_openai_compatible_base(name, &provider_config.base_url);

            let base_provider: Arc<dyn LlmProvider> = match name.as_str() {
                "openai" => Arc::new(OpenAiProvider::new(&api_key).with_base_url(&base_url)),
                "anthropic" => Arc::new(AnthropicProvider::new(&api_key).with_base_url(&base_url)),
                "dashscope" => Arc::new(DashScopeProvider::new(&api_key).with_base_url(&base_url)),
                other => Arc::new(OpenAiCompatibleProvider::new(other, base_url, &api_key)),
            };

            if let Some(default_model) = provider_config
                .default_model
                .as_ref()
                .filter(|m| !m.is_empty())
            {
                router.set_provider_default_model(name.clone(), default_model.clone());
            }

            let resilient = Arc::new(
                ResilientLlmWrapper::new(base_provider)
                    .with_config(ResilientConfig {
                        max_retries: 3,
                        backoff_base_ms: 60_000,
                        backoff_max_ms: 60_000,
                        retry_on_status: vec![429, 500, 502, 503],
                        max_budget_usd: None,
                        fallback_models: Vec::new(),
                    })
                    .with_rate_limiter(RateLimiter::per_minute(config.rate_limit_rpm as usize))
                    .with_cost_tracker(CostTracker::new()),
            );
            router.register(name, resilient);
        }
        Ok(router)
    }

    fn default_target(&self) -> Option<ModelTarget> {
        if let Some(default_model) = self.default_model.as_ref().filter(|m| !m.is_empty()) {
            return self.resolve_target(default_model, None).ok();
        }

        let provider = self.default_provider.as_ref()?;
        let model = self.provider_defaults.get(provider)?;
        Some(ModelTarget {
            provider: provider.clone(),
            model: model.clone(),
        })
    }

    pub fn resolve_selection(
        &self,
        request: &ModelSelectionRequest,
    ) -> MacacaResult<ModelSelection> {
        let (primary_ref, provider_hint, source) =
            if let Some(model) = request.request_model.as_ref().filter(|m| !m.is_empty()) {
                (Some(model.as_str()), None, "request")
            } else if let Some(model) = request.agent_model.as_ref().filter(|m| !m.is_empty()) {
                (Some(model.as_str()), None, "agent")
            } else if let Some(model) = request.app_model.as_ref().filter(|m| !m.is_empty()) {
                (Some(model.as_str()), request.app_provider.as_deref(), "app")
            } else if request.app_provider.as_deref().is_some() {
                (None, request.app_provider.as_deref(), "app")
            } else if let Some(model) = request.system_model.as_ref().filter(|m| !m.is_empty()) {
                (Some(model.as_str()), None, "system")
            } else {
                (None, None, "system")
            };

        let primary = match primary_ref {
            Some(model_ref) => self.resolve_target(model_ref, provider_hint)?,
            None => self.default_target().ok_or_else(|| {
                MacacaError::Llm("No default provider/model available for model resolution".into())
            })?,
        };

        let mut fallbacks = Vec::new();
        for fallback_ref in &request.fallbacks {
            let target = self.resolve_target(fallback_ref, None)?;
            if target != primary && !fallbacks.contains(&target) {
                fallbacks.push(target);
            }
        }

        if let Some(default_target) = self.default_target() {
            if default_target != primary && !fallbacks.contains(&default_target) {
                fallbacks.push(default_target);
            }
        }

        Ok(ModelSelection {
            primary,
            fallbacks,
            source,
        })
    }

    pub fn resolve_target(
        &self,
        model_ref: &str,
        provider_hint: Option<&str>,
    ) -> MacacaResult<ModelTarget> {
        let trimmed = model_ref.trim();
        if trimmed.is_empty() {
            if let Some(provider) = provider_hint {
                let model = self
                    .provider_defaults
                    .get(provider)
                    .cloned()
                    .ok_or_else(|| {
                        MacacaError::Llm(format!(
                            "Provider '{}' has no default model configured",
                            provider
                        ))
                    })?;
                return Ok(ModelTarget {
                    provider: provider.to_string(),
                    model,
                });
            }

            return self.default_target().ok_or_else(|| {
                MacacaError::Llm(
                    "No default provider/model available for empty model reference".into(),
                )
            });
        }

        if let Some((provider, model)) = trimmed.split_once(':') {
            if self.providers.contains_key(provider) {
                return Ok(ModelTarget {
                    provider: provider.to_string(),
                    model: model.to_string(),
                });
            }
        }

        let provider = if let Some(provider_hint) =
            provider_hint.filter(|p| self.providers.contains_key(*p))
        {
            provider_hint.to_string()
        } else {
            Self::resolve_provider_name(trimmed).to_string()
        };

        Ok(ModelTarget {
            provider,
            model: trimmed.to_string(),
        })
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
        } else if model
            .get(..8)
            .is_some_and(|p| p.eq_ignore_ascii_case("minimax-"))
        {
            // MiniMax-M2.7, MiniMax-M2.7-highspeed, etc. (Token Plan / OpenAI-compatible)
            "minimax"
        } else {
            // Fall back to using the model string itself as the provider key,
            // allowing callers to register arbitrary names.
            model
        }
    }

    async fn chat_once(
        &self,
        provider_name: &str,
        messages: &[LlmMessage],
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        let provider = self.providers.get(provider_name).ok_or_else(|| {
            MacacaError::Llm(format!(
                "No provider registered for model '{}' (resolved to '{}')",
                options.model, provider_name
            ))
        })?;
        provider.chat(messages.to_vec(), options).await
    }

    pub async fn chat_with_selection(
        &self,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
        selection: &ModelSelection,
    ) -> MacacaResult<LlmResponse> {
        let mut route_plan = Vec::with_capacity(1 + selection.fallbacks.len());
        route_plan.push(selection.primary.clone());
        route_plan.extend(selection.fallbacks.iter().cloned());

        let mut last_err: Option<MacacaError> = None;
        for target in route_plan {
            let mut routed = options.clone();
            routed.model = target.model.clone();
            match self.chat_once(&target.provider, &messages, &routed).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    tracing::warn!(
                        provider = %target.provider,
                        model = %target.model,
                        error = %err,
                        "LLM route failed"
                    );
                    last_err = Some(err);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| MacacaError::Llm("No provider route available".into())))
    }

    /// Send a chat request, auto-routing based on `options.model`.
    pub async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        let system_model = self.default_target().map(|target| target.reference());
        let selection = self.resolve_selection(&ModelSelectionRequest {
            request_model: Some(options.model.clone()),
            system_model,
            ..Default::default()
        })?;
        self.chat_with_selection(messages, options, &selection)
            .await
    }
}

#[async_trait::async_trait]
impl LlmProvider for LlmRouter {
    fn name(&self) -> &str {
        self.default_provider.as_deref().unwrap_or("router")
    }

    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        LlmRouter::chat(self, messages, options).await
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
    use macaca_proto::types::TokenUsage;

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
                .map(|m| m.content.clone())
                .unwrap_or_default();
            Ok(LlmResponse {
                content,
                reasoning_content: None,
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
        assert_eq!(
            LlmRouter::resolve_provider_name("qwen2.5-coder-32b"),
            "dashscope"
        );
    }

    #[test]
    fn resolve_provider_name_deepseek() {
        assert_eq!(
            LlmRouter::resolve_provider_name("deepseek-chat"),
            "deepseek"
        );
        assert_eq!(
            LlmRouter::resolve_provider_name("deepseek-coder"),
            "deepseek"
        );
    }

    #[test]
    fn resolve_provider_name_openrouter() {
        // Models with "/" are routed to openrouter (aggregator platform)
        assert_eq!(
            LlmRouter::resolve_provider_name("qwen/qwen3.6-plus:free"),
            "openrouter"
        );
        assert_eq!(
            LlmRouter::resolve_provider_name("openai/gpt-4o"),
            "openrouter"
        );
        assert_eq!(
            LlmRouter::resolve_provider_name("anthropic/claude-3.5-sonnet"),
            "openrouter"
        );
        assert_eq!(
            LlmRouter::resolve_provider_name("meta-llama/llama-3-70b"),
            "openrouter"
        );
    }

    #[test]
    fn resolve_provider_name_unknown_falls_back_to_model() {
        assert_eq!(LlmRouter::resolve_provider_name("llama-3"), "llama-3");
    }

    #[test]
    fn resolve_provider_name_minimax_models() {
        assert_eq!(LlmRouter::resolve_provider_name("MiniMax-M2.7"), "minimax");
        assert_eq!(
            LlmRouter::resolve_provider_name("MiniMax-M2.7-highspeed"),
            "minimax"
        );
        assert_eq!(LlmRouter::resolve_provider_name("minimax-m2"), "minimax");
    }

    #[tokio::test]
    async fn router_dispatches_to_registered_provider() {
        let mut router = LlmRouter::new();
        router.register(
            "openai",
            Arc::new(EchoProvider {
                name: "openai".into(),
            }),
        );

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

    #[test]
    fn resolve_selection_prefers_agent_over_app_and_system() {
        let mut router = LlmRouter::new();
        router.set_default_provider("openai");
        router.set_provider_default_model("openai", "gpt-4o");
        router.register(
            "openai",
            Arc::new(EchoProvider {
                name: "openai".into(),
            }),
        );
        router.register(
            "anthropic",
            Arc::new(EchoProvider {
                name: "anthropic".into(),
            }),
        );

        let selection = router
            .resolve_selection(&ModelSelectionRequest {
                agent_model: Some("anthropic:claude-sonnet-4".into()),
                app_model: Some("gpt-4.1".into()),
                app_provider: Some("openai".into()),
                system_model: Some("openai:gpt-4o".into()),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(
            selection.primary,
            ModelTarget {
                provider: "anthropic".into(),
                model: "claude-sonnet-4".into(),
            }
        );
        assert_eq!(selection.source, "agent");
    }

    #[test]
    fn resolve_selection_uses_app_provider_hint() {
        let mut router = LlmRouter::new();
        router.set_default_provider("openai");
        router.set_provider_default_model("openai", "gpt-4o");
        router.set_provider_default_model("anthropic", "claude-sonnet-4");
        router.register(
            "openai",
            Arc::new(EchoProvider {
                name: "openai".into(),
            }),
        );
        router.register(
            "anthropic",
            Arc::new(EchoProvider {
                name: "anthropic".into(),
            }),
        );

        let selection = router
            .resolve_selection(&ModelSelectionRequest {
                app_model: Some("claude-3-5-sonnet-20241022".into()),
                app_provider: Some("anthropic".into()),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(selection.primary.provider, "anthropic");
        assert_eq!(selection.primary.model, "claude-3-5-sonnet-20241022");
    }

    #[tokio::test]
    async fn chat_with_selection_uses_fallback_target() {
        struct FailingProvider;

        #[async_trait]
        impl LlmProvider for FailingProvider {
            fn name(&self) -> &str {
                "fail"
            }

            async fn chat(
                &self,
                _messages: Vec<LlmMessage>,
                _options: &LlmOptions,
            ) -> MacacaResult<LlmResponse> {
                Err(MacacaError::Llm("boom".into()))
            }
        }

        let mut router = LlmRouter::new();
        router.register("openai", Arc::new(FailingProvider));
        router.register(
            "anthropic",
            Arc::new(EchoProvider {
                name: "anthropic".into(),
            }),
        );

        let selection = ModelSelection {
            primary: ModelTarget {
                provider: "openai".into(),
                model: "gpt-4o".into(),
            },
            fallbacks: vec![ModelTarget {
                provider: "anthropic".into(),
                model: "claude-sonnet-4".into(),
            }],
            source: "test",
        };

        let response = router
            .chat_with_selection(
                vec![LlmMessage::user("hello")],
                &LlmOptions {
                    model: "gpt-4o".into(),
                    ..Default::default()
                },
                &selection,
            )
            .await
            .unwrap();

        assert_eq!(response.model, "claude-sonnet-4");
        assert_eq!(response.content, "hello");
    }
}
