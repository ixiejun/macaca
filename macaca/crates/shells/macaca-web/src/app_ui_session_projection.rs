//! Session-log projection for application-owned UI bridge calls.
//!
//! Application-owned UI bundles can run rich task loops through `macaca.call`
//! without going through `/api/chat/v2`.  The shell sidebar, however, is backed
//! by the existing `StoredSession` memento and `app_sessions/{app}/{session}`
//! reverse index.  This module bridges those two generic shell concerns without
//! teaching the OS anything about a specific application, workflow, or payload.
//!
//! The projection is intentionally small: it records only app/session identity
//! plus sanitized bridge routing metadata.  It does not inspect the application
//! payload because payload fields are application-owned business data.

use std::sync::Arc;

use chrono::Utc;
use macaca_persist::PersistBackend;
use macaca_proto::{ApplicationId, ApplicationUiBridgeRequest, MacacaResult};
use tracing::info;

use crate::session::{SessionMeta, StoredSession, StoredTurn, APP_SESSIONS_PREFIX, SESSION_PREFIX};

/// Memento writer that makes app-owned UI bridge sessions visible to shell UI.
///
/// The type is a focused Decorator/Memento helper:
/// - Decorator: route handlers call it around bridge dispatch without changing
///   service-call semantics.
/// - Memento: it persists the smallest refreshable session snapshot needed by
///   the shell sidebar.
pub(crate) struct AppUiSessionProjection {
    store: Arc<dyn PersistBackend>,
}

impl AppUiSessionProjection {
    /// Build a projection writer over the shell session store.
    pub(crate) fn new(store: Arc<dyn PersistBackend>) -> Self {
        Self { store }
    }

    /// Ensure a bridge-scoped session exists in the shell session log.
    ///
    /// Blank or absent `session_id` values are ignored because the shell has no
    /// stable refresh key.  This preserves the current fire-and-forget bridge
    /// behavior for UI calls that do not opt into session visibility.
    pub(crate) async fn record_bridge_received(
        &self,
        app_id: &ApplicationId,
        request: &ApplicationUiBridgeRequest,
    ) -> MacacaResult<()> {
        let Some(session_id) = normalized_session_id(request) else {
            info!(
                app_id = %app_id,
                command_id = %request.command_id,
                capability = %request.capability,
                "application UI bridge session projection skipped because session_id is absent"
            );
            return Ok(());
        };

        let now = Utc::now();
        let session_key = format!("{}{}", SESSION_PREFIX, session_id);
        let mut stored = match self.store.get(&session_key).await? {
            Some(bytes) => serde_json::from_slice::<StoredSession>(&bytes)?,
            None => new_bridge_session(app_id, session_id, request, now),
        };

        // Existing shell sessions may already contain richer chat turns or
        // replay data written by another generic shell path.  The projection is
        // deliberately conservative: it refreshes the metadata that the sidebar
        // sorts by, but it only creates the synthetic bridge turn for a brand
        // new/empty memento so repeated service calls do not inflate the log.
        stored.meta.updated_at = now;
        stored.meta.status = "running".to_string();
        if stored.turns.is_empty() && stored.messages.is_empty() {
            stored.turns.push(bridge_projection_turn(request));
            stored.meta.message_count = stored.turns.len().max(1);
        }

        let data = serde_json::to_vec(&stored)?;
        self.store.set(&session_key, &data).await?;

        let app_index_key = format!("{}{}/{}", APP_SESSIONS_PREFIX, app_id.0, session_id);
        self.store
            .set(&app_index_key, session_id.as_bytes())
            .await?;

        info!(
            app_id = %app_id,
            session_id,
            command_id = %request.command_id,
            capability = %request.capability,
            service_id = ?request.service_id,
            operation = ?request.operation,
            trace_id = ?request.trace_id,
            "application UI bridge session projection persisted"
        );
        Ok(())
    }
}

