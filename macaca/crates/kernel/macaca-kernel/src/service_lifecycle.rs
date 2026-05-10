//! Lifecycle validation for system services.
//!
//! The lifecycle controller protects service state transitions without knowing
//! anything about provider internals.  This keeps the kernel responsible for
//! system invariants while concrete services remain replaceable.

use macaca_proto::{ServiceError, ServiceLifecycleState, ServiceResult};

/// Validate and record system service lifecycle transitions.
pub trait ServiceLifecycleController: Send + Sync {
    /// Return the current lifecycle state.
    fn state(&self) -> ServiceLifecycleState;

    /// Move to a new lifecycle state if the transition is valid.
    fn transition_to(&mut self, next: ServiceLifecycleState) -> ServiceResult<()>;
}

/// Minimal lifecycle state machine shared by service contract tests.
///
/// The state machine is intentionally explicit.  Adding a new transition in a
/// later phase requires changing this one validator rather than scattering
/// lifecycle checks across provider adapters.
#[derive(Debug, Clone)]
pub struct DefaultServiceLifecycleController {
    state: ServiceLifecycleState,
}

impl Default for DefaultServiceLifecycleController {
    fn default() -> Self {
        Self {
            state: ServiceLifecycleState::Installed,
        }
    }
}

impl DefaultServiceLifecycleController {
    /// Create a lifecycle controller starting at `Installed`.
    pub fn new() -> Self {
        Self::default()
    }

    fn is_valid_transition(from: &ServiceLifecycleState, to: &ServiceLifecycleState) -> bool {
        use ServiceLifecycleState::*;
        matches!(
            (from, to),
            (Installed, Registered)
                | (Registered, Authorized)
                | (Authorized, Starting)
                | (Starting, Running)
                | (Running, Calling)
                | (Calling, Running)
                | (Running, Stopping)
                | (Stopping, Stopped)
                | (Stopped, CleaningUp)
                | (CleaningUp, CleanedUp)
                | (_, Failed { .. })
        )
    }
}

impl ServiceLifecycleController for DefaultServiceLifecycleController {
    fn state(&self) -> ServiceLifecycleState {
        self.state.clone()
    }

    fn transition_to(&mut self, next: ServiceLifecycleState) -> ServiceResult<()> {
        if Self::is_valid_transition(&self.state, &next) {
            tracing::info!(
                from = ?self.state,
                to = ?next,
                "system service lifecycle transition"
            );
            self.state = next;
            Ok(())
        } else {
            tracing::warn!(
                from = ?self.state,
                to = ?next,
                "system service lifecycle transition rejected"
            );
            Err(ServiceError::InvalidLifecycleTransition {
                from: self.state.clone(),
                to: next,
            })
        }
    }
}
