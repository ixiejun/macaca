//! Shared report builders and message-budget utilities for all context engines.
//!
//! Centralizing token accounting, prompt hashing, and source categorization here
//! keeps every **Strategy** implementation consistent and auditable. Engines call
//! these helpers rather than duplicating report-shaping logic.

use macaca_proto::{LlmMessage, LlmRole};

use crate::budget::ContextBudget;
use crate::estimate::estimate_text_tokens;
use crate::prompt::{PromptComposer, PromptSection, PromptStability, TrustLevel};
use crate::report::{
    ContextDecisionReport, ContextReport, ContextReportBuilder, ContextSourceKind,
    ContextSourceReport,
};

use super::types::{ContextAssembleInput, LEGACY_ENGINE_ID};

/// Build the compatibility report used by the passthrough engine.
pub(crate) fn build_legacy_report(input: &ContextAssembleInput) -> ContextReport {
    let mut report = build_report_for_messages(LEGACY_ENGINE_ID, input, &input.base_messages);
    report.decisions.push(ContextDecisionReport::info(
        "legacy_passthrough",
        "Legacy engine preserved incoming messages and options.",
    ));
    report
}

/// Convert a concrete message slice into a normalized `ContextReport`.
///
/// Multiple engines share this function so token accounting, prompt hashing,
/// and source categorization remain consistent across strategies.
pub(crate) fn build_report_for_messages(
    engine_id: &str,
    input: &ContextAssembleInput,
    messages: &[LlmMessage],
) -> ContextReport {
    let mut builder = ContextReportBuilder::new(LEGACY_ENGINE_ID)
        .identity(
            input.app_id,
            input.session_id.clone(),
            input.agent_name.clone(),
            input.model.clone(),
        )
        .budget(input.budget)
        .engine(engine_id);

    let mut composer = PromptComposer::new();
    for (idx, message) in messages.iter().enumerate() {
        let kind = message_source_kind(message);
        let source = ContextSourceReport::included(
            format!("message/{idx}"),
            kind.clone(),
            format!("{:?}", message.role),
            estimate_text_tokens(&message.content),
            message.content.len(),
        );
        builder = builder.source(source);

        if matches!(
            kind,
            ContextSourceKind::SystemPrompt | ContextSourceKind::DynamicPrompt
        ) {
            let stability = if kind == ContextSourceKind::SystemPrompt {
                PromptStability::Stable
            } else {
                PromptStability::Dynamic
            };
            composer = composer.push_section(PromptSection {
                id: format!("message/{idx}"),
                kind,
                stability,
                trust_level: TrustLevel::Trusted,
                content: message.content.clone(),
            });
        }
    }

    if let Some(tools) = input.options.tools.as_ref() {
        let bytes = serde_json::to_vec(tools).map(|v| v.len()).unwrap_or(0);
        let token_estimate = serde_json::to_string(tools)
            .map(|text| estimate_text_tokens(&text))
            .unwrap_or(0);
        builder = builder.source(ContextSourceReport::included(
            "tool_schema",
            ContextSourceKind::ToolSchema,
            "LLM tool schema",
            token_estimate,
            bytes,
        ));
    }

    let compiled = composer.compile();
    builder
        .hashes(compiled.stable_hash, compiled.full_hash)
        .build()
}

/// Estimate tokens for a whole message slice using the shared text estimator.
pub(crate) fn estimate_messages_tokens(messages: &[LlmMessage]) -> u32 {
    messages
        .iter()
        .map(|message| estimate_text_tokens(&message.content))
        .sum()
}

/// Trim a message slice to budget while preserving recent context.
///
/// Keeps a leading system prompt when present, retains the tail of recent
/// messages, and inserts a synthetic system note describing omitted history.
pub(crate) fn trim_to_budget(
    messages: Vec<LlmMessage>,
    budget: ContextBudget,
    preserve_recent: usize,
) -> Vec<LlmMessage> {
    if estimate_messages_tokens(&messages) <= budget.input_budget() || messages.len() <= 3 {
        return messages;
    }
    let mut output = Vec::new();
    let start_idx = if messages
        .first()
        .map(|message| message.role == LlmRole::System)
        .unwrap_or(false)
    {
        output.push(messages[0].clone());
        1
    } else {
        0
    };
    let recent = preserve_recent.min(messages.len().saturating_sub(start_idx));
    let recent_start = messages.len().saturating_sub(recent);
    if recent_start > start_idx {
        output.push(LlmMessage::system(format!(
            "[{} earlier messages trimmed to fit the context window.]",
            recent_start - start_idx
        )));
    }
    output.extend(messages[recent_start..].iter().cloned());
    output
}

/// Best-effort mapping from message role to report source category.
pub(crate) fn message_source_kind(message: &LlmMessage) -> ContextSourceKind {
    match message.role {
        LlmRole::System => ContextSourceKind::SystemPrompt,
        LlmRole::Tool => ContextSourceKind::ToolResult,
        LlmRole::User | LlmRole::Assistant => ContextSourceKind::History,
    }
}
