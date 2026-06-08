//! Windowed context engine — budget-driven message trimming (**Strategy**).
//!
//! Enforces a simple context-window policy: preserve the leading system prompt,
//! keep a configurable tail of recent turns, and record trim decisions in the
//! assembly report for downstream audit.

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use tracing::info;

use crate::report::ContextDecisionReport;

use super::helpers::{
    build_report_for_messages, estimate_messages_tokens, trim_to_budget,
};
use super::types::{
    ContextAssembleInput, ContextAssembleResult, ContextEngine, ContextEngineInfo,
    ContextOptionsPatch,
};

/// Engine that enforces a simple context-window trim policy.
///
/// It keeps a leading system prompt when present, preserves a configurable
/// number of recent messages, and inserts a synthetic note describing that the
/// omitted middle section was trimmed for budget reasons.
#[derive(Debug, Clone)]
pub struct WindowedContextEngine {
    preserve_recent: usize,
}

impl Default for WindowedContextEngine {
    fn default() -> Self {
        Self {
            preserve_recent: 10,
        }
    }
}

impl WindowedContextEngine {
    pub const ID: &'static str = "windowed";
}

#[async_trait]
impl ContextEngine for WindowedContextEngine {
    fn info(&self) -> ContextEngineInfo {
        ContextEngineInfo::new(Self::ID, "Windowed Context Engine")
    }

    async fn assemble(&self, input: ContextAssembleInput) -> MacacaResult<ContextAssembleResult> {
        let original_len = input.base_messages.len();
        let original_tokens = estimate_messages_tokens(&input.base_messages);
        let messages = trim_to_budget(
            input.base_messages.clone(),
            input.budget,
            self.preserve_recent,
        );
        let trimmed_tokens = estimate_messages_tokens(&messages);
        let mut report = build_report_for_messages(Self::ID, &input, &messages);
        if messages.len() < original_len || trimmed_tokens < original_tokens {
            report.pruned_tokens = original_tokens.saturating_sub(trimmed_tokens);
            report.decisions.push(ContextDecisionReport::info(
                "context_window_trimmed",
                format!(
                    "Windowed engine reduced messages from {original_len} to {}.",
                    messages.len()
                ),
            ));
            info!(
                engine_id = Self::ID,
                original_messages = original_len,
                trimmed_messages = messages.len(),
                pruned_tokens = report.pruned_tokens,
                "context engine applied window trim for budget"
            );
        }
        Ok(ContextAssembleResult {
            messages,
            options: input.options,
            options_patch: ContextOptionsPatch::default(),
            report,
        })
    }
}
