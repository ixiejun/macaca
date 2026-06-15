//! Tracing utilities for agent framework observability.
//!
//! Provides helper macros and functions for structured tracing spans
//! at agent, model, and tool levels.

/// Create a tracing span for an agent reply operation.
#[macro_export]
macro_rules! trace_agent_reply {
    ($agent_name:expr, $agent_id:expr) => {
        tracing::info_span!(
            "agent.reply",
            agent.id = %$agent_id,
            agent_name_present = !$agent_name.is_empty(),
            otel.kind = "internal",
        )
    };
}

/// Create a tracing span for a model chat call.
#[macro_export]
macro_rules! trace_model_chat {
    ($model_name:expr) => {
        tracing::info_span!(
            "model.chat",
            service_id = "llm",
            command = "chat",
            model_hint_present = !$model_name.is_empty(),
            otel.kind = "client",
        )
    };
}

/// Create a tracing span for a tool execution.
#[macro_export]
macro_rules! trace_tool_exec {
    ($tool_name:expr) => {
        tracing::info_span!(
            "tool.execute",
            tool.name = %$tool_name,
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
        // Verify that all macros compile and produce valid spans.
        let _span = trace_agent_reply!("my-agent", "agent-123");
        let _span = trace_model_chat!("qwen3-max");
        let _span = trace_tool_exec!("search");
        // record_usage works on current span (no-op outside a span context).
        record_usage(100, 50);
    }
}
