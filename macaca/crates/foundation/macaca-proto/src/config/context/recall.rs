//! Context recall runtime: memory tools exposure and preflight recall tunables.

use serde::{Deserialize, Serialize};

/// Runtime toggles for memory tool exposure and preflight recall before model calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRecallRuntimeConfig {
    /// When true, the web runtime exposes read-only `memory_search` / `memory_get` tools.
    #[serde(default)]
    pub expose_memory_tools: bool,
    #[serde(default = "default_memory_search_limit")]
    pub memory_search_default_limit: u32,
    #[serde(default)]
    pub preflight_recall_enabled: bool,
    #[serde(default)]
    pub preflight_allowed_tools: Vec<String>,
    #[serde(default = "default_preflight_timeout_ms")]
    pub preflight_timeout_ms: u64,
    #[serde(default = "default_preflight_max_chars")]
    pub preflight_max_chars: usize,
    #[serde(default = "default_preflight_max_tokens")]
    pub preflight_max_tokens: u32,
    #[serde(default)]
    pub preflight_fatal_on_failure: bool,
}

fn default_memory_search_limit() -> u32 {
    8
}

fn default_preflight_timeout_ms() -> u64 {
    1_500
}

fn default_preflight_max_chars() -> usize {
    4_000
}

fn default_preflight_max_tokens() -> u32 {
    1_000
}

impl Default for ContextRecallRuntimeConfig {
    fn default() -> Self {
        Self {
            expose_memory_tools: false,
            memory_search_default_limit: default_memory_search_limit(),
            preflight_recall_enabled: false,
            preflight_allowed_tools: Vec::new(),
            preflight_timeout_ms: default_preflight_timeout_ms(),
            preflight_max_chars: default_preflight_max_chars(),
            preflight_max_tokens: default_preflight_max_tokens(),
            preflight_fatal_on_failure: false,
        }
    }
}
