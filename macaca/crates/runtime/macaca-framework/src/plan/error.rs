//! Plan error taxonomy for state-machine transition failures.

use super::types::SubTaskState;

// PlanError
// ---------------------------------------------------------------------------

/// Errors returned by `Plan` and `PlanNotebook` operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PlanError {
    /// Another subtask is already `InProgress`; finish it before starting another.
    #[error("Another subtask is already in progress")]
    AnotherInProgress,

    /// The given subtask index does not exist.
    #[error("Subtask index out of bounds: {0}")]
    IndexOutOfBounds(usize),

    /// The requested state transition is not allowed.
    #[error("Invalid state transition: {from:?} → {to:?}")]
    InvalidTransition {
        from: SubTaskState,
        to: SubTaskState,
    },

    /// There is no active plan to operate on.
    #[error("No active plan")]
    NoPlan,

    /// The given historical plan index does not exist.
    #[error("Historical plan index out of bounds: {0}")]
    HistoricalIndexOutOfBounds(usize),
}

// ---------------------------------------------------------------------------
