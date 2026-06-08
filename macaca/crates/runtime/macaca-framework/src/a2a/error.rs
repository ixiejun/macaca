//! A2A protocol error taxonomy.
//!
//! Errors are categorized by failure domain (discovery, communication, protocol,
//! data validation) so callers can branch on error kind without parsing strings.

#[derive(Debug, Clone, thiserror::Error)]
pub enum A2AError {
    /// Failed to locate or fetch an agent card.
    #[error("Discovery error: {0}")]
    Discovery(String),
    /// Network or transport failure.
    #[error("Communication error: {0}")]
    Communication(String),
    /// Unexpected protocol violation.
    #[error("Protocol error: {0}")]
    Protocol(String),
    /// Data part has an unrecognised `type` / `kind` value.
    #[error("Invalid data type: {0}")]
    InvalidDataType(String),
    /// A required field is missing from a Data part.
    #[error("Missing required field: {0}")]
    MissingField(String),
    /// JSON deserialization failed.
    #[error("Deserialization error: {0}")]
    Deserialize(String),
}
