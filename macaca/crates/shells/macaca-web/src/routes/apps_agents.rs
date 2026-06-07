//! Per-application agent listing and SSE status stream routes.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::Json;
use serde::Serialize;

use macaca_proto::ApplicationId;

use crate::state::AppState;

use super::apps::{service_metadata_view, service_status_views};
use super::shared::{
    app_entry_agent_name, app_has_active_session, entry_agent_activity_override,
    select_app_scoped_agent_manifests, AgentStatusQuery, ErrorResponse,
};

// ---------------------------------------------------------------------------
// GET /api/apps/:id/agents — Get agents for an app
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub state: String,
    /// Current activity (what the agent is doing right now).
    pub activity: AgentActivityInfo,
    pub capabilities: Vec<String>,
    pub is_active: bool,
    /// Current task description (if any).
    pub current_task: Option<String>,
}

/// Serializable agent activity info.
#[derive(Serialize)]
pub struct AgentActivityInfo {
    /// Activity type: idle, thinking, executing_tool, waiting, error.
    pub r#type: String,
    /// Additional context (tool name, thinking context, etc.).
    pub context: Option<String>,
    /// Secondary context (tool purpose, wait reason, etc.).
    pub detail: Option<String>,
}

impl From<macaca_proto::AgentActivity> for AgentActivityInfo {
    fn from(activity: macaca_proto::AgentActivity) -> Self {
        match activity {
            macaca_proto::AgentActivity::Idle => Self {
                r#type: "idle".into(),
                context: None,
                detail: None,
            },
            macaca_proto::AgentActivity::Working { context } => Self {
                r#type: "working".into(),
                context: Some(context),
                detail: None,
            },
            macaca_proto::AgentActivity::Error { message } => Self {
                r#type: "error".into(),
                context: Some(message),
                detail: None,
            },
            macaca_proto::AgentActivity::Thinking { context } => Self {
                r#type: "thinking".into(),
                context: Some(context),
                detail: None,
            },
        }
    }
}

pub async fn get_app_agents(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    Query(query): Query<AgentStatusQuery>,
) -> Result<Json<Vec<AgentInfo>>, (StatusCode, Json<ErrorResponse>)> {
    let app_uuid: uuid::Uuid = app_id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid app_id".into(),
            }),
        )
    })?;
    let app_id = macaca_proto::ApplicationId(app_uuid);

    let metadata_view =
        service_metadata_view(&state, app_id, "web-route-app-agents-metadata").await;
    let service_view = if let Some(view) = metadata_view.as_ref() {
        Some(view.application.clone())
    } else {
        service_status_views(&state, "web-route-app-agents-status")
            .await
            .and_then(|views| views.into_iter().find(|view| view.id == app_id))
    };
    let service_agent_names = service_view.as_ref().map(|view| {
        view.agents
            .iter()
            .map(|agent| agent.name.clone())
            .collect::<Vec<_>>()
    });

    // Get agent IDs for this app.  The service view is the preferred source
    // for app-scoped agent names; the legacy runtime id lookup remains the
    // compatibility fallback needed by existing kernel status APIs.
    let agent_ids = match crate::application_shell_adapter::app_agent_ids(&state, &app_id).await {
        Ok(ids) => ids,
        Err(error) if service_agent_names.is_some() => {
            tracing::warn!(
                app_id = %app_id,
                error = %error,
                "legacy app agent id lookup failed; using Application Service agent names"
            );
            Vec::new()
        }
        Err(e) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ));
        }
    };

    // Get manifests from kernel
    let manifests = state.kernel.list_agents().await;

    // Get runtime statuses
    let statuses = state.kernel.list_agent_statuses_for(&agent_ids).await;
    let status_map: std::collections::HashMap<String, _> = statuses
        .into_iter()
        .map(|s| (s.agent_id.0.to_string(), s))
        .collect();
    let has_active_session =
        app_has_active_session(&state, &app_id, query.session_id.as_deref()).await;
    let entry_agent_name = app_entry_agent_name(&state, &app_id).await;

    let agents: Vec<AgentInfo> =
        select_app_scoped_agent_manifests(manifests, &agent_ids, service_agent_names.as_deref())
            .into_iter()
            .map(|m| {
                let id_str = m.id.0.to_string();
                let (raw_activity, current_task) = status_map
                    .get(&id_str)
                    .map(|s| (s.activity.clone(), s.current_task.clone()))
                    .unwrap_or_else(|| (macaca_proto::AgentActivity::Idle, None));
                let activity = entry_agent_activity_override(
                    entry_agent_name.as_deref(),
                    &m.name,
                    has_active_session,
                    raw_activity,
                )
                .into();

                AgentInfo {
                    id: id_str,
                    name: m.name.clone(),
                    state: format!("{:?}", m.state),
                    activity,
                    capabilities: m.capabilities.into_iter().map(|c| c.name).collect(),
                    is_active: m.state == macaca_proto::AgentState::Running,
                    current_task,
                }
            })
            .collect();

    Ok(Json(agents))
}

