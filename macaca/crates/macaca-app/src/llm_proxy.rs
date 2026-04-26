//! `LlmProxy` — mediates between app-declared LLM defaults and user overrides.
//!
//! **Security invariant:** The application never sees the user's API key.
//! It declares a preferred provider/model; the kernel resolves the actual
//! credentials from the user's configuration.

use async_trait::async_trait;
use std::sync::Arc;

use macaca_llm::LlmProvider;
use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaResult};

use crate::model::AppLlmConfig;

/// User-supplied override for an app's LLM configuration.
#[derive(Debug, Clone)]
pub struct UserLlmOverride {
    /// Override the provider (e.g., switch from "openai" to "anthropic").
    pub provider: Option<String>,
    /// Override the model (e.g., switch from "gpt-4" to "claude-sonnet-4-20250514").
    pub model: Option<String>,
}

/// Proxies LLM requests, applying app defaults and user overrides.
///
/// The app declares its preferred `AppLlmConfig`. The user can override
/// the provider and/or model. The proxy delegates to the kernel's real
/// `LlmProvider` (which holds the actual API keys).
pub struct LlmProxy {
    /// The kernel's real LLM provider (holds credentials).
    inner: Arc<dyn LlmProvider>,
    /// App-declared defaults.
    app_defaults: Option<AppLlmConfig>,
    /// User overrides (take priority over app defaults).
    user_overrides: Option<UserLlmOverride>,
}

impl LlmProxy {
    /// Create a new LLM proxy.
    pub fn new(
        inner: Arc<dyn LlmProvider>,
        app_defaults: Option<AppLlmConfig>,
        user_overrides: Option<UserLlmOverride>,
    ) -> Self {
        Self {
            inner,
            app_defaults,
            user_overrides,
        }
    }

    /// Resolve the effective model name based on priority:
    /// user override > app default > options.model (from agent).
    fn resolve_model(&self, agent_model: &str) -> String {
        // User override takes highest priority.
        if let Some(ref overrides) = self.user_overrides {
            if let Some(ref model) = overrides.model {
                return model.clone();
            }
        }

        // App default takes second priority.
        if let Some(ref defaults) = self.app_defaults {
            return defaults.model.clone();
        }

        // Fall back to whatever the agent requested.
        agent_model.to_string()
    }
}

#[async_trait]
impl LlmProvider for LlmProxy {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        // Apply model resolution.
        let mut resolved_options = options.clone();
        resolved_options.model = self.resolve_model(&options.model);

        // Delegate to the real provider (which has the API keys).
        self.inner.chat(messages, &resolved_options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::TokenUsage;

    struct MockInnerLlm;

    #[async_trait]
    impl LlmProvider for MockInnerLlm {
        fn name(&self) -> &str {
            "mock-inner"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            // Echo back which model was requested so tests can verify resolution.
            Ok(LlmResponse {
                content: format!("model={}", options.model),
                reasoning_content: None,
                model: options.model.clone(),
                usage: TokenUsage::default(),
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    #[tokio::test]
    async fn no_overrides_uses_agent_model() {
        let proxy = LlmProxy::new(Arc::new(MockInnerLlm), None, None);
        let opts = LlmOptions {
            model: "agent-model".into(),
            ..Default::default()
        };
        let resp = proxy.chat(vec![], &opts).await.unwrap();
        assert_eq!(resp.model, "agent-model");
    }

    #[tokio::test]
    async fn app_defaults_override_agent() {
        let proxy = LlmProxy::new(
            Arc::new(MockInnerLlm),
            Some(AppLlmConfig {
                provider: "openai".into(),
                model: "gpt-4".into(),
            }),
            None,
        );
        let opts = LlmOptions {
            model: "agent-model".into(),
            ..Default::default()
        };
        let resp = proxy.chat(vec![], &opts).await.unwrap();
        assert_eq!(resp.model, "gpt-4");
    }

    #[tokio::test]
    async fn user_override_takes_priority() {
        let proxy = LlmProxy::new(
            Arc::new(MockInnerLlm),
            Some(AppLlmConfig {
                provider: "openai".into(),
                model: "gpt-4".into(),
            }),
            Some(UserLlmOverride {
                provider: Some("anthropic".into()),
                model: Some("claude-sonnet-4-20250514".into()),
            }),
        );
        let opts = LlmOptions {
            model: "agent-model".into(),
            ..Default::default()
        };
        let resp = proxy.chat(vec![], &opts).await.unwrap();
        assert_eq!(resp.model, "claude-sonnet-4-20250514");
    }

    #[tokio::test]
    async fn partial_user_override() {
        let proxy = LlmProxy::new(
            Arc::new(MockInnerLlm),
            Some(AppLlmConfig {
                provider: "openai".into(),
                model: "gpt-4".into(),
            }),
            Some(UserLlmOverride {
                provider: None,
                model: None, // No model override — falls through to app default.
            }),
        );
        let opts = LlmOptions::default();
        let resp = proxy.chat(vec![], &opts).await.unwrap();
        assert_eq!(resp.model, "gpt-4");
    }
}
