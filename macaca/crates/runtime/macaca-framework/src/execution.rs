//! Framework-level execution session primitive.
//!
//! `ExecutionContext` captures resumable run status (`running/paused/resumed/...`).
//! Callers persist it through the provider-neutral `AgentState` memento.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Lifecycle status of an execution session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Running,
    Paused,
    Resumed,
    Completed,
    Error,
    Stopped,
}

/// Durable execution context for session/trace/resume flows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Session identifier.
    pub session_id: String,
    /// Application identifier (string form).
    pub app_id: String,
    /// Entry/owner agent name.
    pub owner_agent: String,
    /// Current execution status.
    pub status: ExecutionStatus,
    /// Optional explanation for the latest transition.
    pub reason: Option<String>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl ExecutionContext {
    pub fn new(
        session_id: impl Into<String>,
        app_id: impl Into<String>,
        owner_agent: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            app_id: app_id.into(),
            owner_agent: owner_agent.into(),
            status: ExecutionStatus::Running,
            reason: None,
            updated_at: Utc::now(),
        }
    }

    pub fn mark_running(&mut self, reason: Option<String>) {
        self.status = ExecutionStatus::Running;
        self.reason = reason;
        self.updated_at = Utc::now();
    }

    pub fn mark_paused(&mut self, reason: Option<String>) {
        self.status = ExecutionStatus::Paused;
        self.reason = reason;
        self.updated_at = Utc::now();
    }

    pub fn mark_resumed(&mut self, reason: Option<String>) {
        self.status = ExecutionStatus::Resumed;
        self.reason = reason;
        self.updated_at = Utc::now();
    }

    pub fn mark_completed(&mut self, reason: Option<String>) {
        self.status = ExecutionStatus::Completed;
        self.reason = reason;
        self.updated_at = Utc::now();
    }

    pub fn mark_error(&mut self, reason: Option<String>) {
        self.status = ExecutionStatus::Error;
        self.reason = reason;
        self.updated_at = Utc::now();
    }

    pub fn mark_stopped(&mut self, reason: Option<String>) {
        self.status = ExecutionStatus::Stopped;
        self.reason = reason;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_context_roundtrip() {
        let mut ctx = ExecutionContext::new("s1", "app1", "entry");
        ctx.mark_paused(Some("waiting goal".into()));
        let state = serde_json::to_value(&ctx).expect("serialize execution context");

        let restored: ExecutionContext =
            serde_json::from_value(state).expect("load execution context");
        assert_eq!(restored.session_id, "s1");
        assert_eq!(restored.app_id, "app1");
        assert_eq!(restored.owner_agent, "entry");
        assert_eq!(restored.status, ExecutionStatus::Paused);
        assert_eq!(restored.reason.as_deref(), Some("waiting goal"));
    }
}
