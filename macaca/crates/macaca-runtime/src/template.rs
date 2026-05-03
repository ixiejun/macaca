//! Template primitives for runtime loop execution.

/// Result of a single iteration of the agentic loop.
pub(crate) enum RuntimeIterationOutcome {
    /// Tools were executed; caller should continue the loop.
    ToolsExecuted,
    /// LLM produced a final response (no tool calls).
    FinalResponse { content: String },
}
