//! Chat session mediation boundary.

use std::sync::Arc;

use crate::state::AppState;

/// Additive facade that will absorb chat/session orchestration in later slices.
#[derive(Clone)]
pub struct ChatSessionMediator {
    state: Arc<AppState>,
}

impl ChatSessionMediator {
    /// Create a mediator for existing shared web state.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Return the underlying state for compatibility adapters.
    pub fn state(&self) -> Arc<AppState> {
        Arc::clone(&self.state)
    }
}
