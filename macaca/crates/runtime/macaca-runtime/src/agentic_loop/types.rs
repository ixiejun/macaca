//! Core types for the agentic execution loop (P5 iteration 89 split).
//!
//! [`RuntimeConfig`] holds iteration limits and context-engine selection.
//! [`LoopResult`] is the terminal outcome of a loop run.
//! [`AgenticLoop`] orchestrates LLM ↔ tool round-trips via sibling modules.

use std::time::Duration;

use macaca_context::ContextBudget;
use macaca_proto::{config::ContextConfig, LlmMessage, TokenUsage};

/// Configuration for the agentic runtime loop.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum number of LLM round-trips before forcing a stop.
    pub max_iterations: usize,
    /// Timeout for a single tool execution.
    pub tool_timeout: Duration,
    /// Selected runtime context engine.
    pub context_engine: String,
    /// Fallback context engine if the selected engine fails.
    pub context_fallback_engine: String,
    /// Provider-neutral context budget.
    pub context_budget: ContextBudget,
    /// Merged [`ContextConfig`] slice — when default, catalog assembly yields only neutral skips.
    pub context: ContextConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            tool_timeout: Duration::from_secs(60),
            context_engine: "passthrough".into(),
            context_fallback_engine: "passthrough".into(),
            context_budget: ContextBudget::default(),
            context: ContextConfig::default(),
        }
    }
}

/// The result of an agentic loop execution.
#[derive(Debug, Clone)]
pub struct LoopResult {
    /// The final text response from the LLM.
    pub content: String,
    /// Total token usage across all iterations.
    pub total_usage: TokenUsage,
    /// Number of LLM round-trips performed.
    pub iterations: usize,
    /// The complete conversation history.
    pub messages: Vec<LlmMessage>,
}

/// The agentic execution loop.
///
/// Drives the LLM → tool → LLM cycle until the model produces a final
/// response or the iteration limit is reached.
pub struct AgenticLoop {
    pub(crate) config: RuntimeConfig,
}

impl AgenticLoop {
    /// Build one loop instance with fixed runtime policy.
    pub fn new(config: RuntimeConfig) -> Self {
        Self { config }
    }
}
