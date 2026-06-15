//! Session snapshot persistence with per-session write locks (Unit of Work seam).
//!
//! `persist_session_snapshot` performs read-modify-write under a session-scoped async mutex
//! so concurrent SSE savers and executor status updates cannot clobber each other.

use std::sync::Arc;

use chrono::Utc;
use macaca_host_composition::persist::PersistStore;
use macaca_proto::ApplicationId;

use super::turn_model::{ensure_running_assistant_turn, stored_turns_or_messages};
use super::types::{
    AgentTrace, AssistantExecutionMeta, SessionMeta, StoredSession, StoredTraceStep,
};

// Key prefixes for redb storage
pub(crate) const SESSION_PREFIX: &str = "session/";
pub(crate) const APP_SESSIONS_PREFIX: &str = "app_sessions/";
/// Separate key prefix for agent traces — stored independently from session
/// to avoid read-modify-write races with session updates.
pub(crate) const AGENT_TRACES_PREFIX: &str = "agent_traces/";

pub(crate) async fn persist_session_snapshot<S>(
    store: &Arc<S>,
    session_id: &str,
    app_id: &ApplicationId,
    status: Option<&str>,
    content: Option<String>,
    trace_steps: Option<Vec<StoredTraceStep>>,
    agent_traces: Option<std::collections::HashMap<String, Vec<AgentTrace>>>,
    meta: Option<AssistantExecutionMeta>,
) where
    S: PersistStore + ?Sized,
{
    // Acquire per-session write lock to prevent concurrent read-modify-write
    // from overwriting each other's data (e.g., periodic saver vs snapshot closure).
    // We use a simple approach: lock the session_key_db string as a file-level mutex.
    static SESSION_LOCKS: std::sync::OnceLock<
        tokio::sync::Mutex<std::collections::HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    > = std::sync::OnceLock::new();
    let locks =
        SESSION_LOCKS.get_or_init(|| tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let lock = {
        let mut map = locks.lock().await;
        map.entry(session_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };
    let _guard = lock.lock().await;

    let session_key_db = format!("{}{}", SESSION_PREFIX, session_id);
    let existing = match store.get(&session_key_db).await {
        Ok(Some(data)) => match serde_json::from_slice::<StoredSession>(&data) {
            Ok(s) => s,
            Err(_) => return,
        },
        _ => return,
    };

    let mut turns = stored_turns_or_messages(&existing);
    let mut next_status = existing.meta.status.clone();

    if let Some(status) = status {
        next_status = status.to_string();
        let turn = ensure_running_assistant_turn(&mut turns);
        turn.status = Some(status.to_string());
        if let Some(content) = content {
            turn.content = content;
        }
        if let Some(trace_steps) = trace_steps {
            turn.trace_steps = trace_steps;
        }
        if let Some(agent_traces) = agent_traces {
            turn.agent_traces = agent_traces;
        }
        if let Some(meta) = meta {
            turn.meta = Some(meta);
        }
    }

    let stored = StoredSession {
        meta: SessionMeta {
            session_id: session_id.to_string(),
            app_id: app_id.0.to_string(),
            created_at: existing.meta.created_at,
            updated_at: Utc::now(),
            message_count: existing.meta.message_count,
            title: existing.meta.title,
            status: next_status,
        },
        messages: existing.messages,
        turns,
    };

    if let Ok(data) = serde_json::to_vec(&stored) {
        let _ = store.set(&session_key_db, &data).await;
    }
}
