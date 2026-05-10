//! Optional bounded preflight recall contracts.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::memory::MemoryRecallQuery;
use crate::report::{ContextDecisionReport, ContextDecisionSeverity};
use crate::source::ContextSnippet;

/// Configuration for bounded recall that runs before normal context assembly completes.
///
/// Preflight recall is intentionally opt-in because it changes the prompt by
/// injecting additional memory before the model call. The config keeps the
/// feature bounded by tool allowlist, timeout, and token/character budgets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextPreflightRecallConfig {
    pub enabled: bool,
    pub allowed_tool_names: Vec<String>,
    pub timeout_ms: u64,
    pub max_chars: usize,
    pub max_tokens: u32,
    pub fatal_on_failure: bool,
}

impl Default for ContextPreflightRecallConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_tool_names: Vec::new(),
            timeout_ms: 1_500,
            max_chars: 4_000,
            max_tokens: 1_000,
            fatal_on_failure: false,
        }
    }
}

impl ContextPreflightRecallConfig {
    /// Convert the configured timeout into a runtime `Duration`.
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    /// Check whether a tool name is explicitly allowed for preflight recall.
    pub fn allows_tool(&self, tool_name: &str) -> bool {
        self.allowed_tool_names
            .iter()
            .any(|allowed| allowed == tool_name)
    }
}

/// Request envelope for one preflight recall pass.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextPreflightRecallInput {
    pub query: MemoryRecallQuery,
    pub config: ContextPreflightRecallConfig,
}

/// Result of a preflight recall attempt.
///
/// The output mirrors the reporting model used by the engines themselves:
/// snippets are returned separately from explanatory decisions so callers can
/// degrade gracefully without losing observability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextPreflightRecallOutput {
    pub snippets: Vec<ContextSnippet>,
    pub decisions: Vec<ContextDecisionReport>,
}

impl ContextPreflightRecallOutput {
    /// Standard "feature disabled" output used when preflight recall is off.
    pub fn empty_disabled() -> Self {
        Self {
            snippets: Vec::new(),
            decisions: vec![ContextDecisionReport::info(
                "preflight_recall_disabled",
                "Preflight recall is disabled by configuration.",
            )],
        }
    }

    /// Standard warning output used when preflight recall degrades non-fatally.
    pub fn degraded_warning(message: impl Into<String>) -> Self {
        Self {
            snippets: Vec::new(),
            decisions: vec![ContextDecisionReport {
                code: "preflight_recall_degraded".into(),
                severity: ContextDecisionSeverity::Warning,
                message: message.into(),
            }],
        }
    }
}

/// Conservative heuristic for identifying read-only recall tools.
///
/// This helper is deliberately biased toward false negatives rather than false
/// positives: a tool should only be auto-classified as safe for preflight if
/// its name clearly implies read/search semantics.
pub fn read_only_recall_tool_name(tool_name: &str) -> bool {
    let lower = tool_name.to_ascii_lowercase();
    lower.contains("recall")
        || lower.contains("search")
        || lower.contains("get")
        || lower.contains("read")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_recall_is_disabled_by_default() {
        let config = ContextPreflightRecallConfig::default();
        assert!(!config.enabled);
        assert!(!config.fatal_on_failure);
    }

    #[test]
    fn allowlist_controls_tools() {
        let config = ContextPreflightRecallConfig {
            allowed_tool_names: vec!["memory_search".into()],
            ..Default::default()
        };
        assert!(config.allows_tool("memory_search"));
        assert!(!config.allows_tool("shell"));
    }

    #[test]
    fn read_only_recall_tool_names_are_classified_conservatively() {
        assert!(read_only_recall_tool_name("memory_search"));
        assert!(read_only_recall_tool_name("memory_get"));
        assert!(!read_only_recall_tool_name("write_file"));
        assert!(!read_only_recall_tool_name("shell_execute"));
    }
}
