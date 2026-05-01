//! Security audit log system — records structured audit events to RedbStore
//! and provides a query API with filtering support.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use macaca_persist::PersistBackend;
use macaca_proto::ApplicationId;
use serde::{Deserialize, Serialize};

/// Types of auditable events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    ToolExecuted {
        tool_name: String,
        success: bool,
    },
    TaskDelegated {
        agent: String,
        task_id: String,
    },
    PermissionDenied {
        tool_name: String,
        reason: String,
    },
    BudgetExceeded {
        spent_usd: f64,
        limit_usd: f64,
    },
    ForkCreated {
        fork_id: String,
        target_agent: String,
    },
    ScheduleTriggered {
        schedule_id: String,
        schedule_name: String,
    },
    WorkerRestarted {
        agent: String,
        restart_count: u32,
    },
    AgentStarted {
        agent: String,
    },
    AgentStopped {
        agent: String,
        reason: String,
    },
}

/// A single audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub app_id: ApplicationId,
    pub agent_id: String,
    pub action: AuditAction,
    pub timestamp: DateTime<Utc>,
    /// Optional metadata (e.g., truncated args hash).
    pub metadata: Option<String>,
}

impl AuditEvent {
    pub fn new(app_id: ApplicationId, agent_id: impl Into<String>, action: AuditAction) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            app_id,
            agent_id: agent_id.into(),
            action,
            timestamp: Utc::now(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, meta: impl Into<String>) -> Self {
        self.metadata = Some(meta.into());
        self
    }
}

/// Query parameters for filtering audit events.
#[derive(Debug, Default)]
pub struct AuditQuery {
    pub agent_id: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

const AUDIT_PREFIX: &str = "audit/";

/// Persistent audit logger backed by RedbStore.
#[derive(Clone)]
pub struct AuditLogger {
    store: Arc<dyn PersistBackend>,
}

impl AuditLogger {
    pub fn new<T>(store: Arc<T>) -> Self
    where
        T: PersistBackend + 'static,
    {
        Self { store }
    }

    fn event_key(app_id: &ApplicationId, event_id: &str) -> String {
        format!("{}{}/{}", AUDIT_PREFIX, app_id, event_id)
    }

    fn app_prefix(app_id: &ApplicationId) -> String {
        format!("{}{}/", AUDIT_PREFIX, app_id)
    }

    /// Record an audit event. Errors are logged but not propagated.
    pub async fn record(&self, event: AuditEvent) {
        let key = Self::event_key(&event.app_id, &event.id);
        match serde_json::to_vec(&event) {
            Ok(data) => {
                let _ = self.store.set(&key, &data).await;
            }
            Err(e) => tracing::error!("Failed to serialize audit event: {}", e),
        }
    }

    /// Convenience: record a tool execution event.
    pub async fn log_tool(
        &self,
        app_id: ApplicationId,
        agent: &str,
        tool_name: &str,
        success: bool,
    ) {
        self.record(AuditEvent::new(
            app_id,
            agent,
            AuditAction::ToolExecuted {
                tool_name: tool_name.into(),
                success,
            },
        ))
        .await;
    }

    /// Query audit events with optional filters. Results are sorted newest-first.
    pub async fn query(&self, app_id: &ApplicationId, query: &AuditQuery) -> Vec<AuditEvent> {
        let prefix = Self::app_prefix(app_id);
        let keys = self.store.list_keys(&prefix).await.unwrap_or_default();
        let mut events = Vec::new();
        for key in keys {
            if let Ok(Some(data)) = self.store.get(&key).await {
                if let Ok(event) = serde_json::from_slice::<AuditEvent>(&data) {
                    if let Some(ref agent) = query.agent_id {
                        if &event.agent_id != agent {
                            continue;
                        }
                    }
                    if let Some(since) = query.since {
                        if event.timestamp < since {
                            continue;
                        }
                    }
                    if let Some(until) = query.until {
                        if event.timestamp > until {
                            continue;
                        }
                    }
                    events.push(event);
                }
            }
        }
        // Sort by timestamp descending (newest first)
        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if let Some(limit) = query.limit {
            events.truncate(limit);
        }
        events
    }

