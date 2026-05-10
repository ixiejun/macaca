//! Application ABI lifecycle state machine.
//!
//! The lifecycle module applies the State pattern to Application ABI v0.  The
//! state machine is intentionally small and explicit so host runtimes, YAML
//! adapters, future WASM runtimes, and audit tooling can agree on which
//! transitions are legal without copying ad-hoc conditionals across crates.

use macaca_proto::{
    ApplicationAbiError, ApplicationLifecycleState, ApplicationLifecycleTransition, TraceContext,
};
use tracing::{info, warn};

/// Deterministic lifecycle state machine for one application instance.
#[derive(Debug, Clone)]
pub struct ApplicationLifecycle {
    application_id: String,
    state: ApplicationLifecycleState,
}

impl ApplicationLifecycle {
    /// Start a lifecycle in the declared state.
    pub fn declared(application_id: impl Into<String>) -> Self {
        Self {
            application_id: application_id.into(),
            state: ApplicationLifecycleState::Declared,
        }
    }

    /// Return the current lifecycle state.
    pub fn state(&self) -> &ApplicationLifecycleState {
        &self.state
    }

    /// Transition to a new state after validating the state graph.
    ///
    /// The method emits logs for both accepted and rejected transitions.  The
    /// log payload contains only ids and state names; opaque application state
    /// stays outside logs to avoid leaking user data.
    pub fn transition_to(
        &mut self,
        to: ApplicationLifecycleState,
        trace: Option<TraceContext>,
    ) -> Result<ApplicationLifecycleTransition, ApplicationAbiError> {
        let from = self.state.clone();
        if !is_valid_transition(&from, &to) {
            warn!(
                application_id = %self.application_id,
                from = ?from,
                to = ?to,
                "application ABI lifecycle transition rejected"
            );
            return Err(ApplicationAbiError::InvalidLifecycleTransition { from, to });
        }

        info!(
            application_id = %self.application_id,
            from = ?from,
            to = ?to,
            "application ABI lifecycle transition accepted"
        );
        self.state = to.clone();
        Ok(ApplicationLifecycleTransition {
            application_id: self.application_id.clone(),
            from,
            to,
            trace,
            metadata: Default::default(),
        })
    }
}

/// Validate the ABI lifecycle graph.
///
/// Failure is always allowed from active states because failures represent an
/// audited terminal-ish outcome rather than business workflow behavior.
fn is_valid_transition(from: &ApplicationLifecycleState, to: &ApplicationLifecycleState) -> bool {
    use ApplicationLifecycleState::*;
    matches!(
        (from, to),
        (Declared, Initialized)
            | (Initialized, Started)
            | (Started, Paused)
            | (Paused, Resumed)
            | (Resumed, Started)
            | (Started, ShuttingDown)
            | (Paused, ShuttingDown)
            | (Resumed, ShuttingDown)
            | (Initialized, ShuttingDown)
            | (ShuttingDown, Stopped)
            | (Declared, Failed { .. })
            | (Initialized, Failed { .. })
            | (Started, Failed { .. })
            | (Paused, Failed { .. })
            | (Resumed, Failed { .. })
            | (ShuttingDown, Failed { .. })
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_abi_lifecycle_accepts_valid_transitions() {
        let mut lifecycle = ApplicationLifecycle::declared("app.lifecycle");

        lifecycle
            .transition_to(ApplicationLifecycleState::Initialized, None)
            .unwrap();
        lifecycle
            .transition_to(ApplicationLifecycleState::Started, None)
            .unwrap();
        lifecycle
            .transition_to(ApplicationLifecycleState::Paused, None)
            .unwrap();
        lifecycle
            .transition_to(ApplicationLifecycleState::Resumed, None)
            .unwrap();

        assert_eq!(lifecycle.state(), &ApplicationLifecycleState::Resumed);
    }

    #[test]
    fn application_abi_lifecycle_rejects_invalid_transitions() {
        let mut lifecycle = ApplicationLifecycle::declared("app.lifecycle");

        let error = lifecycle
            .transition_to(ApplicationLifecycleState::Started, None)
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationAbiError::InvalidLifecycleTransition { .. }
        ));
        assert_eq!(lifecycle.state(), &ApplicationLifecycleState::Declared);
    }
}
