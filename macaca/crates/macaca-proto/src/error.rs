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
