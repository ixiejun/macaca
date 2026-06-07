//! Thin Web Shell adapter routes for GenUI Runtime v0.
//!
//! This module deliberately keeps application UI semantics out of `macaca-web`.
//! Routes validate session/application scope, persist traceable UI records, and
//! hand schema-defined data to the frontend.  Actual application behavior and
//! rendering strategy remain behind Application Framework and frontend
//! renderer boundaries.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_runtime_host::persist::{AppendEventCommand, EventLog};
use macaca_proto::{
    ApplicationGenUiSurfaceCommand, ApplicationServiceScope, TraceContext, UiAction, UiEvent,
    UiEventCommand, UiIntent, UiRenderError, UiRenderSurface,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

use crate::routes::{err, ErrorResponse};
use crate::state::AppState;

/// Query for fetching a GenUI surface.
#[derive(Debug, Deserialize)]
pub struct GenUiSurfaceQuery {
    pub session_id: Option<String>,
}

/// Request body for posting a UI event command.
#[derive(Debug, Deserialize)]
pub struct GenUiEventRequest {
    pub session_id: String,
    pub surface_id: String,
    pub component_id: String,
    pub action: UiAction,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// Response returned after persisting a UI event.
#[derive(Debug, Serialize)]
pub struct GenUiEventResponse {
    pub event_id: String,
    pub seq: u64,
    pub persisted: bool,
}

/// GET /api/apps/{app_id}/genui/surface?session_id=...
pub async fn get_genui_surface(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<GenUiSurfaceQuery>,
) -> Result<Json<Option<UiIntent>>, (StatusCode, Json<ErrorResponse>)> {
    let session_id = required_scope(&app_id, query.session_id.as_deref())?;
    let application_id = parse_application_id(&app_id)?;
    let trace = trace_for_genui(&session_id, "genui-surface");
    let command = ApplicationGenUiSurfaceCommand {
        trace: trace.clone(),
        scope: ApplicationServiceScope {
            application_id: Some(application_id),
            application_name: None,
            session_id: Some(session_id.clone()),
            agent_name: None,
        },
        surface_id: None,
    };
    match state.application_client.genui_surface(command).await {
        Ok(surface) => {
            info!(
                app_id = %app_id,
                session_id = %session_id,
                trace_id = %trace.trace_id,
                found = surface.is_some(),
                "GenUI surface query completed through Application Service"
            );
            return Ok(Json(surface));
        }
        Err(error) => {
            warn!(
                app_id = %app_id,
                session_id = %session_id,
                trace_id = %trace.trace_id,
                error = %error,
                "Application Service GenUI surface query failed; returning v0 no-surface fallback"
            );
        }
    }
    info!(
        app_id = %app_id,
        session_id = %session_id,
        "GenUI surface requested; returning empty shell fallback after Application Service error"
    );
    Ok(Json(None))
}

/// POST /api/apps/{app_id}/genui/events
pub async fn post_genui_event(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(request): Json<GenUiEventRequest>,
) -> Result<Json<GenUiEventResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session_id = required_scope(&app_id, Some(&request.session_id))?;
    let trace = trace_for_genui(
        &session_id,
        request.trace_id.as_deref().unwrap_or("genui-event"),
    );
    let event_id = Uuid::new_v4().to_string();
    let command = build_event_command(&app_id, &session_id, event_id.clone(), request, trace);
    let seq = persist_event_command(
        state.persist.event_log.as_ref(),
        &app_id,
        &session_id,
        &command,
    )
    .await;

    info!(
        app_id = %app_id,
        session_id = %session_id,
        event_id = %event_id,
        seq,
        "GenUI event command persisted"
    );
    Ok(Json(GenUiEventResponse {
        event_id,
        seq,
        persisted: true,
    }))
}

/// Build the protocol command from the route request.
///
/// Keeping command construction in a small helper applies the Command pattern
/// explicitly and makes the route handler a transport adapter.  The helper is
/// intentionally application-agnostic: it copies only schema scope, component,
/// action, payload, and trace data into the protocol object.
fn build_event_command(
    app_id: &str,
    session_id: &str,
    event_id: String,
    request: GenUiEventRequest,
    trace: TraceContext,
) -> UiEventCommand {
    UiEventCommand::new(UiEvent {
        event_id,
        app_id: app_id.to_string(),
        session_id: session_id.to_string(),
        surface_id: UiRenderSurface::new(request.surface_id),
        component_id: request.component_id,
        action: request.action,
        payload: request.payload,
        trace: Some(trace),
        metadata: Default::default(),
    })
}

/// Persist a GenUI event command into the shared session EventLog.
///
/// This helper is the audit boundary for UI interactions.  The event is durable
/// before the route returns, and subscribers receive the normal EventLog
/// notification path used by session trace replay.
async fn persist_event_command(
    event_log: &EventLog,
    app_id: &str,
    session_id: &str,
    command: &UiEventCommand,
) -> u64 {
    event_log
        .append_command(
            AppendEventCommand::new(
                session_id,
                "genui_event",
                "genui_web_shell",
                serde_json::to_value(command).unwrap_or_else(|_| json!({})),
            )
            .with_app_id(app_id.to_string()),
        )
        .await
}

/// Validate app/session route scope.
fn required_scope(
    app_id: &str,
    session_id: Option<&str>,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    if app_id.trim().is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            UiRenderError::MissingApplicationId.to_string(),
        ));
    }
    let Some(session_id) = session_id.map(str::trim).filter(|value| !value.is_empty()) else {
        warn!(app_id = %app_id, "GenUI route rejected missing session scope");
        return Err(err(
            StatusCode::BAD_REQUEST,
            UiRenderError::MissingSessionId.to_string(),
        ));
    };
    Ok(session_id.to_string())
}

