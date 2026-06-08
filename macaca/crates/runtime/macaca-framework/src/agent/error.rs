//! Agent execution error taxonomy.
//!
//! **Design pattern:** Value Object — structured error variants for framework
//! agents without embedding application-specific failure semantics.

/// Errors surfaced by agent `reply`, `observe`, or hook execution.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Tool error: {0}")]
    Tool(String),
    #[error("Interrupted")]
    Interrupted,
    #[error("Max iterations reached: {0}")]
    MaxIterations(usize),
    #[error("{0}")]
    Other(String),
}

/// Convenience result alias for agent and hook operations.
pub type AgentResult<T> = Result<T, AgentError>;
