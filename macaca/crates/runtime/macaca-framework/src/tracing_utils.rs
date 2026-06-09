//! Tracing utilities for agent framework observability.
//!
//! Provides helper macros and functions for structured tracing spans at agent,
//! model, and tool levels. Span fields use provider-neutral stable identifiers
//! (`agent.id`, `service_id`, `tool.id`) so OS logs remain application-agnostic
//! and align with audit replay dimensions.

/// Create a tracing span for an agent reply operation.
///
/// Uses `agent.id` as the stable correlation key; persona display names must not
/// appear in OS-layer spans because they encode application-specific roles.
#[macro_export]
macro_rules! trace_agent_reply {
    ($agent_id:expr) => {
        tracing::info_span!(
            "agent.reply",
            agent.id = %$agent_id,
            service_id = "framework.agent",
            otel.kind = "internal",
        )
    };
}

/// Create a tracing span for a model chat call routed through the LLM service.
///
/// Model/vendor labels are resolved inside `macaca-llm`; framework spans only
/// record the neutral service command boundary.
#[macro_export]
macro_rules! trace_model_chat {
    () => {
        tracing::info_span!(
            "model.chat",
            service_id = "llm",
            command = "chat",
            otel.kind = "client",
        )
    };
}

/// Create a tracing span for a tool execution.
///
/// Tool identity is expressed as `tool.id` (stable catalog key), not a free-form
/// display label that may embed application vocabulary.
#[macro_export]
macro_rules! trace_tool_exec {
    ($tool_id:expr) => {
        tracing::info_span!(
            "tool.execute",
            tool.id = %$tool_id,
            service_id = "tool",
            otel.kind = "internal",
        )
    };
}

/// Record token usage on the current span.
pub fn record_usage(input_tokens: u32, output_tokens: u32) {
    tracing::Span::current().record("llm.input_tokens", input_tokens);
    tracing::Span::current().record("llm.output_tokens", output_tokens);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_macros_compile() {
        // Verify that all macros compile and produce valid provider-neutral spans.
        let _span = trace_agent_reply!("agent-123");
        let _span = trace_model_chat!();
        let _span = trace_tool_exec!("search");
        // record_usage works on current span (no-op outside a span context).
        record_usage(100, 50);
    }
}
