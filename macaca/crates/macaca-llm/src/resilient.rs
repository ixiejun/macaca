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

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{error::MacacaError, types::TokenUsage};
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    struct FailThenSucceedProvider {
        fail_times: u32,
        calls: Arc<AtomicU32>,
        error_msg: String,
    }

    #[async_trait]
    impl LlmProvider for FailThenSucceedProvider {
        fn name(&self) -> &str {
            "test-provider"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call < self.fail_times {
                Err(MacacaError::Llm(self.error_msg.clone()))
            } else {
                Ok(LlmResponse {
                    content: "ok".into(),
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
    }

    /// Provider that fails for a specific model name, succeeds for all others.
    struct ModelSelectiveProvider {
        fail_model: String,
        calls: Arc<AtomicU32>,
        error_msg: String,
    }

    #[async_trait]
    impl LlmProvider for ModelSelectiveProvider {
        fn name(&self) -> &str {
            "model-selective-provider"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if options.model == self.fail_model {
                Err(MacacaError::Llm(self.error_msg.clone()))
            } else {
                Ok(LlmResponse {
                    content: "ok".into(),
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
    }

    /// Provider that always fails regardless of model.
    struct AlwaysFailProvider {
        calls: Arc<AtomicU32>,
        error_msg: String,
    }

    #[async_trait]
    impl LlmProvider for AlwaysFailProvider {
        fn name(&self) -> &str {
            "always-fail-provider"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(MacacaError::Llm(self.error_msg.clone()))
        }
    }

    fn fast_config(max_retries: u32) -> ResilientConfig {
        ResilientConfig {
            max_retries,
            backoff_base_ms: 0, // no actual sleeping in tests
            backoff_max_ms: 0,
            retry_on_status: vec![429, 500, 502, 503],
            max_budget_usd: None,
            fallback_models: Vec::new(),
        }
    }

    fn fast_config_with_fallbacks(
        max_retries: u32,
        fallback_models: Vec<String>,
    ) -> ResilientConfig {
        ResilientConfig {
            fallback_models,
            ..fast_config(max_retries)
        }
    }

    #[tokio::test]
    async fn retries_on_429_and_succeeds() {
        let calls = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(FailThenSucceedProvider {
            fail_times: 2,
            calls: calls.clone(),
            error_msg: "status 429 Too Many Requests".into(),
        });
        let wrapper = ResilientLlmWrapper::new(provider).with_config(fast_config(3));
        let options = LlmOptions {
            model: "gpt-4".into(),
            ..Default::default()
        };
        let result = wrapper.chat(vec![LlmMessage::user("hi")], &options).await;
        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 3); // 2 failures + 1 success
    }

    #[tokio::test]
    async fn non_retryable_error_returned_immediately() {
        let calls = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(FailThenSucceedProvider {
            fail_times: 10,
            calls: calls.clone(),
            error_msg: "invalid api key".into(),
        });
        let wrapper = ResilientLlmWrapper::new(provider).with_config(fast_config(3));
        let options = LlmOptions {
            model: "gpt-4".into(),
            ..Default::default()
        };
        let result = wrapper.chat(vec![LlmMessage::user("hi")], &options).await;
        assert!(result.is_err());
        // Should stop after first failure — no retry for non-retryable errors.
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn exhausts_retries_and_returns_last_error() {
        let calls = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(FailThenSucceedProvider {
            fail_times: 10,
            calls: calls.clone(),
            error_msg: "status 503 Service Unavailable".into(),
        });
        let wrapper = ResilientLlmWrapper::new(provider).with_config(fast_config(2));
        let options = LlmOptions {
            model: "gpt-4".into(),
            ..Default::default()
        };
        let result = wrapper.chat(vec![LlmMessage::user("hi")], &options).await;
        assert!(result.is_err());
        // initial + 2 retries = 3 total calls
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn cost_tracker_records_on_success() {
        let calls = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(FailThenSucceedProvider {
            fail_times: 0,
            calls,
            error_msg: String::new(),
        });
        let tracker = CostTracker::new();
        let wrapper = ResilientLlmWrapper::new(provider)
            .with_config(fast_config(3))
            .with_cost_tracker(tracker.clone());
        let options = LlmOptions {
            model: "gpt-4o".into(),
            ..Default::default()
        };
        wrapper
            .chat(vec![LlmMessage::user("hi")], &options)
            .await
            .unwrap();
        assert_eq!(tracker.request_count(), 1);
        assert_eq!(tracker.total_tokens(), 15);
    }

    #[test]
    fn backoff_duration_doubles_and_caps() {
        let wrapper = ResilientLlmWrapper::new(Arc::new(FailThenSucceedProvider {
            fail_times: 0,
            calls: Arc::new(AtomicU32::new(0)),
            error_msg: String::new(),
        }));
        // Default: base=1000, max=8000
        assert_eq!(wrapper.backoff_duration(0).as_millis(), 1000);
        assert_eq!(wrapper.backoff_duration(1).as_millis(), 2000);
        assert_eq!(wrapper.backoff_duration(2).as_millis(), 4000);
        assert_eq!(wrapper.backoff_duration(3).as_millis(), 8000);
        // Capped at max.
        assert_eq!(wrapper.backoff_duration(10).as_millis(), 8000);
    }

    #[tokio::test]
    async fn budget_exceeded_blocks_call() {
        let calls = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(FailThenSucceedProvider {
            fail_times: 0,
            calls: calls.clone(),
            error_msg: String::new(),
        });
        let tracker = CostTracker::new();
        // Record enough cost to exceed a $0.01 budget
        tracker.record(
            "gpt-4o",
            TokenUsage {
                prompt_tokens: 10_000,
                completion_tokens: 5_000,
                total_tokens: 15_000,
            },
        );
        let config = ResilientConfig {
            max_budget_usd: Some(0.01),
            ..fast_config(3)
        };
        let wrapper = ResilientLlmWrapper::new(provider)
            .with_config(config)
            .with_cost_tracker(tracker.clone());
        let options = LlmOptions {
            model: "gpt-4o".into(),
            ..Default::default()
        };
        let result = wrapper.chat(vec![LlmMessage::user("hi")], &options).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Budget exceeded"));
        // Provider should never have been called
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn budget_ok_allows_call() {
        let calls = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(FailThenSucceedProvider {
            fail_times: 0,
            calls: calls.clone(),
            error_msg: String::new(),
        });
        let tracker = CostTracker::new();
        let config = ResilientConfig {
            max_budget_usd: Some(100.0),
            ..fast_config(3)
        };
        let wrapper = ResilientLlmWrapper::new(provider)
            .with_config(config)
            .with_cost_tracker(tracker);
        let options = LlmOptions {
            model: "gpt-4o".into(),
            ..Default::default()
        };
        let result = wrapper.chat(vec![LlmMessage::user("hi")], &options).await;
        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn name_delegates_to_inner() {
        let provider = Arc::new(FailThenSucceedProvider {
            fail_times: 0,
            calls: Arc::new(AtomicU32::new(0)),
            error_msg: String::new(),
        });
        let wrapper = ResilientLlmWrapper::new(provider);
        assert_eq!(wrapper.name(), "test-provider");
    }

    #[tokio::test]
    async fn test_fallback_succeeds_after_primary_fails() {
        // Primary model ("primary-model") always fails with a retryable 503.
        // Fallback model ("fallback-model") succeeds.
        let calls = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(ModelSelectiveProvider {
            fail_model: "primary-model".into(),
            calls: calls.clone(),
            error_msg: "status 503 Service Unavailable".into(),
        });
        let config = fast_config_with_fallbacks(1, vec!["fallback-model".into()]);
        let wrapper = ResilientLlmWrapper::new(provider).with_config(config);
        let options = LlmOptions {
            model: "primary-model".into(),
            ..Default::default()
        };
        let result = wrapper.chat(vec![LlmMessage::user("hi")], &options).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        // Response model should reflect the fallback model name.
        assert_eq!(response.model, "fallback-model");
        // Primary: 1 initial + 1 retry = 2 calls. Fallback: 1 call. Total = 3.
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_fallback_chain_exhausted() {
        // All models fail — primary and both fallbacks.
        let calls = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(AlwaysFailProvider {
            calls: calls.clone(),
            error_msg: "status 500 Internal Server Error".into(),
        });
        let config = fast_config_with_fallbacks(1, vec!["fallback-a".into(), "fallback-b".into()]);
        let wrapper = ResilientLlmWrapper::new(provider).with_config(config);
        let options = LlmOptions {
            model: "primary-model".into(),
            ..Default::default()
        };
        let result = wrapper.chat(vec![LlmMessage::user("hi")], &options).await;
        assert!(result.is_err());
        // primary: 2 calls (1 + 1 retry), fallback-a: 2, fallback-b: 2 = 6 total
        assert_eq!(calls.load(Ordering::SeqCst), 6);
    }

    #[tokio::test]
    async fn test_no_fallback_when_primary_succeeds() {
        // Primary succeeds on first try — fallback should never be attempted.
        let calls = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(ModelSelectiveProvider {
            fail_model: "fallback-model".into(), // fallback would fail if called
            calls: calls.clone(),
            error_msg: "status 503 Service Unavailable".into(),
        });
        let config = fast_config_with_fallbacks(2, vec!["fallback-model".into()]);
        let wrapper = ResilientLlmWrapper::new(provider).with_config(config);
        let options = LlmOptions {
            model: "primary-model".into(),
            ..Default::default()
        };
        let result = wrapper.chat(vec![LlmMessage::user("hi")], &options).await;
        assert!(result.is_ok());
        // Only 1 call: primary succeeded immediately.
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_non_retryable_error_skips_fallback() {
        // Non-retryable errors (e.g. auth failures) should not trigger fallback.
        let calls = Arc::new(AtomicU32::new(0));
        let provider = Arc::new(AlwaysFailProvider {
            calls: calls.clone(),
            error_msg: "invalid api key".into(), // not retryable
        });
        let config = fast_config_with_fallbacks(3, vec!["fallback-model".into()]);
        let wrapper = ResilientLlmWrapper::new(provider).with_config(config);
        let options = LlmOptions {
            model: "primary-model".into(),
            ..Default::default()
        };
        let result = wrapper.chat(vec![LlmMessage::user("hi")], &options).await;
        assert!(result.is_err());
        // Non-retryable: returns immediately after 1 call, no fallback.
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
