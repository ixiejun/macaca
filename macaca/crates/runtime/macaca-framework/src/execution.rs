//! Framework-level execution session primitive.
//!
//! `ExecutionContext` captures resumable run status (`running/paused/resumed/...`)
//! and is persistable via `StateModule` + `SessionStore`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::{StateError, StateModule};

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

impl StateModule for ExecutionContext {
    fn state_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "session_id": self.session_id,
            "app_id": self.app_id,
            "owner_agent": self.owner_agent,
            "status": self.status,
            "reason": self.reason,
            "updated_at": self.updated_at,
        })
    }

    fn load_state_dict(&mut self, state: serde_json::Value) -> Result<(), StateError> {
        self.session_id = state
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StateError::MissingField("session_id".into()))?
            .to_string();
        self.app_id = state
            .get("app_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StateError::MissingField("app_id".into()))?
            .to_string();
        self.owner_agent = state
            .get("owner_agent")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StateError::MissingField("owner_agent".into()))?
            .to_string();
        self.status = serde_json::from_value(
            state
                .get("status")
                .ok_or_else(|| StateError::MissingField("status".into()))?
                .clone(),
        )
        .map_err(|e| StateError::DeserializeFailed(e.to_string()))?;
        self.reason = state
            .get("reason")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        self.updated_at = serde_json::from_value(
            state
                .get("updated_at")
                .ok_or_else(|| StateError::MissingField("updated_at".into()))?
                .clone(),
        )
        .map_err(|e| StateError::DeserializeFailed(e.to_string()))?;
        Ok(())
    }

    fn module_name(&self) -> &str {
        "execution_context"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_context_roundtrip() {
        let mut ctx = ExecutionContext::new("s1", "app1", "entry");
        ctx.mark_paused(Some("waiting goal".into()));
        let state = ctx.state_dict();

        let mut restored = ExecutionContext::new("", "", "");
        restored
            .load_state_dict(state)
            .expect("load execution context");
        assert_eq!(restored.session_id, "s1");
        assert_eq!(restored.app_id, "app1");
        assert_eq!(restored.owner_agent, "entry");
        assert_eq!(restored.status, ExecutionStatus::Paused);
        assert_eq!(restored.reason.as_deref(), Some("waiting goal"));
    }
}
