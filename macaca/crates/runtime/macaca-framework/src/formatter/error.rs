//! Formatter error taxonomy for wire-format conversion failures.

/// Errors produced during message formatting or response parsing.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FormatterError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Unsupported content type: {0}")]
    Unsupported(String),
}
