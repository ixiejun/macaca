//! Agent state-machine: enforces valid lifecycle transitions.

use macaca_proto::{AgentState, MacacaError, MacacaResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentTransitionReason {
    Start,
    Suspend,
    Resume,
    Terminate,
}

pub trait AgentLifecyclePolicy: Send + Sync {
    fn can_transition(
        &self,
        from: AgentState,
        to: AgentState,
        reason: AgentTransitionReason,
    ) -> bool;
}

pub struct DefaultAgentLifecyclePolicy;

impl AgentLifecyclePolicy for DefaultAgentLifecyclePolicy {
    fn can_transition(
        &self,
        from: AgentState,
        to: AgentState,
        reason: AgentTransitionReason,
    ) -> bool {
        matches!(
            (from, to, reason),
            (
                AgentState::Created,
                AgentState::Running,
                AgentTransitionReason::Start
            ) | (
                AgentState::Running,
                AgentState::Suspended,
                AgentTransitionReason::Suspend
            ) | (
                AgentState::Running,
                AgentState::Terminated,
                AgentTransitionReason::Terminate
            ) | (
                AgentState::Suspended,
                AgentState::Running,
                AgentTransitionReason::Resume
            ) | (
                AgentState::Suspended,
                AgentState::Terminated,
                AgentTransitionReason::Terminate
            )
        )
    }
}

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
    pub fn new() -> Self {
        Self {
            state: AgentState::Created,
            policy: Box::new(DefaultAgentLifecyclePolicy),
        }
    }

    pub fn with_policy(policy: Box<dyn AgentLifecyclePolicy>) -> Self {
        Self {
            state: AgentState::Created,
            policy,
        }
    }

    /// Return the current state.
    pub fn state(&self) -> AgentState {
        self.state
    }

    /// Attempt to transition to `next`. Returns `Err` for invalid transitions.
    pub fn transition(&mut self, next: AgentState) -> MacacaResult<()> {
        let valid =
            self.policy
                .can_transition(self.state, next, transition_reason(self.state, next)?);

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

fn transition_reason(from: AgentState, to: AgentState) -> MacacaResult<AgentTransitionReason> {
    match (from, to) {
        (AgentState::Created, AgentState::Running) => Ok(AgentTransitionReason::Start),
        (AgentState::Running, AgentState::Suspended) => Ok(AgentTransitionReason::Suspend),
        (AgentState::Suspended, AgentState::Running) => Ok(AgentTransitionReason::Resume),
        (AgentState::Running, AgentState::Terminated)
        | (AgentState::Suspended, AgentState::Terminated) => Ok(AgentTransitionReason::Terminate),
        _ => Err(MacacaError::Agent(format!(
            "invalid state transition: {:?} -> {:?}",
            from, to
        ))),
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
            let actual = transition_reason(from, to)
                .map(|reason| policy.can_transition(from, to, reason))
                .unwrap_or(false);
            assert_eq!((from, to, actual), (from, to, expected));
        }
    }

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
