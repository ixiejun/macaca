use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use macaca_proto::{
    error::MacacaResult,
    types::{LlmMessage, LlmOptions, LlmResponse},
};

use crate::{cost::CostTracker, provider::LlmProvider, rate_limit::RateLimiter};

/// Configuration for retry and backoff behaviour.
pub struct ResilientConfig {
    /// Maximum number of retry attempts after the initial failure.
    pub max_retries: u32,
    /// Base delay in milliseconds for the first retry.
    pub backoff_base_ms: u64,
    /// Upper bound on the computed backoff delay in milliseconds.
    pub backoff_max_ms: u64,
    /// HTTP status codes that are considered retryable (matched against the error string).
    pub retry_on_status: Vec<u16>,
    /// Optional budget cap in USD. When set, calls exceeding this are rejected
    /// with `MacacaError::BudgetExceeded`.
    pub max_budget_usd: Option<f64>,
    /// Fallback model names to try when the primary model fails all retries.
    /// Each fallback gets its own full retry cycle.
    pub fallback_models: Vec<String>,
}

impl Default for ResilientConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_base_ms: 1000,
            backoff_max_ms: 8000,
            retry_on_status: vec![429, 500, 502, 503],
            max_budget_usd: None,
            fallback_models: Vec::new(),
        }
    }
}

/// Wraps any [`LlmProvider`] with retry/backoff, optional rate limiting, and
/// optional cost tracking.
pub struct ResilientLlmWrapper {
    inner: Arc<dyn LlmProvider>,
    config: ResilientConfig,
    rate_limiter: Option<RateLimiter>,
    cost_tracker: Option<CostTracker>,
}

impl ResilientLlmWrapper {
    pub fn new(inner: Arc<dyn LlmProvider>) -> Self {
        Self {
            inner,
            config: ResilientConfig::default(),
            rate_limiter: None,
            cost_tracker: None,
        }
    }

    pub fn with_config(mut self, config: ResilientConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    pub fn with_cost_tracker(mut self, tracker: CostTracker) -> Self {
        self.cost_tracker = Some(tracker);
        self
    }

    /// Compute the backoff duration for the given attempt index (0-based).
    ///
    /// Formula: `min(base * 2^attempt, max)`
    fn backoff_duration(&self, attempt: u32) -> Duration {
        // 2^attempt, capped at u64::MAX to avoid overflow before the min().
        let multiplier = 1u64.checked_shl(attempt.min(63)).unwrap_or(u64::MAX);
        let delay_ms = self
            .config
            .backoff_base_ms
            .saturating_mul(multiplier)
            .min(self.config.backoff_max_ms);
        Duration::from_millis(delay_ms)
    }

    /// Returns true if the error is considered retryable.
    ///
    /// Checks whether the error string contains any of the configured status
    /// codes or network-related keywords.
    fn is_retryable(&self, err: &macaca_proto::error::MacacaError) -> bool {
        let msg = err.to_string();
        let lower = msg.to_lowercase();

        // Check for transient error keywords (network, parse, server issues).
        if lower.contains("network")
            || lower.contains("timeout")
            || lower.contains("connection")
            || lower.contains("timed out")
            || lower.contains("parse failed")
            || lower.contains("error decoding")
            || lower.contains("response read failed")
            || lower.contains("broken pipe")
            || lower.contains("reset by peer")
            || lower.contains("unexpected eof")
        {
            return true;
        }

        // Check for configured HTTP status codes in the error string.
        for code in &self.config.retry_on_status {
            if msg.contains(&code.to_string()) {
                return true;
            }
        }

        false
    }

    /// Attempt a chat call with a full retry cycle for the model specified in `options`.
    async fn chat_with_retries(
        &self,
        messages: &[LlmMessage],
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        let mut last_err: Option<macaca_proto::error::MacacaError> = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let delay = self.backoff_duration(attempt - 1);
                tracing::warn!(
                    provider = self.inner.name(),
                    model = options.model,
                    attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying LLM request after error: {}",
                    last_err.as_ref().map(|e| e.to_string()).unwrap_or_default()
                );
                tokio::time::sleep(delay).await;
            }

            match self.inner.chat(messages.to_vec(), options).await {
                Ok(response) => {
                    if let Some(ref tracker) = self.cost_tracker {
                        tracker.record(&options.model, response.usage);
                    }
                    return Ok(response);
                }
                Err(err) => {
                    if self.is_retryable(&err) {
                        last_err = Some(err);
                    } else {
                        return Err(err);
                    }
                }
            }
        }

        Err(last_err.expect("loop body always sets last_err before reaching here"))
    }
}

#[async_trait]
impl LlmProvider for ResilientLlmWrapper {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        // Check budget before attempting any call.
        if let (Some(max_usd), Some(ref tracker)) = (self.config.max_budget_usd, &self.cost_tracker)
        {
            if tracker.is_over_budget(max_usd) {
                return Err(macaca_proto::error::MacacaError::BudgetExceeded(format!(
                    "Cost budget exceeded: spent ${:.4}, limit ${:.4}",
                    tracker.total_cost_usd(),
                    max_usd
                )));
            }
        }

        // Honour the rate limiter before any attempt (once for the full call, including fallbacks).
        if let Some(ref limiter) = self.rate_limiter {
            limiter.acquire().await;
        }

        // Try primary model.
        match self.chat_with_retries(&messages, options).await {
            Ok(response) => return Ok(response),
            Err(primary_err) => {
                // Non-retryable errors (e.g. auth failures) bypass the fallback chain.
                if self.config.fallback_models.is_empty() || !self.is_retryable(&primary_err) {
                    return Err(primary_err);
                }

                let mut last_err = primary_err;
                for fallback_model in &self.config.fallback_models {
                    tracing::warn!(
                        primary = options.model,
                        fallback = fallback_model,
                        "Primary model failed, attempting fallback"
                    );
                    let mut fallback_options = options.clone();
                    fallback_options.model = fallback_model.clone();

                    match self.chat_with_retries(&messages, &fallback_options).await {
                        Ok(response) => {
                            tracing::warn!(
                                model = fallback_model,
                                "Request succeeded using fallback model"
                            );
                            return Ok(response);
                        }
                        Err(err) => {
                            if !self.is_retryable(&err) {
                                // Non-retryable error from a fallback — stop immediately.
                                return Err(err);
                            }
                            last_err = err;
                        }
                    }
                }

                Err(last_err)
            }
        }
    }
}

/// Contract tests for resilient LLM retry, backoff, budget, and fallback chain.
#[cfg(test)]
mod tests;
