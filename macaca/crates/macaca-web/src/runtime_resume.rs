//! Web-local resume signals for coordinator and goal wait flows.

/// Generic resume signal used by web coordinator/session plumbing.
#[derive(Debug, Clone)]
pub enum RuntimeResumeSignal {
    /// Normal resume request.
    Manual,
    /// Resume due to delegated work or goal completion.
    DelegateCompleted {
        task_id: String,
        success: bool,
        output: String,
    },
    /// Resume due to delegated work failure.
    DelegateFailed { task_id: String, error: String },
    /// Resume due to timeout.
    Timeout,
}
