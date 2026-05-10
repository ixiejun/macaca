use thiserror::Error;

#[derive(Error, Debug)]
pub enum MacacaError {
    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Task error: {0}")]
    Task(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Persist error: {0}")]
    Persist(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Gateway error: {0}")]
    Gateway(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Driver error: {0}")]
    Driver(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type MacacaResult<T> = Result<T, MacacaError>;

/// Stable proto-layer adapter for extracting display metadata without
/// embedding transport or recovery policy into `macaca-proto`.
pub trait ProtoErrorAdapter {
    fn code(&self) -> &'static str;
    fn display_message(&self) -> String;
}

impl ProtoErrorAdapter for MacacaError {
    fn code(&self) -> &'static str {
        match self {
            MacacaError::Agent(_) => "agent_error",
            MacacaError::Task(_) => "task_error",
            MacacaError::Memory(_) => "memory_error",
            MacacaError::Ipc(_) => "ipc_error",
            MacacaError::Llm(_) => "llm_error",
            MacacaError::Persist(_) => "persist_error",
            MacacaError::Config(_) => "config_error",
            MacacaError::Gateway(_) => "gateway_error",
            MacacaError::PermissionDenied(_) => "permission_denied",
            MacacaError::NotFound(_) => "not_found",
            MacacaError::Timeout(_) => "timeout",
            MacacaError::BudgetExceeded(_) => "budget_exceeded",
            MacacaError::Serialization(_) => "serialization_error",
            MacacaError::Driver(_) => "driver_error",
            MacacaError::Io(_) => "io_error",
            MacacaError::Json(_) => "json_error",
        }
    }

    fn display_message(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macaca_error_adapter_preserves_display_and_code() {
        let err = MacacaError::Config("missing key".into());
        assert_eq!(err.code(), "config_error");
        assert_eq!(err.display_message(), "Config error: missing key");
    }

    #[test]
    fn macaca_error_adapter_keeps_original_error_type() {
        let err = MacacaError::Task("review failed".into());
        match err {
            MacacaError::Task(message) => assert_eq!(message, "review failed"),
            other => panic!("unexpected variant: {other:?}"),
        }
    }
}
