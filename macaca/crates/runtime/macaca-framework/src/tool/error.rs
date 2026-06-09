//! Tool lookup and execution error taxonomy.
//!
//! [`ToolError`] values are provider-neutral: they describe *why* a toolkit operation
//! failed without embedding application names, workflow ids, or driver identifiers.
//! Middleware and handlers propagate these variants so audit traces can classify failures
//! by `reason_code`-like semantics (`not_found`, `invalid_args`, etc.).

/// Errors that can occur during tool lookup or execution.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ToolError {
    /// The requested tool name is not registered in the toolkit registry.
    #[error("Tool not found: {0}")]
    NotFound(String),

    /// Caller-supplied arguments failed validation inside the handler.
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    /// Handler or middleware reported a runtime execution failure.
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// The tool exists but its group is inactive (policy gate).
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Execution exceeded the configured timeout budget.
    #[error("Timeout")]
    Timeout,
}
