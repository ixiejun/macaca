//! Agent runtime status transition policy.

use macaca_proto::{AgentActivity, AgentRuntimeStatus, AgentState};

/// Applies existing agent status transitions in one place.
pub struct AgentStatusTransitionPolicy;

impl AgentStatusTransitionPolicy {
    /// Apply a lifecycle state transition.
    pub fn apply_state(status: &mut AgentRuntimeStatus, state: AgentState) {
        status.state = state;
        status.updated_at = chrono::Utc::now();
    }

    /// Apply an activity transition.
    pub fn apply_activity(status: &mut AgentRuntimeStatus, activity: AgentActivity) {
        status.activity = activity;
        status.updated_at = chrono::Utc::now();
    }

    /// Apply the current idle behavior.
    pub fn apply_idle(status: &mut AgentRuntimeStatus) {
        status.activity = AgentActivity::Idle;
        status.current_task = None;
        status.updated_at = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{AgentId, AgentRuntimeStatus};

    fn status() -> AgentRuntimeStatus {
        AgentRuntimeStatus {
            agent_id: AgentId::new(),
            name: "agent".into(),
            state: AgentState::Created,
            activity: AgentActivity::Idle,
            updated_at: Utc::now(),
            current_task: Some("task".into()),
        }
    }

    #[test]
    fn idle_clears_current_task() {
        let mut status = status();
        AgentStatusTransitionPolicy::apply_idle(&mut status);
        assert!(matches!(status.activity, AgentActivity::Idle));
        assert_eq!(status.current_task, None);
    }

    #[test]
    fn activity_transition_preserves_current_task() {
        let mut status = status();
        AgentStatusTransitionPolicy::apply_activity(
            &mut status,
            AgentActivity::Working {
                context: "run".into(),
            },
        );
        assert_eq!(status.current_task, Some("task".into()));
        assert!(matches!(status.activity, AgentActivity::Working { .. }));
    }

    #[test]
    fn state_transition_updates_state() {
        let mut status = status();
        AgentStatusTransitionPolicy::apply_state(&mut status, AgentState::Running);
        assert_eq!(status.state, AgentState::Running);
    }
}