fn parse_application_id(
    app_id: &str,
) -> Result<macaca_proto::ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    let parsed = Uuid::parse_str(app_id).map_err(|_| {
        err(
            StatusCode::BAD_REQUEST,
            "app_id must be a valid application UUID".into(),
        )
    })?;
    Ok(macaca_proto::ApplicationId(parsed))
}

/// Create a trace context for shell-created GenUI route commands.
fn trace_for_genui(session_id: &str, trace_id: &str) -> TraceContext {
    let mut trace = TraceContext::new(trace_id);
    trace.session_id = Some(session_id.to_string());
    trace
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use macaca_runtime_host::persist::{EventLogQuery, RedbStore};

    #[test]
    fn genui_required_scope_rejects_missing_session() {
        let err = required_scope("app", None).unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn genui_required_scope_accepts_trimmed_session() {
        let session = match required_scope("app", Some(" session-a ")) {
            Ok(session) => session,
            Err(_) => panic!("expected GenUI scope validation to accept non-empty session"),
        };
        assert_eq!(session, "session-a");
    }

    #[tokio::test]
    async fn genui_event_command_is_persisted_to_event_log() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let store =
            Arc::new(RedbStore::open(dir.path().join("genui-events.redb")).expect("redb store"));
        let event_log = EventLog::new(Arc::clone(&store));
        let session_id = "session-ui-event";
        let request = GenUiEventRequest {
            session_id: session_id.to_string(),
            surface_id: "main".to_string(),
            component_id: "submit".to_string(),
            action: UiAction {
                name: "click".to_string(),
                label: "Click".to_string(),
                payload: json!({"kind": "test"}),
                metadata: Default::default(),
            },
            payload: json!({"value": "ok"}),
            trace_id: Some("trace-ui-event".to_string()),
        };
        let trace = trace_for_genui(session_id, "trace-ui-event");
        let command =
            build_event_command("app-ui", session_id, "event-1".to_string(), request, trace);

        let seq = persist_event_command(&event_log, "app-ui", session_id, &command).await;
        let events = event_log
            .query_indexed(
                EventLogQuery::new(session_id).event_type(Some("genui_event".to_string())),
            )
            .await;

        assert_eq!(seq, 1);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "genui_event");
        assert_eq!(events[0].source, "genui_web_shell");
        assert_eq!(events[0].payload["event"]["component_id"], "submit");
    }
}
