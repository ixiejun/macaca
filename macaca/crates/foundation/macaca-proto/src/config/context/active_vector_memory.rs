//! Active vector memory prefetch tunables for the context composer.

use serde::{Deserialize, Serialize};

/// Tunables for composer-stage **active vector memory** recall (`MemoryActiveRecallContextProvider`).
///
/// This controls whether long-term memory is prefetched through [`macaca_context::ActiveRecallCapability`]
/// before each model call. It is intentionally separate from tool-exposed memory APIs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveVectorMemoryContextConfig {
    /// Master switch: when false, no composer recall runs (explicit memory tools remain governed elsewhere).
    #[serde(default)]
    pub enabled: bool,
    /// Include hits tagged for the **current agent** private route when routing metadata exists.
    #[serde(default = "default_avm_include_agent_private")]
    pub include_agent_private: bool,
    /// Include session-scoped shared hits (typically entries without a competing agent owner).
    #[serde(default = "default_avm_include_session_shared")]
    pub include_session_shared: bool,
    /// Hard limit around the `ActiveRecallCapability::prefetch` async call (fail-open on expiry).
    #[serde(default = "default_avm_timeout_ms")]
    pub timeout_ms: u64,
    /// Passed through [`macaca_context::MemoryRecallQuery::max_tokens`] as a soft ceiling for providers.
    #[serde(default = "default_avm_max_query_tokens")]
    pub max_query_tokens: u32,
    /// Budget knobs mapped into [`macaca_context::ActiveRecallPolicy`].
    #[serde(default = "default_avm_max_hits")]
    pub max_hits: usize,
    #[serde(default = "default_avm_max_chars")]
    pub max_chars: usize,
    #[serde(default = "default_avm_max_tokens_budget")]
    pub max_tokens: u32,
}

fn default_avm_include_agent_private() -> bool {
    true
}

fn default_avm_include_session_shared() -> bool {
    true
}

fn default_avm_timeout_ms() -> u64 {
    1_500
}

fn default_avm_max_query_tokens() -> u32 {
    2_048
}

fn default_avm_max_hits() -> usize {
    8
}

fn default_avm_max_chars() -> usize {
    4_000
}

fn default_avm_max_tokens_budget() -> u32 {
    1_000
}

impl Default for ActiveVectorMemoryContextConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            include_agent_private: default_avm_include_agent_private(),
            include_session_shared: default_avm_include_session_shared(),
            timeout_ms: default_avm_timeout_ms(),
            max_query_tokens: default_avm_max_query_tokens(),
            max_hits: default_avm_max_hits(),
            max_chars: default_avm_max_chars(),
            max_tokens: default_avm_max_tokens_budget(),
        }
    }
}
