//! Session and execution-context persistence adapters.
//!
//! The web shell owns browser-visible session envelopes while framework module
//! state lives in the framework session store. These helpers keep both stores
//! aligned without leaking persistence details into route handlers.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use macaca_proto::ApplicationId;
use macaca_sdk::framework::execution::ExecutionContext;

use crate::session::{SessionMeta, StoredSession, StoredTurn, APP_SESSIONS_PREFIX, SESSION_PREFIX};
use crate::state::AppState;

pub(crate) async fn persist_execution_context(state: &Arc<AppState>, context: &ExecutionContext) {
    crate::framework_state_memento::save_execution_context(
        state.sessions.framework_session_store.as_ref(),
        context,
    )
    .await;
}

pub(crate) async fn persist_initial_chat_session(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_key: &str,
    prompt: &str,
) {
    let now = Utc::now();
    let title = prompt.chars().take(50).collect::<String>();
    let session_key_db = format!("{}{}", SESSION_PREFIX, session_key);
    let initial_stored = StoredSession {
        meta: SessionMeta {
            session_id: session_key.to_string(),
            app_id: app_id.0.to_string(),
            created_at: now,
            updated_at: now,
            message_count: 1,
            title: Some(title),
            status: "running".to_string(),
        },
        turns: vec![StoredTurn {
            role: "user".into(),
            content: prompt.to_string(),
            status: None,
            trace_steps: Vec::new(),
            meta: None,
            agent_traces: HashMap::new(),
        }],
        messages: vec![],
    };
    if let Ok(data) = serde_json::to_vec(&initial_stored) {
        let _ = state
            .persist
            .session_store
            .set(&session_key_db, &data)
            .await;
    }

    // Maintain the per-app reverse index used by the left sidebar session log.
    let app_index_key = format!("{}{}/{}", APP_SESSIONS_PREFIX, app_id.0, session_key);
    let _ = state
        .persist
        .session_store
        .set(&app_index_key, session_key.as_bytes())
        .await;
}
