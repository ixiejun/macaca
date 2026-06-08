//! LLM provider routing and API key resolution configuration.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{MacacaError, MacacaResult};

/// Top-level LLM routing: default provider, rate limits, and provider table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub default_provider: String,
    pub default_model: Option<String>,
    pub max_tokens_per_request: u32,
    pub rate_limit_rpm: u32,
    pub providers: HashMap<String, LlmProviderConfig>,
}

/// Per-provider LLM endpoint credentials and defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    /// Coding-plan / subscription key (e.g. MiniMax [Token Plan](https://platform.minimaxi.com/docs/token-plan/intro)).
    /// When non-empty after resolution, **takes precedence** over [`Self::api_key`] (pay-as-you-go).
    /// Value: raw key, or `ALL_CAPS` env var name (same rules as `api_key`).
    #[serde(default)]
    pub api_key_plan: Option<String>,
    /// Pay-as-you-go API key: raw `sk-…` string, or env var name if `ALL_CAPS_WITH_UNDERSCORES`
    /// (e.g. `OPENAI_API_KEY`). Used when `api_key_plan` is unset or empty.
    #[serde(default)]
    pub api_key: String,
    pub base_url: String,
    /// Default model for this provider (e.g. "" for DashScope, "gpt-4o" for OpenAI)
    #[serde(default)]
    pub default_model: Option<String>,
}

/// Fluent builder for [`LlmProviderConfig`] used in tests and SDK helpers.
pub struct LlmProviderConfigBuilder {
    inner: LlmProviderConfig,
}

impl LlmProviderConfigBuilder {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            inner: LlmProviderConfig {
                api_key_plan: None,
                api_key: String::new(),
                base_url: base_url.into(),
                default_model: None,
            },
        }
    }

    pub fn api_key_plan(mut self, api_key_plan: impl Into<String>) -> Self {
        self.inner.api_key_plan = Some(api_key_plan.into());
        self
    }

    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.inner.api_key = api_key.into();
        self
    }

    pub fn default_model(mut self, default_model: impl Into<String>) -> Self {
        self.inner.default_model = Some(default_model.into());
        self
    }

    pub fn build(self) -> LlmProviderConfig {
        self.inner
    }
}

/// Resolve one key field: empty → `Ok("")`; `ALL_CAPS` → `std::env::var`; else literal.
pub(crate) fn resolve_llm_key_field(raw: &str) -> MacacaResult<String> {
    let v = raw.trim();
    if v.is_empty() {
        return Ok(String::new());
    }
    let is_env_var_name = v
        .chars()
        .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit());
    if is_env_var_name {
        std::env::var(v).map_err(|_| MacacaError::Config(format!("{v} not set")))
    } else {
        Ok(v.to_string())
    }
}

fn resolve_llm_optional_key(opt: &Option<String>) -> MacacaResult<String> {
    match opt {
        None => Ok(String::new()),
        Some(s) => resolve_llm_key_field(s),
    }
}

impl LlmProviderConfig {
    /// Effective API key: **`api_key_plan` (coding plan) first**, then **`api_key` (按量)**.
    pub fn resolve_api_key(&self) -> MacacaResult<String> {
        let plan = resolve_llm_optional_key(&self.api_key_plan)?;
        if !plan.is_empty() {
            return Ok(plan);
        }
        resolve_llm_key_field(&self.api_key)
    }
}

#[cfg(test)]
mod llm_provider_config_tests {
    use super::*;

    #[test]
    fn resolve_api_key_prefers_api_key_plan() {
        let c = LlmProviderConfig {
            api_key_plan: Some("  sk-plan  ".into()),
            api_key: "SHOULD_NOT_READ".into(),
            base_url: "https://example.com/v1".into(),
            default_model: None,
        };
        assert_eq!(c.resolve_api_key().unwrap(), "sk-plan");
    }

    #[test]
    fn resolve_api_key_falls_back_to_api_key_paygo() {
        let c = LlmProviderConfig {
            api_key_plan: None,
            api_key: "sk-paygo-inline".into(),
            base_url: "https://example.com/v1".into(),
            default_model: None,
        };
        assert_eq!(c.resolve_api_key().unwrap(), "sk-paygo-inline");
    }
}