    /// Count total audit events for an application.
    pub async fn count(&self, app_id: &ApplicationId) -> usize {
        let prefix = Self::app_prefix(app_id);
        self.store
            .list_keys(&prefix)
            .await
            .unwrap_or_default()
            .len()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Duration;
    use tempfile::tempdir;

    use super::*;

    fn make_logger() -> (AuditLogger, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("audit_test.db");
        let store = Arc::new(macaca_persist::RedbStore::open(path).expect("open store"));
        (AuditLogger::new(store), dir)
    }

    #[tokio::test]
    async fn test_record_and_query() {
        let (logger, _dir) = make_logger();
        let app_id = ApplicationId::new();

        logger
            .record(AuditEvent::new(
                app_id,
                "agent-a",
                AuditAction::AgentStarted {
                    agent: "agent-a".into(),
                },
            ))
            .await;
        logger
            .record(AuditEvent::new(
                app_id,
                "agent-b",
                AuditAction::ToolExecuted {
                    tool_name: "shell".into(),
                    success: true,
                },
            ))
            .await;
        logger
            .record(AuditEvent::new(
                app_id,
                "agent-c",
                AuditAction::AgentStopped {
                    agent: "agent-c".into(),
                    reason: "done".into(),
                },
            ))
            .await;

        let events = logger.query(&app_id, &AuditQuery::default()).await;
        assert_eq!(events.len(), 3);

        let count = logger.count(&app_id).await;
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_query_filter_by_agent() {
        let (logger, _dir) = make_logger();
        let app_id = ApplicationId::new();

        for _ in 0..3 {
            logger
                .record(AuditEvent::new(
                    app_id,
                    "agent-x",
                    AuditAction::ToolExecuted {
                        tool_name: "read".into(),
                        success: true,
                    },
                ))
                .await;
        }
        logger
            .record(AuditEvent::new(
                app_id,
                "agent-y",
                AuditAction::ToolExecuted {
                    tool_name: "write".into(),
                    success: false,
                },
            ))
            .await;

        let query = AuditQuery {
            agent_id: Some("agent-x".into()),
            ..Default::default()
        };
        let events = logger.query(&app_id, &query).await;
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| e.agent_id == "agent-x"));

        let query_y = AuditQuery {
            agent_id: Some("agent-y".into()),
            ..Default::default()
        };
        let events_y = logger.query(&app_id, &query_y).await;
        assert_eq!(events_y.len(), 1);
    }

    #[tokio::test]
    async fn test_query_filter_by_time() {
        let (logger, _dir) = make_logger();
        let app_id = ApplicationId::new();

        let now = Utc::now();

        // Record one event with a manually constructed past timestamp
        let mut old_event = AuditEvent::new(
            app_id,
            "agent-a",
            AuditAction::AgentStarted {
                agent: "agent-a".into(),
            },
        );
        old_event.timestamp = now - Duration::hours(2);
        logger.record(old_event).await;

        // Record a recent event
        logger
            .record(AuditEvent::new(
                app_id,
                "agent-a",
                AuditAction::AgentStarted {
                    agent: "agent-a".into(),
                },
            ))
            .await;

        // Query with since = 1 hour ago — should only return the recent event
        let query = AuditQuery {
            since: Some(now - Duration::hours(1)),
            ..Default::default()
        };
        let events = logger.query(&app_id, &query).await;
        assert_eq!(events.len(), 1);

        // Query with until = 1 hour ago — should only return the old event
        let query_until = AuditQuery {
            until: Some(now - Duration::hours(1)),
            ..Default::default()
        };
        let events_until = logger.query(&app_id, &query_until).await;
        assert_eq!(events_until.len(), 1);
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let (logger, _dir) = make_logger();
        let app_id = ApplicationId::new();

        for i in 0..10u32 {
            logger
                .record(AuditEvent::new(
                    app_id,
                    "agent-a",
                    AuditAction::WorkerRestarted {
                        agent: "agent-a".into(),
                        restart_count: i,
                    },
                ))
                .await;
        }

        let query = AuditQuery {
            limit: Some(3),
            ..Default::default()
        };
        let events = logger.query(&app_id, &query).await;
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn test_log_tool_convenience() {
        let (logger, _dir) = make_logger();
        let app_id = ApplicationId::new();

        logger.log_tool(app_id, "agent-z", "bash", true).await;
        logger.log_tool(app_id, "agent-z", "file_read", false).await;

        let events = logger.query(&app_id, &AuditQuery::default()).await;
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_events_isolated_by_app_id() {
        let (logger, _dir) = make_logger();
        let app1 = ApplicationId::new();
        let app2 = ApplicationId::new();

        logger
            .record(AuditEvent::new(
                app1,
                "agent-a",
                AuditAction::AgentStarted {
                    agent: "agent-a".into(),
                },
            ))
            .await;
        logger
            .record(AuditEvent::new(
                app2,
                "agent-b",
                AuditAction::AgentStarted {
                    agent: "agent-b".into(),
                },
            ))
            .await;

        assert_eq!(logger.count(&app1).await, 1);
        assert_eq!(logger.count(&app2).await, 1);
        assert_eq!(logger.query(&app1, &AuditQuery::default()).await.len(), 1);
    }
}
