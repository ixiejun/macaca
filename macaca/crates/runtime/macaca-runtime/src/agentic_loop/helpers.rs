//! Shared helpers for usage accounting and tool-definition projection.

use macaca_proto::TokenUsage;

/// Accumulate token usage across iterations.
pub(super) fn accumulate_usage(total: &mut TokenUsage, delta: &TokenUsage) {
    total.prompt_tokens += delta.prompt_tokens;
    total.completion_tokens += delta.completion_tokens;
    total.total_tokens += delta.total_tokens;
}

pub(super) fn tool_definitions(
    tools: &dyn macaca_tools::ToolCatalog,
) -> Option<Vec<macaca_proto::ToolDefinition>> {
    let defs = macaca_tools::ToolCatalog::definitions(tools);
    if defs.is_empty() {
        None
    } else {
        Some(defs)
    }
}
