//! Memory error taxonomy for long-term storage and compression failures.

// ---------------------------------------------------------------------------
// MemoryError
// ---------------------------------------------------------------------------

/// Errors returned by `LongTermMemory` operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MemoryError {
    /// Failure while persisting messages.
    #[error("Storage error: {0}")]
    Storage(String),

    /// Failure while retrieving messages.
    #[error("Retrieval error: {0}")]
    Retrieval(String),
}

// ---------------------------------------------------------------------------
// CompressError
// ---------------------------------------------------------------------------

/// Errors returned by `MemoryCompressor` operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CompressError {
    /// Error from the model during summary generation.
    #[error("Model error: {0}")]
    Model(String),
    /// Error during message formatting or response parsing.
    #[error("Format error: {0}")]
    Format(String),
}