fn normalized_session_id(request: &ApplicationUiBridgeRequest) -> Option<&str> {
    request
        .session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn new_bridge_session(
    app_id: &ApplicationId,
    session_id: &str,
    request: &ApplicationUiBridgeRequest,
    now: chrono::DateTime<Utc>,
) -> StoredSession {
    StoredSession {
        meta: SessionMeta {
            session_id: session_id.to_string(),
            app_id: app_id.0.to_string(),
            created_at: now,
            updated_at: now,
            message_count: 1,
            title: Some(bridge_projection_title(request)),
            status: "running".to_string(),
        },
        turns: vec![bridge_projection_turn(request)],
        messages: Vec::new(),
    }
}

fn bridge_projection_title(request: &ApplicationUiBridgeRequest) -> String {
    match (
        request
            .service_id
            .as_deref()
            .filter(|value| !value.is_empty()),
        request
            .operation
            .as_deref()
            .filter(|value| !value.is_empty()),
    ) {
        (Some(service_id), Some(operation)) => {
            format!("Application UI bridge: {service_id}/{operation}")
        }
        _ => format!("Application UI bridge: {}", request.capability),
    }
}

fn bridge_projection_turn(request: &ApplicationUiBridgeRequest) -> StoredTurn {
    let mut content = format!(
        "Application UI bridge command received: capability={}",
        request.capability
    );
    if let Some(service_id) = request
        .service_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        content.push_str(&format!(", service_id={service_id}"));
    }
    if let Some(operation) = request
        .operation
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        content.push_str(&format!(", operation={operation}"));
    }
    if let Some(trace_id) = request
        .trace_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        content.push_str(&format!(", trace_id={trace_id}"));
    }

    StoredTurn {
        role: "system".into(),
        content,
        status: Some("running".into()),
        trace_steps: Vec::new(),
        meta: None,
        agent_traces: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use macaca_persist::{PersistBackend, RedbStore};
    use macaca_proto::{ApplicationId, ApplicationUiBridgeRequest};
    use serde_json::json;
    use tempfile::tempdir;
    use uuid::Uuid;

    use super::*;
    use crate::session::{StoredSession, APP_SESSIONS_PREFIX, SESSION_PREFIX};

    fn temp_store() -> (Arc<dyn PersistBackend>, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("session-projection.redb");
        let store = RedbStore::open(db_path).expect("open redb store");
        (Arc::new(store), dir)
    }

    #[tokio::test]
    async fn bridge_projection_creates_sidebar_session_index() {
        let (store, _dir) = temp_store();
        let app_id =
            ApplicationId(Uuid::parse_str("6fbb0369-e1c9-5a98-89b7-eb01f9c9fa93").unwrap());
        let request = ApplicationUiBridgeRequest {
            bridge_version: "macaca.ui.bridge.v1".into(),
            session_id: Some("workbench-session-1".into()),
            surface_id: Some("app-owned-ui".into()),
            trace_id: Some("trace-workbench-session-1".into()),
            command_id: "command-1".into(),
            capability: "service.call".into(),
            service_id: Some("service.git".into()),
            operation: Some("git.status".into()),
            payload: json!({"ignored":"business payload must not be projected"}),
        };

        AppUiSessionProjection::new(Arc::clone(&store))
            .record_bridge_received(&app_id, &request)
            .await
            .expect("projection writes");

        let stored_bytes = store
            .get(&format!("{}{}", SESSION_PREFIX, "workbench-session-1"))
            .await
            .expect("session get")
            .expect("session exists");
        let stored: StoredSession = serde_json::from_slice(&stored_bytes).expect("stored session");
        assert_eq!(stored.meta.app_id, app_id.0.to_string());
        assert_eq!(stored.meta.session_id, "workbench-session-1");
        assert_eq!(stored.meta.status, "running");
        assert_eq!(stored.meta.message_count, 1);
        assert_eq!(stored.turns[0].role, "system");
        assert!(stored.turns[0].content.contains("service.call"));
        assert!(!stored.turns[0].content.contains("business payload"));

        let index_key = format!(
            "{}{}/{}",
            APP_SESSIONS_PREFIX, app_id.0, "workbench-session-1"
        );
        let indexed = store
            .get(&index_key)
            .await
            .expect("index get")
            .expect("index exists");
        assert_eq!(indexed, b"workbench-session-1");
    }

    #[tokio::test]
    async fn bridge_projection_ignores_empty_session_id() {
        let (store, _dir) = temp_store();
        let app_id =
            ApplicationId(Uuid::parse_str("6fbb0369-e1c9-5a98-89b7-eb01f9c9fa93").unwrap());
        let request = ApplicationUiBridgeRequest {
            bridge_version: "macaca.ui.bridge.v1".into(),
            session_id: Some("   ".into()),
            surface_id: Some("app-owned-ui".into()),
            trace_id: Some("trace-without-session".into()),
            command_id: "command-without-session".into(),
            capability: "service.call".into(),
            service_id: Some("service.git".into()),
            operation: Some("git.status".into()),
            payload: json!({"ignored":"business payload must not be projected"}),
        };

        AppUiSessionProjection::new(Arc::clone(&store))
            .record_bridge_received(&app_id, &request)
            .await
            .expect("empty session id is accepted as a no-op");

        let mut keys = store
            .list_keys(SESSION_PREFIX)
            .await
            .expect("scan sessions");
        keys.sort();
        assert!(
            keys.is_empty(),
            "bridge calls without a stable session id must not create refresh-only session entries"
        );
    }
}
