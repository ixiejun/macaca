//! Contract tests for `ResilientLlmWrapper` retry, backoff, budget, and fallback behavior.
//!
//! Extracted from `resilient.rs` so the Decorator/Chain-of-Responsibility production
//! wrapper stays under the OS 500-line constitution.

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
