//! Agent status tracker — tracks runtime activity of all agents.

use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use macaca_proto::{AgentActivity, AgentId, AgentRuntimeStatus, AgentState};

use crate::status_transition::AgentStatusTransitionPolicy;

/// Thread-safe tracker for agent runtime status.
pub struct AgentStatusTracker {
    statuses: Arc<RwLock<HashMap<AgentId, AgentRuntimeStatus>>>,
}

impl AgentStatusTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self {
            statuses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an agent with initial idle status.
    pub async fn register(&self, agent_id: AgentId, name: String) {
        let status = AgentRuntimeStatus {
            agent_id,
            name,
            state: AgentState::Created,
            activity: AgentActivity::Idle,
            updated_at: Utc::now(),
            current_task: None,
        };
        self.statuses.write().await.insert(agent_id, status);
        tracing::debug!(agent_id = %agent_id.0, "agent status registered");
    }

    /// Unregister an agent.
    pub async fn unregister(&self, agent_id: &AgentId) {
        self.statuses.write().await.remove(agent_id);
        tracing::debug!(agent_id = %agent_id.0, "agent status unregistered");
    }

    /// Update an agent's lifecycle state.
    pub async fn update_state(&self, agent_id: &AgentId, state: AgentState) {
        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.get_mut(agent_id) {
            AgentStatusTransitionPolicy::apply_state(status, state);
        }
    }

    /// Update an agent's current activity.
    pub async fn update_activity(&self, agent_id: &AgentId, activity: AgentActivity) {
        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.get_mut(agent_id) {
            AgentStatusTransitionPolicy::apply_activity(status, activity);
        }
    }

    /// Set the current task description for an agent.
    pub async fn set_task(&self, agent_id: &AgentId, task: Option<String>) {
        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.get_mut(agent_id) {
            status.current_task = task;
            status.updated_at = Utc::now();
        }
    }

    /// Convenience method: mark agent as thinking.
    pub async fn set_thinking(&self, agent_id: &AgentId, context: impl Into<String>) {
        self.update_activity(
            agent_id,
            AgentActivity::Thinking {
                context: context.into(),
            },
        )
        .await;
    }

    /// Convenience method: mark agent as working.
    pub async fn set_working(&self, agent_id: &AgentId, context: impl Into<String>) {
        self.update_activity(
            agent_id,
            AgentActivity::Working {
                context: context.into(),
            },
        )
        .await;
    }

    /// Convenience method: mark agent as having an error.
    pub async fn set_error(&self, agent_id: &AgentId, message: impl Into<String>) {
        self.update_activity(
            agent_id,
            AgentActivity::Error {
                message: message.into(),
            },
        )
        .await;
    }

    /// Convenience method: mark agent as idle.
    pub async fn set_idle(&self, agent_id: &AgentId) {
        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.get_mut(agent_id) {
            AgentStatusTransitionPolicy::apply_idle(status);
        }
    }

    /// Get the status of a specific agent.
    pub async fn get(&self, agent_id: &AgentId) -> Option<AgentRuntimeStatus> {
        self.statuses.read().await.get(agent_id).cloned()
    }

    /// Get all agent statuses.
    pub async fn list(&self) -> Vec<AgentRuntimeStatus> {
        self.statuses.read().await.values().cloned().collect()
    }

    /// Get statuses for agents belonging to a specific app.
    /// This requires the caller to provide the agent IDs.
    pub async fn list_for_agents(&self, agent_ids: &[AgentId]) -> Vec<AgentRuntimeStatus> {
        let statuses = self.statuses.read().await;
        agent_ids
            .iter()
            .filter_map(|id| statuses.get(id).cloned())
            .collect()
    }
}

impl Default for AgentStatusTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_and_get_status() {
        let tracker = AgentStatusTracker::new();
        let id = AgentId::new();
        tracker.register(id, "test-agent".into()).await;

        let status = tracker.get(&id).await.unwrap();
        assert_eq!(status.name, "test-agent");
        assert_eq!(status.state, AgentState::Created);
        assert!(matches!(status.activity, AgentActivity::Idle));
    }

    #[tokio::test]
    async fn update_activity() {
        let tracker = AgentStatusTracker::new();
        let id = AgentId::new();
        tracker.register(id, "test".into()).await;

        tracker.set_thinking(&id, "processing request").await;
        let status = tracker.get(&id).await.unwrap();
        match status.activity {
            AgentActivity::Thinking { context } => assert_eq!(context, "processing request"),
            _ => panic!("Expected Thinking activity"),
        }

        tracker.set_working(&id, "run tests").await;
        let status = tracker.get(&id).await.unwrap();
        match status.activity {
            AgentActivity::Working { context } => {
                assert_eq!(context, "run tests");
            }
            _ => panic!("Expected Working activity"),
        }

        tracker.set_idle(&id).await;
        let status = tracker.get(&id).await.unwrap();
        assert!(matches!(status.activity, AgentActivity::Idle));
    }

    #[tokio::test]
    async fn list_all_statuses() {
        let tracker = AgentStatusTracker::new();
        let id1 = AgentId::new();
        let id2 = AgentId::new();

        tracker.register(id1, "agent1".into()).await;
        tracker.register(id2, "agent2".into()).await;

        let all = tracker.list().await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn unregister_removes_status() {
        let tracker = AgentStatusTracker::new();
        let id = AgentId::new();
        tracker.register(id, "test".into()).await;
        assert_eq!(tracker.list().await.len(), 1);

        tracker.unregister(&id).await;
        assert_eq!(tracker.list().await.len(), 0);
        assert!(tracker.get(&id).await.is_none());
    }
}
