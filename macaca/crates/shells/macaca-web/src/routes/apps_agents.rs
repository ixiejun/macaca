//! Per-application agent listing and SSE status stream routes.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::Json;
use serde::Serialize;

use crate::application_shell_adapter::{app_runtime_agent_ids, select_app_scoped_agent_manifests};
use crate::state::AppState;

use super::apps::{service_metadata_view, service_status_views};
use super::shared::{
    app_entry_agent_name, app_has_active_session, entry_agent_activity_override, AgentStatusQuery,
    ErrorResponse,
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

/// Application-scoped runtime projection used by both polling and SSE routes.
///
/// The Application Service owns the sanitized list of agent names declared by
/// the application, while the kernel owns live runtime status keyed by
/// `AgentId`.  This helper is the route-layer Adapter between those two
/// provider-neutral views: it first selects the manifests that belong to the
/// application boundary and then uses the selected ids to fetch runtime
/// activity.  Keeping the join in one place prevents the HTTP route and the SSE
/// route from drifting back to an empty-id status query, which would make every
/// working agent render as idle.
struct AppAgentRuntimeProjection {
    manifests: Vec<macaca_proto::AgentManifest>,
    statuses_by_id: HashMap<String, macaca_proto::AgentRuntimeStatus>,
}

impl AppAgentRuntimeProjection {
    async fn load(
        state: &Arc<AppState>,
        app_id: &macaca_proto::ApplicationId,
        service_agent_names: &[String],
        trace_id: &'static str,
    ) -> Self {
        let runtime_agent_ids = app_runtime_agent_ids(state, app_id, trace_id).await;
        let manifests = select_app_scoped_agent_manifests(
            state.kernel.list_agents().await,
            &runtime_agent_ids,
            Some(service_agent_names),
        );
        let agent_ids = manifests
            .iter()
            .map(|manifest| manifest.id)
            .collect::<Vec<_>>();
        let statuses = state.kernel.list_agent_statuses_for(&agent_ids).await;
        let statuses_by_id: HashMap<String, macaca_proto::AgentRuntimeStatus> = statuses
            .into_iter()
            .map(|status| (status.agent_id.0.to_string(), status))
            .collect();

        tracing::info!(
            trace_id,
            declared_agent_count = service_agent_names.len(),
            runtime_agent_id_count = runtime_agent_ids.len(),
            selected_agent_count = manifests.len(),
            status_count = statuses_by_id.len(),
            "projected application-scoped agent runtime statuses"
        );

        Self {
            manifests,
            statuses_by_id,
        }
    }

    fn activity_for(
        &self,
        manifest: &macaca_proto::AgentManifest,
    ) -> (macaca_proto::AgentActivity, Option<String>) {
        self.statuses_by_id
            .get(&manifest.id.0.to_string())
            .map(|status| (status.activity.clone(), status.current_task.clone()))
            .unwrap_or_else(|| (macaca_proto::AgentActivity::Idle, None))
    }

    fn state_for(&self, manifest: &macaca_proto::AgentManifest) -> macaca_proto::AgentState {
        self.statuses_by_id
            .get(&manifest.id.0.to_string())
            .map(|status| status.state)
            .unwrap_or(manifest.state)
    }
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

    let Some(service_agent_names) = service_agent_names else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "application agents are unavailable".into(),
            }),
        ));
    };
    let projection = AppAgentRuntimeProjection::load(
        &state,
        &app_id,
        &service_agent_names,
        "web-route-app-agents-runtime-status",
    )
    .await;
    let has_active_session =
        app_has_active_session(&state, &app_id, query.session_id.as_deref()).await;
    let entry_agent_name = app_entry_agent_name(&state, &app_id).await;

    let agents: Vec<AgentInfo> = projection
        .manifests
        .iter()
        .map(|m| {
            let id_str = m.id.0.to_string();
            let (raw_activity, current_task) = projection.activity_for(m);
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
                state: format!("{:?}", projection.state_for(m)),
                activity,
                capabilities: m
                    .capabilities
                    .iter()
                    .map(|capability| capability.name.clone())
                    .collect(),
                is_active: projection.state_for(m) == macaca_proto::AgentState::Running,
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
            let Some(service_agent_names) = service_agent_names else {
                    yield Ok(Event::default()
                        .event("error")
                        .data(r#"{"error":"application agents are unavailable"}"#));
                    return;
            };
            let projection = AppAgentRuntimeProjection::load(
                &state_clone,
                &app_id,
                &service_agent_names,
                "web-route-app-agent-stream-runtime-status",
            ).await;
            let has_active_session =
                app_has_active_session(&state_clone, &app_id, scoped_session_id.as_deref()).await;
            let entry_agent_name = app_entry_agent_name(&state_clone, &app_id).await;

            // Build simplified status
            let agents: Vec<SimpleAgentStatus> = projection
                .manifests
                .iter()
                .map(|m| {
                    let id_str = m.id.0.to_string();
                    let (raw_activity, current_task) = projection.activity_for(m);
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
                        name: m.name.clone(),
                        status,
                        detail: detail.or(current_task),
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
