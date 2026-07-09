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

pub mod anthropic;
pub mod coding_plans;
pub mod cost;
pub mod dashscope;
pub mod error_sanitize;
pub mod hardening_contract;
pub mod openai;
pub mod openai_compatible;
pub mod provider;
pub mod rate_limit;
pub mod resilient;
pub mod resolver;
pub mod router;
pub mod service_adapter;
pub mod service_contract;
pub mod tool_wire;

pub use anthropic::AnthropicProvider;
pub use coding_plans::normalize_openai_compatible_base;
pub use cost::{default_pricing, CostTracker, ModelPricing};
pub use dashscope::DashScopeProvider;
pub use hardening_contract::{
    LlmBudgetStatusCommand, LlmBudgetStatusResult, LlmCatalogReadCommand,
    LlmContinuationValidateCommand, LlmContinuationValidateResult, LlmDegradationExplainCommand,
    LlmDegradationExplainResult, LlmDiagnostic, LlmHardenedServiceResponse, LlmModelCatalogEntry,
    LlmModelCatalogResult, LlmProviderCapabilitiesResult, LlmProviderCapabilityRecord,
    LlmProviderProtocolMetadata, LlmRetryPolicy, LlmRouteResolveCommand, LlmRouteResolveResult,
    LLM_BUDGET_STATUS_COMMAND, LLM_CONTINUATION_VALIDATE_COMMAND, LLM_DEGRADATION_EXPLAIN_COMMAND,
    LLM_MODEL_LIST_COMMAND, LLM_PROVIDER_CAPABILITIES_READ_COMMAND, LLM_ROUTE_RESOLVE_COMMAND,
};
pub use openai::OpenAiProvider;
pub use openai_compatible::OpenAiCompatibleProvider;
pub use provider::LlmProvider;
pub use rate_limit::RateLimiter;
pub use resilient::{ResilientConfig, ResilientLlmWrapper};
pub use resolver::{
    default_model_route_descriptors, ModelRouteDescriptor, ModelRouteMatcher,
    PrefixProviderResolver, ProviderResolver, ResolverChain,
};
pub use router::{LlmRouter, ModelSelection, ModelSelectionRequest, ModelTarget};
pub use service_adapter::llm_service_descriptor;
pub use service_contract::{
    LlmChatCommand, LlmChatResult, LlmModelSelectionCommand, LlmModelSelectionResult,
    LlmPolicyHints, LlmProviderInventoryItem, LlmRouteSummary, LlmServiceChatClient,
    LlmServiceEvent, LlmServiceScope, LlmServiceSnapshot, LlmServiceSnapshotCommand,
    LLM_CHAT_COMMAND, LLM_MODEL_SELECTION_COMMAND, LLM_SERVICE_ID, LLM_SNAPSHOT_COMMAND,
};
