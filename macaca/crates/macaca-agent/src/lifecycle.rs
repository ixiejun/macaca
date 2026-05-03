//! Agent lifecycle policy primitives.

use macaca_proto::{AgentState, MacacaError, MacacaResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentTransitionReason {
    Start,
    Suspend,
    Resume,
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentLifecycleTransition {
    pub from: AgentState,
    pub to: AgentState,
    pub reason: AgentTransitionReason,
}

impl AgentLifecycleTransition {
    pub fn new(from: AgentState, to: AgentState) -> MacacaResult<Self> {
        Ok(Self {
            from,
            to,
            reason: transition_reason(from, to)?,
        })
    }
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

pub fn transition_reason(from: AgentState, to: AgentState) -> MacacaResult<AgentTransitionReason> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_transition_rejects_invalid_pair() {
        let transition = AgentLifecycleTransition::new(AgentState::Created, AgentState::Suspended);

        assert!(transition.is_err());
    }

    #[test]
    fn default_policy_accepts_valid_transition_value() {
        let transition =
            AgentLifecycleTransition::new(AgentState::Created, AgentState::Running).unwrap();
        let policy = DefaultAgentLifecyclePolicy;

        assert!(policy.can_transition(transition.from, transition.to, transition.reason));
    }
}
