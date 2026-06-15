use crate::anthropic::AnthropicProvider;
use crate::coding_plans::normalize_openai_compatible_base;
use crate::dashscope::DashScopeProvider;
use crate::openai::OpenAiProvider;
use crate::openai_compatible::OpenAiCompatibleProvider;
use crate::provider::LlmProvider;
use crate::resolver::ResolverChain;
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
    resolver: ResolverChain,
}

impl LlmRouter {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: None,
            default_model: None,
            provider_defaults: HashMap::new(),
            resolver: ResolverChain::built_in(),
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
            self.resolver.resolve_provider(trimmed)
        };

        Ok(ModelTarget {
            provider,
            model: trimmed.to_string(),
        })
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
#[path = "router_tests.rs"]
mod router_tests;
