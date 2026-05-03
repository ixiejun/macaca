//! Agent state-machine: enforces valid lifecycle transitions.

use macaca_proto::{AgentState, MacacaError, MacacaResult};

use crate::lifecycle::{
    AgentLifecyclePolicy, AgentLifecycleTransition, DefaultAgentLifecyclePolicy,
};

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
    policy: Box<dyn AgentLifecyclePolicy>,
}

impl AgentStateMachine {
    /// Create a new state machine starting in [`AgentState::Created`].
    #[deprecated(note = "use AgentStateMachine::default() for new code")]
    pub fn new() -> Self {
        Self {
            state: AgentState::Created,
            policy: Box::new(DefaultAgentLifecyclePolicy),
        }
    }

    #[deprecated(note = "use AgentStateMachine::with_lifecycle_policy(...) for new code")]
    pub fn with_policy(policy: Box<dyn AgentLifecyclePolicy>) -> Self {
        Self::with_lifecycle_policy(policy)
    }

    pub fn with_lifecycle_policy(policy: Box<dyn AgentLifecyclePolicy>) -> Self {
        Self {
            state: AgentState::Created,
            policy,
        }
    }

    /// Return the current state.
    pub fn state(&self) -> AgentState {
        self.state
    }

    /// Return whether the next transition would be accepted without mutating state.
    pub fn can_transition_to(&self, next: AgentState) -> bool {
        AgentLifecycleTransition::new(self.state, next)
            .map(|transition| {
                self.policy
                    .can_transition(transition.from, transition.to, transition.reason)
            })
            .unwrap_or(false)
    }

    /// Attempt to transition to `next`. Returns `Err` for invalid transitions.
    pub fn transition(&mut self, next: AgentState) -> MacacaResult<()> {
        let transition = AgentLifecycleTransition::new(self.state, next)?;
        let valid = self
            .policy
            .can_transition(transition.from, transition.to, transition.reason);

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
        Self {
            state: AgentState::Created,
            policy: Box::new(DefaultAgentLifecyclePolicy),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_matrix_matches_expected_rules() {
        let cases = [
            (AgentState::Created, AgentState::Running, true),
            (AgentState::Created, AgentState::Suspended, false),
            (AgentState::Created, AgentState::Terminated, false),
            (AgentState::Running, AgentState::Suspended, true),
            (AgentState::Running, AgentState::Running, false),
            (AgentState::Running, AgentState::Terminated, true),
            (AgentState::Suspended, AgentState::Running, true),
            (AgentState::Suspended, AgentState::Terminated, true),
            (AgentState::Suspended, AgentState::Created, false),
            (AgentState::Terminated, AgentState::Running, false),
        ];

        let policy = DefaultAgentLifecyclePolicy;
        for (from, to, expected) in cases {
            let actual = crate::lifecycle::transition_reason(from, to)
                .map(|reason| policy.can_transition(from, to, reason))
                .unwrap_or(false);
            assert_eq!((from, to, actual), (from, to, expected));
        }
    }

    #[test]
    fn created_to_running() {
        let mut sm = AgentStateMachine::default();
        assert!(sm.transition(AgentState::Running).is_ok());
        assert_eq!(sm.state(), AgentState::Running);
    }

    #[test]
    fn running_to_suspended() {
        let mut sm = AgentStateMachine::default();
        sm.transition(AgentState::Running).unwrap();
        assert!(sm.transition(AgentState::Suspended).is_ok());
    }

    #[test]
    fn suspended_to_running() {
        let mut sm = AgentStateMachine::default();
        sm.transition(AgentState::Running).unwrap();
        sm.transition(AgentState::Suspended).unwrap();
        assert!(sm.transition(AgentState::Running).is_ok());
    }

    #[test]
    fn running_to_terminated() {
        let mut sm = AgentStateMachine::default();
        sm.transition(AgentState::Running).unwrap();
        assert!(sm.transition(AgentState::Terminated).is_ok());
    }

    #[test]
    fn invalid_created_to_suspended() {
        let mut sm = AgentStateMachine::default();
        assert!(sm.transition(AgentState::Suspended).is_err());
    }

    #[test]
    fn invalid_created_to_terminated() {
        let mut sm = AgentStateMachine::default();
        assert!(sm.transition(AgentState::Terminated).is_err());
    }

    #[test]
    fn invalid_terminated_to_running() {
        let mut sm = AgentStateMachine::default();
        sm.transition(AgentState::Running).unwrap();
        sm.transition(AgentState::Terminated).unwrap();
        assert!(sm.transition(AgentState::Running).is_err());
    }

    #[test]
    fn can_transition_to_matches_transition_success_matrix() {
        let mut sm = AgentStateMachine::default();
        assert!(sm.can_transition_to(AgentState::Running));
        assert!(!sm.can_transition_to(AgentState::Suspended));

        sm.transition(AgentState::Running).unwrap();
        assert!(sm.can_transition_to(AgentState::Suspended));
        assert!(sm.can_transition_to(AgentState::Terminated));
        assert!(!sm.can_transition_to(AgentState::Created));
    }
}
