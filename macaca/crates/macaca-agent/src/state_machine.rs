//! Agent state-machine: enforces valid lifecycle transitions.

use macaca_proto::{AgentState, MacacaError, MacacaResult};

/// Manages an agent's lifecycle state and validates transitions.
///
/// Valid transitions:
/// ```text
/// Created → Running
/// Running → Suspended
/// Running → Terminated
/// Suspended → Running
/// Suspended → Terminated
/// ```
pub struct AgentStateMachine {
    state: AgentState,
}

impl AgentStateMachine {
    /// Create a new state machine starting in [`AgentState::Created`].
    pub fn new() -> Self {
        Self {
            state: AgentState::Created,
        }
    }

    /// Return the current state.
    pub fn state(&self) -> AgentState {
        self.state
    }

    /// Attempt to transition to `next`. Returns `Err` for invalid transitions.
    pub fn transition(&mut self, next: AgentState) -> MacacaResult<()> {
        let valid = matches!(
            (self.state, next),
            (AgentState::Created, AgentState::Running)
                | (AgentState::Running, AgentState::Suspended)
                | (AgentState::Running, AgentState::Terminated)
                | (AgentState::Suspended, AgentState::Running)
                | (AgentState::Suspended, AgentState::Terminated)
        );

        if valid {
            self.state = next;
            Ok(())
        } else {
            Err(MacacaError::Agent(format!(
                "invalid state transition: {:?} -> {:?}",
                self.state, next
            )))
        }
    }
}

impl Default for AgentStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_to_running() {
        let mut sm = AgentStateMachine::new();
        assert!(sm.transition(AgentState::Running).is_ok());
        assert_eq!(sm.state(), AgentState::Running);
    }

    #[test]
    fn running_to_suspended() {
        let mut sm = AgentStateMachine::new();
        sm.transition(AgentState::Running).unwrap();
        assert!(sm.transition(AgentState::Suspended).is_ok());
    }

    #[test]
    fn suspended_to_running() {
        let mut sm = AgentStateMachine::new();
        sm.transition(AgentState::Running).unwrap();
        sm.transition(AgentState::Suspended).unwrap();
        assert!(sm.transition(AgentState::Running).is_ok());
    }

    #[test]
    fn running_to_terminated() {
        let mut sm = AgentStateMachine::new();
        sm.transition(AgentState::Running).unwrap();
        assert!(sm.transition(AgentState::Terminated).is_ok());
    }

    #[test]
    fn invalid_created_to_suspended() {
        let mut sm = AgentStateMachine::new();
        assert!(sm.transition(AgentState::Suspended).is_err());
    }

    #[test]
    fn invalid_created_to_terminated() {
        let mut sm = AgentStateMachine::new();
        assert!(sm.transition(AgentState::Terminated).is_err());
    }

    #[test]
    fn invalid_terminated_to_running() {
        let mut sm = AgentStateMachine::new();
        sm.transition(AgentState::Running).unwrap();
        sm.transition(AgentState::Terminated).unwrap();
        assert!(sm.transition(AgentState::Running).is_err());
    }
}
