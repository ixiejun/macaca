//! MCP error taxonomy — protocol, transport, and registration failures.

/// Errors specific to MCP operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum McpError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Execution error: {0}")]
    Execution(String),
    #[error("Timeout")]
    Timeout,
    #[error("Not connected")]
    NotConnected,
    #[error("Already connected")]
    AlreadyConnected,
    #[error("IO error: {0}")]
    Io(String),
    #[error("Tool name collision: {0}")]
    ToolNameCollision(String),
    #[error("Unsupported transport: {0}")]
    UnsupportedTransport(String),
}