// ---------------------------------------------------------------------------
// GET /api/apps/:id/agents/stream — SSE stream of agent status updates
// ---------------------------------------------------------------------------

/// Simplified agent status for frontend (IDLE, WORKING, ERROR)
#[derive(Serialize, Clone)]
pub struct SimpleAgentStatus {
    pub id: String,
    pub name: String,
    pub status: String, // "IDLE" | "WORKING" | "ERROR"
    pub detail: Option<String>,
}

pub async fn stream_agent_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    Query(query): Query<AgentStatusQuery>,
) -> Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>> {
    let app_uuid_result: Result<uuid::Uuid, _> = app_id.parse();
    let state_clone = Arc::clone(&state);
    let scoped_session_id = query.session_id.filter(|id| !id.is_empty());

    let stream = async_stream::stream! {
        // Handle parse error inside the stream
        let app_uuid = match app_uuid_result {
            Ok(u) => u,
            Err(_) => {
                yield Ok(Event::default().data(r#"{"error":"Invalid app_id"}"#));
                return;
            }
        };
        let app_id = macaca_proto::ApplicationId(app_uuid);

        loop {
            // Get agent IDs for this app
            let service_view = service_status_views(&state_clone, "web-route-app-agent-stream-status")
                .await
                .and_then(|views| views.into_iter().find(|view| view.id == app_id));
            let service_agent_names = service_view.as_ref().map(|view| {
                view.agents
                    .iter()
                    .map(|agent| agent.name.clone())
                    .collect::<Vec<_>>()
            });
            let agent_ids =
                match crate::application_shell_adapter::app_agent_ids(&state_clone, &app_id).await {
                Ok(ids) => ids,
                Err(_) if service_agent_names.is_some() => Vec::new(),
                Err(_) => {
                    yield Ok(Event::default()
                        .event("error")
                        .data(r#"{"error":"App not found"}"#));
                    return;
                }
            };

            // Get manifests
            let manifests = state_clone.kernel.list_agents().await;
            let statuses = state_clone.kernel.list_agent_statuses_for(&agent_ids).await;
            let status_map: std::collections::HashMap<String, _> = statuses
                .into_iter()
                .map(|s| (s.agent_id.0.to_string(), s))
                .collect();
            let has_active_session =
                app_has_active_session(&state_clone, &app_id, scoped_session_id.as_deref()).await;
            let entry_agent_name = app_entry_agent_name(&state_clone, &app_id).await;

            // Build simplified status
            let agents: Vec<SimpleAgentStatus> = select_app_scoped_agent_manifests(
                manifests,
                &agent_ids,
                service_agent_names.as_deref(),
            )
            .into_iter()
                .map(|m| {
                    let id_str = m.id.0.to_string();
                    let raw_activity = status_map.get(&id_str)
                        .map(|s| s.activity.clone())
                        .unwrap_or(macaca_proto::AgentActivity::Idle);
                    let activity = entry_agent_activity_override(
                        entry_agent_name.as_deref(),
                        &m.name,
                        has_active_session,
                        raw_activity,
                    );
                    let (status, detail) = match &activity {
                        macaca_proto::AgentActivity::Idle => ("IDLE".to_string(), None),
                        macaca_proto::AgentActivity::Working { context } => ("WORKING".to_string(), Some(context.clone())),
                        macaca_proto::AgentActivity::Thinking { context } => ("THINKING".to_string(), Some(context.clone())),
                        macaca_proto::AgentActivity::Error { message } => ("ERROR".to_string(), Some(message.clone())),
                    };

                    SimpleAgentStatus {
                        id: id_str,
                        name: m.name,
                        status,
                        detail,
                    }
                })
                .collect();

            let json = serde_json::to_string(&agents).unwrap_or_else(|_| "[]".to_string());
            yield Ok(Event::default().data(json));

            // Wait 500ms before next update
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    };

    Sse::new(stream)
}
