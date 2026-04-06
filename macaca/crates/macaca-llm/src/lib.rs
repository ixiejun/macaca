//! `aos-llm` — LLM abstraction layer with cost tracking and rate limiting.
//!
//! Supports OpenAI, Anthropic, DashScope (阿里百炼), and any OpenAI-compatible API.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use macaca_llm::{LlmRouter, OpenAiProvider, DashScopeProvider, OpenAiCompatibleProvider, CostTracker, RateLimiter};
//! use macaca_proto::types::{LlmMessage, LlmOptions, LlmRole};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut router = LlmRouter::new();
//!     router.register("openai", Arc::new(OpenAiProvider::from_env().unwrap()));
//!     router.register("dashscope", Arc::new(DashScopeProvider::from_env().unwrap()));
//!     // Any OpenAI-compatible API (vLLM, Ollama, DeepSeek, etc.)
//!     router.register("deepseek", Arc::new(
//!         OpenAiCompatibleProvider::from_env("deepseek", "https://api.deepseek.com/v1", "DEEPSEEK_API_KEY").unwrap()
//!     ));
//!
//!     let messages = vec![LlmMessage::user("Hello!")];
//!
//!     // Routes automatically: qwen-* → dashscope, gpt-* → openai, deepseek-* → deepseek
//!     let options = LlmOptions { model: "qwen-turbo".into(), ..Default::default() };
//!     let resp = router.chat(messages, &options).await.unwrap();
//! }
//! ```

pub mod provider;
pub mod openai;
pub mod anthropic;
pub mod dashscope;
pub mod openai_compatible;
pub mod router;
pub mod cost;
pub mod rate_limit;
pub mod resilient;

pub use provider::LlmProvider;
pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;
pub use dashscope::DashScopeProvider;
pub use openai_compatible::OpenAiCompatibleProvider;
pub use router::LlmRouter;
pub use cost::{CostTracker, ModelPricing, default_pricing};
pub use rate_limit::RateLimiter;
pub use resilient::{ResilientConfig, ResilientLlmWrapper};
