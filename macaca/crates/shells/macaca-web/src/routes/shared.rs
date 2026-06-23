//! Shared route utilities: error envelopes, app-scoped agent helpers, catch-all.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use macaca_proto::{
    ApplicationId, ApplicationMetadataQueryCommand, MacacaError, ProtoErrorAdapter, TraceContext,
};

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Shared utilities (used by sibling modules via `crate::routes::err` etc.)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub(crate) fn err(status: StatusCode, msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse { error: msg }))
}

pub(crate) fn proto_err(
    status: StatusCode,
    error: &MacacaError,
) -> (StatusCode, Json<ErrorResponse>) {
    err(
        status,
        format!("{}: {}", error.code(), error.display_message()),
    )
}

pub(crate) fn default_model() -> String {
    // Placeholder for serde default. The actual model is resolved at runtime
    // from AppState.default_model in the handler.
    String::new()
}

#[derive(Debug, Deserialize, Default)]
pub struct AgentStatusQuery {
    pub session_id: Option<String>,
}

pub(crate) async fn app_has_active_session(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    session_id: Option<&str>,
) -> bool {
    let sessions = state.sessions.active_sessions.read().await;
    if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
        return sessions
            .get(session_id)
            .is_some_and(|session| session.app_id == *app_id);
    }

    sessions.values().any(|session| session.app_id == *app_id)
}

pub(crate) async fn app_entry_agent_name(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> Option<String> {
    match ApplicationMetadataQueryCommand::application(
        TraceContext::new("web-route-entry-agent-metadata"),
        *app_id,
    ) {
        Ok(command) => match state.application_client.metadata(command).await {
            Ok(view) => return view.entry.agent_name,
            Err(error) => tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Application metadata query failed; entry agent is unavailable"
            ),
        },
        Err(error) => tracing::warn!(
            app_id = %app_id,
            error = %error,
            "Application metadata query rejected before dispatch"
        ),
    }
    None
}

pub(crate) fn entry_agent_activity_override(
    entry_agent_name: Option<&str>,
    agent_name: &str,
    has_active_session: bool,
    activity: macaca_proto::AgentActivity,
) -> macaca_proto::AgentActivity {
    if entry_agent_name.is_some_and(|entry| entry == agent_name)
        && has_active_session
        && matches!(activity, macaca_proto::AgentActivity::Idle)
    {
        macaca_proto::AgentActivity::Working {
            context: "Coordinating active session".to_string(),
        }
    } else {
        activity
    }
}

// ---------------------------------------------------------------------------
// Catch-all
// ---------------------------------------------------------------------------

pub async fn root_not_found() -> (StatusCode, Json<ErrorResponse>) {
    err(
        StatusCode::NOT_FOUND,
        "Agent OS API server does not host a web UI at /".into(),
    )
}
