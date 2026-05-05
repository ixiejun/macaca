//! Optional bounded preflight recall contracts.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::memory::MemoryRecallQuery;
use crate::report::{ContextDecisionReport, ContextDecisionSeverity};
use crate::source::ContextSnippet;

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
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    pub fn allows_tool(&self, tool_name: &str) -> bool {
        self.allowed_tool_names
            .iter()
            .any(|allowed| allowed == tool_name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextPreflightRecallInput {
    pub query: MemoryRecallQuery,
    pub config: ContextPreflightRecallConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextPreflightRecallOutput {
    pub snippets: Vec<ContextSnippet>,
    pub decisions: Vec<ContextDecisionReport>,
}

impl ContextPreflightRecallOutput {
    pub fn empty_disabled() -> Self {
        Self {
            snippets: Vec::new(),
            decisions: vec![ContextDecisionReport::info(
                "preflight_recall_disabled",
                "Preflight recall is disabled by configuration.",
            )],
        }
    }

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
