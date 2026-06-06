//! API route handlers for the Agent OS web server.
//!
//! Thin route handlers for system status, application management, todo/goal
//! queries, schedules, and event logs. Heavy-lifting modules:
//!
//! - [`crate::chat_orchestrator`] — SSE streaming, agentic loop, workflow
//! - [`crate::session`] — session CRUD, persistence, EventLog reconstruction
//! - [`crate::loop_manager`] — PlanLoop / WorkerLoop lifecycle
//! - [`crate::sse`] — SSE event conversion, broadcasting, plan decisions

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::Json;
use serde::{Deserialize, Serialize};

use macaca_app::{app_entry_agent_name as manifest_entry_agent_name, AppLoader};
use macaca_context::{
    CompactionSummaryEnvelope, ContextAdapterSafetyPolicy, ContextEngineInfo,
    ContextFallbackPolicy, LineageKind, ProviderHealthSnapshot, SessionLineage, TranscriptSegment,
};
use macaca_driver::{DriverInventoryCommand, DriverLoadServiceCommand, DriverServiceScope};
use macaca_persist::{AppendEventCommand, EventLogQuery, SessionLineageStore};
use macaca_proto::{
    AgentId, AgentManifest, ApplicationDiscoverCommand, ApplicationId,
    ApplicationMetadataQueryCommand, ApplicationMetadataView, ApplicationServiceAppView,
    ApplicationServiceScope, ApplicationStatusCommand, ApplicationUiRuntimeView, AutonomyScope,
    CancelScheduledAgentTaskCommand, CreateScheduledAgentTaskCommand, KernelServiceId, MacacaError,
    McpProbeCommand, McpRuntimeStatusView, McpToolPolicySnapshot, ProtoErrorAdapter,
    ScheduledAgentTaskId, ScheduledAgentTaskQueryCommand, ScheduledAgentTaskSchedule,
    ScheduledAgentTaskSummary, SchedulerDeleteJobCommand, SchedulerGetJobCommand,
    SchedulerJobDefinition, SchedulerJobId, SchedulerJobLifecycleCommand, SchedulerJobSummary,
    SchedulerLifecycleJobCommand, SchedulerListJobsCommand, SchedulerQueryCommand,
    SchedulerRegisterJobCommand, SchedulerRunSummary, SchedulerScheduleSpec,
    SchedulerTargetCommand, SchedulerUpdateJobCommand, ServiceCommandName, ServiceTargetCommand,
    TraceContext,
};
use macaca_skill::{SkillPolicy, SkillRuntimeFacade, SkillSnapshotRequest};

use crate::shell::WebShellFacade;
use crate::skill_mcp::SkillMcpStatus;
use crate::source_artifact::{
    ContextSourceArtifactRepository, SourceArtifactQuery, SourceArtifactResponse,
};
use crate::state::{AppState, ExternalAdapterRuntimeInstallation};

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

async fn app_has_active_session(
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

async fn app_entry_agent_name(state: &Arc<AppState>, app_id: &ApplicationId) -> Option<String> {
    match ApplicationMetadataQueryCommand::application(
        TraceContext::new("web-route-entry-agent-metadata"),
        *app_id,
    ) {
        Ok(command) => match state.application_client.metadata(command).await {
            Ok(view) => return view.entry.agent_name,
            Err(error) => tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Application metadata query failed; using deprecated raw manifest fallback"
            ),
        },
        Err(error) => tracing::warn!(
            app_id = %app_id,
            error = %error,
            "Application metadata query rejected before dispatch"
        ),
    }
    // Deprecated compatibility fallback: only kept for migration of existing
    // Web routes. New production behavior must use Application Service
    // metadata views rather than reading raw application manifests.
    #[allow(deprecated)]
    let registry = state.registry.read().await;
    registry
        .get_app(app_id)
        .and_then(|app| manifest_entry_agent_name(&app.manifest).map(str::to_string))
}

fn entry_agent_activity_override(
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

/// Select the application-scoped agent manifests that should be rendered by Web.
///
/// Application Service metadata is the authoritative app boundary for declared
/// agent names, but the legacy kernel registry can temporarily contain multiple
/// manifests with the same name during migration windows.  A pure name filter
/// would leak those global or stale agents into one application's UI.  This
/// helper implements a small Adapter over the legacy registry: it preserves the
/// Application Service order, returns at most one manifest per declared name,
/// and prefers manifests whose id is also bound to the application runtime.
fn select_app_scoped_agent_manifests(
    manifests: Vec<AgentManifest>,
    runtime_agent_ids: &[AgentId],
    service_agent_names: Option<&[String]>,
) -> Vec<AgentManifest> {
    let runtime_agent_ids: std::collections::HashSet<AgentId> =
        runtime_agent_ids.iter().copied().collect();

    if let Some(service_agent_names) = service_agent_names {
        let mut selected = Vec::with_capacity(service_agent_names.len());
        let mut consumed_names = std::collections::HashSet::new();

        for declared_name in service_agent_names {
            if !consumed_names.insert(declared_name.as_str()) {
                continue;
            }

            let preferred = manifests
                .iter()
                .find(|manifest| {
                    manifest.name == *declared_name && runtime_agent_ids.contains(&manifest.id)
                })
                .or_else(|| {
                    manifests
                        .iter()
                        .find(|manifest| manifest.name == *declared_name)
                });

            if let Some(manifest) = preferred {
                selected.push(manifest.clone());
            }
        }

        return selected;
    }

    manifests
        .into_iter()
        .filter(|manifest| runtime_agent_ids.contains(&manifest.id))
        .collect()
}

// ---------------------------------------------------------------------------
// GET /api/status
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub agent_count: usize,
    pub app_count: usize,
    pub llm_provider: String,
}

pub async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    tracing::info!("web status route entering thin system facade boundary");
    match state
        .system_facade
        .service_client()
        .inspect_services(
            &macaca_sdk::ServiceInspectionCommand::new("web-route-status-services")
                .expect("static service inspection scope is non-empty"),
        )
        .await
    {
        Ok(snapshot) => tracing::info!(
            services = snapshot.services.len(),
            "web status route inspected service runtime through facade bundle"
        ),
        Err(error) => tracing::warn!(
            error = %error,
            "web status route service inspection failed; preserving legacy response"
        ),
    }
    let agent_count = state.kernel.agent_count().await;
    let app_count = if let Some(views) = service_status_views(&state, "web-route-status-apps").await
    {
        views.len()
    } else {
        #[allow(deprecated)]
        {
            state.runtime.list_apps().await.len()
        }
    };

    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").into(),
        agent_count,
        app_count,
        llm_provider: state.llm.name().into(),
    })
}

// ---------------------------------------------------------------------------
// GET /api/apps
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub agent_count: usize,
    /// Manifest-declared entry agent.  Workspace clients treat this identity
    /// as the main thread and can hide it from delegated-agent tabs without
    /// hard-coding application-specific coordinator names.
    pub entry_agent: Option<String>,
    pub description: String,
    pub icon: String,
    pub ui: Option<ApplicationUiRuntimeView>,
}

fn app_info_from_service_view(view: &ApplicationServiceAppView) -> AppInfo {
    AppInfo {
        id: view.id.0.to_string(),
        name: view.name.clone(),
        status: view.runtime.compatibility_status.clone(),
        agent_count: view.agents.len(),
        entry_agent: view.entry_agent.clone(),
        description: view
            .description
            .clone()
            .unwrap_or_else(|| "An Agent OS application.".to_string()),
        icon: "cube".to_string(),
        ui: view.ui.clone(),
    }
}

fn app_info_from_metadata_view(view: &ApplicationMetadataView) -> AppInfo {
    app_info_from_service_view(&view.application)
}

async fn service_status_views(
    state: &Arc<AppState>,
    trace_id: &'static str,
) -> Option<Vec<ApplicationServiceAppView>> {
    let command = ApplicationStatusCommand {
        trace: TraceContext::new(trace_id),
        scope: ApplicationServiceScope::default(),
    };
    match state.application_client.status(command).await {
        Ok(views) => Some(views),
        Err(error) => {
            tracing::warn!(error = %error, trace_id, "Application Service status failed; falling back to legacy runtime");
            None
        }
    }
}

async fn service_metadata_view(
    state: &Arc<AppState>,
    app_id: ApplicationId,
    trace_id: &'static str,
) -> Option<ApplicationMetadataView> {
    let command =
        match ApplicationMetadataQueryCommand::application(TraceContext::new(trace_id), app_id) {
            Ok(command) => command,
            Err(error) => {
                tracing::warn!(error = %error, trace_id, "Application metadata command rejected");
                return None;
            }
        };
    match state.application_client.metadata(command).await {
        Ok(view) => Some(view),
        Err(error) => {
            tracing::warn!(
                error = %error,
                trace_id,
                "Application metadata query failed; preserving deprecated fallback"
            );
            None
        }
    }
}

pub async fn get_apps(State(state): State<Arc<AppState>>) -> Json<Vec<AppInfo>> {
    if let Some(views) = service_status_views(&state, "web-route-apps-status").await {
        tracing::info!(
            count = views.len(),
            "Application list served from Application Service"
        );
        return Json(views.iter().map(app_info_from_service_view).collect());
    }

    #[allow(deprecated)]
    let apps = state.runtime.list_apps().await;
    let agent_count = state.kernel.agent_count().await;
    #[allow(deprecated)]
    let registry = state.registry.read().await;

    let infos = apps
        .into_iter()
        .map(|(id, name, status)| {
            // Try to get description from registry (app.yaml)
            let (description, icon) = registry
                .get_app_by_name(&name)
                .map(|app| {
                    let desc = app
                        .manifest
                        .description
                        .as_deref()
                        .unwrap_or("An Agent OS application.");
                    (desc.to_string(), "cube".to_string())
                })
                .unwrap_or_else(|| ("An Agent OS application.".to_string(), "cube".to_string()));

            AppInfo {
                id: id.0.to_string(),
                name,
                status: format!("{:?}", status),
                agent_count,
                entry_agent: None,
                description,
                icon,
                ui: None,
            }
        })
        .collect();
    Json(infos)
}

// ---------------------------------------------------------------------------
// GET /api/apps/:id — Get single app info
// ---------------------------------------------------------------------------

pub async fn get_app(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Result<Json<AppInfo>, (StatusCode, Json<ErrorResponse>)> {
    let app_uuid: uuid::Uuid = app_id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid app_id".into(),
            }),
        )
    })?;
    let app_id = macaca_proto::ApplicationId(app_uuid);

    if let Some(view) = service_metadata_view(&state, app_id, "web-route-app-detail-metadata").await
    {
        tracing::info!(
            app_id = %app_id,
            "Application detail served from sanitized metadata view"
        );
        return Ok(Json(app_info_from_metadata_view(&view)));
    }

    if let Some(views) = service_status_views(&state, "web-route-app-detail-status").await {
        if let Some(view) = views.iter().find(|view| view.id == app_id) {
            tracing::info!(app_id = %app_id, "Application detail served from Application Service status");
            return Ok(Json(app_info_from_service_view(view)));
        }
    }

    // Get description from registry
    let description = {
        #[allow(deprecated)]
        let registry = state.registry.read().await;
        registry
            .get_app(&app_id)
            .map(|a| {
                a.manifest
                    .description
                    .clone()
                    .unwrap_or_else(|| "An Agent OS application.".to_string())
            })
            .unwrap_or_else(|| "An Agent OS application.".to_string())
    };
    let icon = "cube".to_string();

    #[allow(deprecated)]
    let apps = state.runtime.list_apps().await;
    for (id, name, status) in apps {
        if id == app_id {
            return Ok(Json(AppInfo {
                id: id.0.to_string(),
                name,
                status: format!("{:?}", status),
                agent_count: state.kernel.agent_count().await,
                entry_agent: None,
                description,
                icon,
                ui: None,
            }));
        }
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "App not found".into(),
        }),
    ))
}

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
    #[allow(deprecated)]
    let agent_ids = match state.runtime.app_agents(&app_id).await {
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
            #[allow(deprecated)]
            let agent_ids = match state_clone.runtime.app_agents(&app_id).await {
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

// ---------------------------------------------------------------------------
// POST /api/apps/reload — Reload apps from disk
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ReloadResponse {
    pub discovered_count: usize,
    pub apps: Vec<AppInfo>,
}

pub async fn reload_apps(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ReloadResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Reload the registry
    let discovered_count = match state
        .application_client
        .discover(
            ApplicationDiscoverCommand::new(TraceContext::new("web-route-apps-reload"))
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        )
        .await
    {
        Ok(discovered) => {
            tracing::info!(
                count = discovered.len(),
                "Application reload discovery served through Application Service"
            );
            discovered.len()
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                "Application Service reload failed; falling back to legacy registry"
            );
            #[allow(deprecated)]
            {
                let mut registry = state.registry.write().await;
                registry
                    .reload()
                    .map_err(|e| {
                        err(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to reload apps: {e}"),
                        )
                    })?
                    .len()
            }
        }
    };

    // Build app info from discovered apps
    if let Some(views) = service_status_views(&state, "web-route-apps-reload-status").await {
        return Ok(Json(ReloadResponse {
            discovered_count,
            apps: views.iter().map(app_info_from_service_view).collect(),
        }));
    }

    #[allow(deprecated)]
    let apps = state.runtime.list_apps().await;
    let agent_count = state.kernel.agent_count().await;

    #[allow(deprecated)]
    let registry = state.registry.read().await;
    let app_infos: Vec<AppInfo> = apps
        .into_iter()
        .map(|(id, name, status)| {
            let (description, icon) = registry
                .get_app_by_name(&name)
                .map(|app| {
                    let desc = app
                        .manifest
                        .description
                        .as_deref()
                        .unwrap_or("An Agent OS application.");
                    (desc.to_string(), "cube".to_string())
                })
                .unwrap_or_else(|| ("An Agent OS application.".to_string(), "cube".to_string()));
            AppInfo {
                id: id.0.to_string(),
                name,
                status: format!("{:?}", status),
                agent_count,
                entry_agent: None,
                description,
                icon,
                ui: None,
            }
        })
        .collect();
    drop(registry);

    Ok(Json(ReloadResponse {
        discovered_count,
        apps: app_infos,
    }))
}

// ---------------------------------------------------------------------------
// GET /api/skills
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
}

pub async fn get_skills(State(state): State<Arc<AppState>>) -> Json<Vec<SkillInfo>> {
    let catalog = state.config.catalog.read().await;
    let skills = catalog
        .catalog()
        .into_iter()
        .map(|e| SkillInfo {
            name: e.name,
            description: e.description,
        })
        .collect();
    Json(skills)
}

// ---------------------------------------------------------------------------
// GET /api/apps/:id/skills — Per-app/agent standard AgentSkills status
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
pub struct SkillStatusQuery {
    pub agent: Option<String>,
}

#[derive(Serialize)]
pub struct AppSkillStatus {
    pub agent: String,
    pub visible: Vec<AppSkillInfo>,
    pub filtered: Vec<AppFilteredSkillInfo>,
    pub mcp: Vec<SkillMcpStatus>,
    pub truncated: bool,
    pub compact: bool,
}

/// GET /api/mcp — Agent OS level MCP registry/runtime status.
pub async fn get_mcp_status(State(state): State<Arc<AppState>>) -> Json<Vec<McpRuntimeStatusView>> {
    let trace = TraceContext::new("web-route-mcp-status");
    let command = McpProbeCommand::new(trace, McpToolPolicySnapshot::default());
    match command {
        Ok(command) => match state.mcp_client.probe(command).await {
            Ok(result) => Json(result.statuses),
            Err(error) => {
                tracing::warn!(error = %error, "MCP service status failed; returning empty status");
                Json(Vec::new())
            }
        },
        Err(error) => {
            tracing::warn!(error = %error, "MCP service status command rejected");
            Json(Vec::new())
        }
    }
}

#[derive(Serialize)]
pub struct AppSkillInfo {
    pub name: String,
    pub description: String,
    pub location: String,
    pub source: String,
}

#[derive(Serialize)]
pub struct AppFilteredSkillInfo {
    pub name: String,
    pub reason: String,
    pub source: String,
}

pub async fn get_app_skills(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    Query(query): Query<SkillStatusQuery>,
) -> Result<Json<Vec<AppSkillStatus>>, (StatusCode, Json<ErrorResponse>)> {
    let app_uuid: uuid::Uuid = app_id
        .parse()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?;
    let app_id = ApplicationId(app_uuid);

    let app = {
        let registry = state.registry.read().await;
        registry
            .get_app(&app_id)
            .cloned()
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "App not found".into()))?
    };
    let agent_configs = AppLoader::resolve_agent_configs(&app.manifest, &app.path)
        .map_err(|e| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let workspace_root = {
        let workspaces = state.config.app_workspaces.read().await;
        workspaces.get(&app_id).map(|ws| ws.root.clone())
    };

    let mut statuses = Vec::new();
    for agent in agent_configs {
        if query
            .agent
            .as_deref()
            .is_some_and(|name| name != agent.name.as_str())
        {
            continue;
        }
        let policy = agent
            .skills
            .as_ref()
            .map(|skills| SkillPolicy {
                allow: skills.allow.clone(),
                deny: skills.deny.clone(),
            })
            .unwrap_or_default();
        let request = SkillSnapshotRequest::builder(agent.name.clone())
            .workspace_dir(workspace_root.clone())
            .app_dir(Some(app.path.clone()))
            .policy(policy)
            .build();
        let snapshot = SkillRuntimeFacade::new()
            .build_snapshot(request)
            .await
            .map_err(|e| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
        let mcp = crate::skill_mcp::probe_skill_mcp_servers(&snapshot).await;
        statuses.push(AppSkillStatus {
            agent: snapshot.agent,
            visible: snapshot
                .skills
                .into_iter()
                .map(|skill| AppSkillInfo {
                    name: skill.name,
                    description: skill.description,
                    location: skill.location.display().to_string(),
                    source: skill.source,
                })
                .collect(),
            filtered: snapshot
                .filtered
                .into_iter()
                .map(|skill| AppFilteredSkillInfo {
                    name: skill.name,
                    reason: skill.reason,
                    source: skill.source,
                })
                .collect(),
            mcp,
            truncated: snapshot.truncated,
            compact: snapshot.compact,
        });
    }

    Ok(Json(statuses))
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

// ---------------------------------------------------------------------------
// Todo Board API
// ---------------------------------------------------------------------------

/// Optional session_id filter for todo/goal queries.
#[derive(Deserialize, Default)]
pub struct SessionQuery {
    #[serde(default)]
    session_id: Option<String>,
}

fn required_session_id<'a>(
    query: &'a SessionQuery,
    route_name: &str,
) -> Result<&'a str, (StatusCode, Json<ErrorResponse>)> {
    let Some(session_id) = query.session_id.as_deref().map(str::trim) else {
        return Err(err(
            StatusCode::BAD_REQUEST,
            format!("{route_name} requires session_id"),
        ));
    };
    if session_id.is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            format!("{route_name} requires non-empty session_id"),
        ));
    }
    Ok(session_id)
}

/// GET /api/apps/{app_id}/todos — list todos for the current session only.
///
/// The Web UI Task Board is intentionally session-scoped. Requiring `session_id`
/// here prevents accidental application-wide scans when a caller forgets to pass
/// the current chat session id.
pub async fn list_todos(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let store = Arc::clone(&state.persist.todo_store);
    let session_id = required_session_id(&query, "list_todos")?;
    let shell = WebShellFacade::for_task_board(store);
    let response = shell
        .list_todos_json(app_id, session_id)
        .await
        .map_err(|error| err(StatusCode::BAD_REQUEST, error.to_string()))?;
    Ok(Json(response))
}

/// GET /api/apps/{app_id}/todos/claim-diagnostics — why workers may not claim `Pending` tasks (session-scoped).
///
/// Requires `session_id`: claim order is evaluated on [`TodoStore::list_all_todos_for_session`].
pub async fn get_todo_claim_diagnostics(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let Some(ref sid) = query.session_id else {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "session_id is required for claim diagnostics".into(),
        ));
    };
    let store = Arc::clone(&state.persist.todo_store);
    let todos = store.list_all_todos_for_session(&app_id, sid).await;
    let diag = macaca_task::diagnose_session_claims(&todos);
    Ok(Json(
        serde_json::to_value(&diag).unwrap_or(serde_json::json!({})),
    ))
}

/// GET /api/apps/{app_id}/todos/progress — overall progress (optionally filtered by session_id)
pub async fn get_todo_progress(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let store = Arc::clone(&state.persist.todo_store);
    let space = macaca_task::TaskSpace::for_session(app_id, query.session_id, store);
    let p = space.overall_progress().await;
    Ok(Json(serde_json::json!({
        "total": p.total, "pending": p.pending, "assigned": p.assigned,
        "in_progress": p.in_progress, "pending_review": p.pending_review,
        "needs_optimization": p.needs_optimization, "completed": p.completed,
        "blocked": p.blocked, "failed": p.failed, "cancelled": p.cancelled,
        "all_done": space.all_tasks_done().await,
    })))
}

/// GET /api/apps/{app_id}/todos/{agent_name} — list agent's board
pub async fn list_agent_todos(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, agent_name)): axum::extract::Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let store = Arc::clone(&state.persist.todo_store);
    let mut todos = store.list_agent_todos(&app_id, &None, &agent_name).await;
    todos.sort_by_key(|t| t.sequence_number);
    Ok(Json(
        serde_json::json!({ "agent": agent_name, "todos": todos, "count": todos.len() }),
    ))
}

/// GET /api/apps/{app_id}/goals — list goals (optionally filtered by session_id)
pub async fn list_goals(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let store = Arc::clone(&state.persist.todo_store);
    let goals = if let Some(ref sid) = query.session_id {
        store.list_goals_for_session(&app_id, sid).await
    } else {
        store.list_goals(&app_id).await
    };
    Ok(Json(
        serde_json::json!({ "goals": goals, "count": goals.len() }),
    ))
}

// ---------------------------------------------------------------------------
// Schedule routes
// GET    /api/apps/{app_id}/schedules           → list_schedules
// POST   /api/apps/{app_id}/schedules           → create_schedule
// GET    /api/apps/{app_id}/schedules/{id}      → get_schedule
// DELETE /api/apps/{app_id}/schedules/{id}      → delete_schedule
// PUT    /api/apps/{app_id}/schedules/{id}/toggle → toggle_schedule
// ---------------------------------------------------------------------------

/// GET /api/apps/{app_id}/schedules — list all schedules for an application
pub async fn list_schedules(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let scheduler = macaca_task::TaskScheduler::new(
        Arc::clone(&state.persist.session_store),
        macaca_task::SchedulerConfig::default(),
    );
    let entries = scheduler.list(&app_id).await;
    Ok(Json(
        serde_json::json!({ "schedules": entries, "count": entries.len() }),
    ))
}

/// POST /api/apps/{app_id}/schedules — create a new schedule
///
/// Body (JSON):
/// ```json
/// {
///   "name": "daily-report",
///   "cron_expr": "0 9 * * *",          // optional; use this OR interval_secs
///   "interval_secs": 3600,              // optional; use this OR cron_expr
///   "action": {
///     "kind": "create_goal",
///     "description": "Generate daily report"
///   }
/// }
/// ```
pub async fn create_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );

    let name = body["name"]
        .as_str()
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "Missing 'name' field".into()))?
        .to_owned();

    let action: macaca_task::ScheduleAction = serde_json::from_value(body["action"].clone())
        .map_err(|e| err(StatusCode::BAD_REQUEST, format!("Invalid 'action': {e}")))?;

    let entry = if let Some(expr) = body["cron_expr"].as_str() {
        macaca_task::ScheduleEntry::new_cron(app_id.clone(), name, expr, action)
    } else if let Some(secs) = body["interval_secs"].as_u64() {
        macaca_task::ScheduleEntry::new_interval(app_id.clone(), name, secs, action)
    } else {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "One of 'cron_expr' or 'interval_secs' is required".into(),
        ));
    };

    let scheduler = macaca_task::TaskScheduler::new(
        Arc::clone(&state.persist.session_store),
        macaca_task::SchedulerConfig::default(),
    );
    let created = scheduler.create(entry).await;

    // Lazily start Scheduler loop for this app if not already running
    {
        let handles = state.loops.scheduler_handles.read().await;
        if !handles.contains_key(&app_id) {
            drop(handles);
            let mut handles = state.loops.scheduler_handles.write().await;
            if !handles.contains_key(&app_id) {
                let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
                handles.insert(app_id.clone(), Arc::clone(&shutdown));

                let sched_runner = macaca_task::TaskScheduler::new(
                    Arc::clone(&state.persist.session_store),
                    macaca_task::SchedulerConfig::default(),
                );
                let (sched_event_tx, mut sched_event_rx) =
                    tokio::sync::mpsc::channel::<macaca_task::ScheduleEvent>(64);
                let app_id_sched = app_id.clone();

                tokio::spawn(async move {
                    sched_runner
                        .run(app_id_sched, shutdown, sched_event_tx)
                        .await;
                });

                let state_for_sched = Arc::clone(&state);
                let app_id_for_sched = app_id.clone();
                tokio::spawn(async move {
                    while let Some(event) = sched_event_rx.recv().await {
                        match event {
                            macaca_task::ScheduleEvent::Triggered { action, .. } => match action {
                                macaca_task::ScheduleAction::CreateGoal { description } => {
                                    let space = macaca_task::TaskSpace::for_session(
                                        app_id_for_sched.clone(),
                                        None,
                                        Arc::clone(&state_for_sched.persist.todo_store),
                                    );
                                    space.push_goal(description).await;
                                }
                                macaca_task::ScheduleAction::CreateTask {
                                    agent,
                                    title,
                                    description,
                                    priority,
                                } => {
                                    let space = macaca_task::TaskSpace::for_session(
                                        app_id_for_sched.clone(),
                                        None,
                                        Arc::clone(&state_for_sched.persist.todo_store),
                                    );
                                    space
                                        .create_task_assignment(
                                            &agent,
                                            "scheduler",
                                            &title,
                                            &description,
                                            vec![],
                                            priority,
                                            vec![],
                                            None,
                                        )
                                        .await;
                                }
                            },
                        }
                    }
                });

                tracing::info!(app_id = %app_id, "Scheduler started for app");
            }
        }
    }

    Ok(Json(serde_json::json!({
        "schedule_id": created.id.to_string(),
        "name": created.name,
        "next_run_at": created.next_run_at,
        "enabled": created.enabled,
    })))
}

/// GET /api/apps/{app_id}/schedules/{id} — get a single schedule
pub async fn get_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, schedule_id)): axum::extract::Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let id = macaca_proto::TaskId(
        uuid::Uuid::parse_str(&schedule_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid schedule_id".into()))?,
    );

    let scheduler = macaca_task::TaskScheduler::new(
        Arc::clone(&state.persist.session_store),
        macaca_task::SchedulerConfig::default(),
    );
    let entry = scheduler
        .get(&app_id, &id)
        .await
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "Schedule not found".into()))?;
    Ok(Json(serde_json::to_value(&entry).map_err(|e| {
        err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?))
}

/// DELETE /api/apps/{app_id}/schedules/{id} — delete a schedule
pub async fn delete_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, schedule_id)): axum::extract::Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let id = macaca_proto::TaskId(
        uuid::Uuid::parse_str(&schedule_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid schedule_id".into()))?,
    );

    let scheduler = macaca_task::TaskScheduler::new(
        Arc::clone(&state.persist.session_store),
        macaca_task::SchedulerConfig::default(),
    );
    scheduler.delete(&app_id, &id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/apps/{app_id}/schedules/{id}/toggle — enable or disable a schedule
///
/// Body: `{ "enabled": true }` or `{ "enabled": false }`
pub async fn toggle_schedule(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((app_id, schedule_id)): axum::extract::Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = ApplicationId(
        uuid::Uuid::parse_str(&app_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))?,
    );
    let id = macaca_proto::TaskId(
        uuid::Uuid::parse_str(&schedule_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid schedule_id".into()))?,
    );
    let enabled = body["enabled"].as_bool().ok_or_else(|| {
        err(
            StatusCode::BAD_REQUEST,
            "Missing boolean 'enabled' field".into(),
        )
    })?;

    let scheduler = macaca_task::TaskScheduler::new(
        Arc::clone(&state.persist.session_store),
        macaca_task::SchedulerConfig::default(),
    );
    let ok = scheduler.set_enabled(&app_id, &id, enabled).await;
    if ok {
        Ok(Json(
            serde_json::json!({ "schedule_id": schedule_id, "enabled": enabled }),
        ))
    } else {
        Err(err(StatusCode::NOT_FOUND, "Schedule not found".into()))
    }
}

// ---------------------------------------------------------------------------
// Serviceized application autonomy schedule routes
// GET/POST/PATCH/DELETE /api/apps/{app_id}/autonomy/schedules
// GET  /api/apps/{app_id}/autonomy/scheduler/runs
// ---------------------------------------------------------------------------

/// Request body for application-scoped serviceized autonomy schedule creation.
///
/// The route accepts only provider-neutral scheduling and target data.
/// Application-facing schedule creation requires an explicit Scheduler target;
/// native Heartbeat cadence is managed by `service.heartbeat`, not by default
/// Web-created Scheduler jobs.
#[derive(Debug, Deserialize)]
pub struct CreateAutonomyScheduleRequest {
    pub name: Option<String>,
    pub interval_secs: Option<u64>,
    pub target: Option<AutonomyScheduleTargetRequest>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

/// Request body for updating an existing serviceized Scheduler job.
///
/// The Web shell still does not own Scheduler semantics. It accepts
/// provider-neutral form data, rebuilds a typed `SchedulerJobDefinition`, and
/// delegates all state-machine decisions to `service.scheduler`.
#[derive(Debug, Deserialize)]
pub struct UpdateAutonomyScheduleRequest {
    pub name: Option<String>,
    pub interval_secs: Option<u64>,
    pub target: Option<AutonomyScheduleTargetRequest>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

/// Request body for pause/resume lifecycle operations.
#[derive(Debug, Deserialize)]
pub struct AutonomyScheduleLifecycleRequest {
    pub lifecycle: String,
    pub reason_code: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

/// Generic target accepted by the Web route before conversion to Scheduler DTOs.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutonomyScheduleTargetRequest {
    Service {
        service_id: String,
        command_name: String,
        #[serde(default)]
        metadata: BTreeMap<String, String>,
    },
}

#[derive(Debug, Deserialize, Default)]
pub struct AutonomyRunsQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
pub struct AutonomySchedulesQuery {
    pub limit: Option<usize>,
}

/// Request body for manual scheduled-agent-task creation.
///
/// Web accepts the prompt only as inbound command material and immediately
/// delegates it to `service.scheduled_agent_task`. It never stores the prompt,
/// renders it in list responses, or registers Scheduler jobs directly.
#[derive(Debug, Deserialize)]
pub struct CreateScheduledAgentTaskRequest {
    pub name: Option<String>,
    pub target_agent: String,
    pub task_prompt: String,
    pub interval_secs: Option<u64>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub delegated_context: serde_json::Value,
}

/// POST /api/apps/{app_id}/autonomy/schedules — register a serviceized Scheduler job.
pub async fn create_autonomy_schedule(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<CreateAutonomyScheduleRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-autonomy-schedule-register-{}", app_id.0));
    let definition = autonomy_definition_from_request(
        app_id,
        body.name,
        body.interval_secs,
        body.target,
        body.metadata,
    )?;

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        "web route registering application autonomy schedule through Scheduler service"
    );
    let result = state
        .scheduler_client
        .register_job(
            SchedulerRegisterJobCommand::new(trace.clone(), definition)
                .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?,
        )
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(serde_json::json!({
        "accepted": result.accepted,
        "job_id": result.job_id.as_ref().map(|id| id.as_str()),
        "run_id": result.run_id.as_ref().map(|id| id.as_str()),
        "lifecycle": result.lifecycle,
        "run_state": result.run_state,
        "audit_id": result.audit_id,
        "trace_id": result.trace.trace_id,
        "metadata": result.metadata,
    })))
}

/// GET /api/apps/{app_id}/autonomy/schedules — list serviceized Scheduler jobs.
pub async fn list_autonomy_schedules(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<AutonomySchedulesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-autonomy-schedule-list-{}", app_id.0));
    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        "web route listing application autonomy schedules through Scheduler service"
    );
    let jobs = state
        .scheduler_client
        .list_jobs(
            SchedulerListJobsCommand::new(
                trace.clone(),
                AutonomyScope::application(app_id),
                query.limit,
            )
            .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?,
        )
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let count = jobs.len();
    Ok(Json(serde_json::json!({
        "schedules": sanitized_scheduler_jobs(jobs),
        "count": count,
        "trace_id": trace.trace_id,
    })))
}

/// GET /api/apps/{app_id}/autonomy/schedules/{job_id} — read one serviceized job.
pub async fn get_autonomy_schedule(
    State(state): State<Arc<AppState>>,
    Path((app_id, job_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let job_id =
        SchedulerJobId::new(job_id).map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let trace = TraceContext::new(format!("web-autonomy-schedule-get-{}", app_id.0));
    let job = state
        .scheduler_client
        .get_job(
            SchedulerGetJobCommand::new(trace.clone(), AutonomyScope::application(app_id), job_id)
                .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?,
        )
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(serde_json::json!({
        "schedule": job.map(sanitized_scheduler_job),
        "trace_id": trace.trace_id,
    })))
}

/// PATCH /api/apps/{app_id}/autonomy/schedules/{job_id} — update one serviceized job.
pub async fn update_autonomy_schedule(
    State(state): State<Arc<AppState>>,
    Path((app_id, job_id)): Path<(String, String)>,
    Json(body): Json<UpdateAutonomyScheduleRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let job_id =
        SchedulerJobId::new(job_id).map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let trace = TraceContext::new(format!("web-autonomy-schedule-update-{}", app_id.0));
    let definition = autonomy_definition_from_request(
        app_id,
        body.name,
        body.interval_secs,
        body.target,
        body.metadata,
    )?;
    let result = state
        .scheduler_client
        .update_job(
            SchedulerUpdateJobCommand::new(
                trace.clone(),
                AutonomyScope::application(app_id),
                job_id,
                definition,
                "web_schedule_update",
            )
            .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?,
        )
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(sanitized_scheduler_mutation(result)))
}

/// DELETE /api/apps/{app_id}/autonomy/schedules/{job_id} — delete one serviceized job.
pub async fn delete_autonomy_schedule(
    State(state): State<Arc<AppState>>,
    Path((app_id, job_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let job_id =
        SchedulerJobId::new(job_id).map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let trace = TraceContext::new(format!("web-autonomy-schedule-delete-{}", app_id.0));
    let result = state
        .scheduler_client
        .delete_job(
            SchedulerDeleteJobCommand::new(
                trace.clone(),
                AutonomyScope::application(app_id),
                job_id,
                "web_schedule_delete",
            )
            .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?,
        )
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(sanitized_scheduler_mutation(result)))
}

/// PUT /api/apps/{app_id}/autonomy/schedules/{job_id}/lifecycle — pause/resume one job.
pub async fn transition_autonomy_schedule(
    State(state): State<Arc<AppState>>,
    Path((app_id, job_id)): Path<(String, String)>,
    Json(body): Json<AutonomyScheduleLifecycleRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let job_id =
        SchedulerJobId::new(job_id).map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let trace = TraceContext::new(format!("web-autonomy-schedule-lifecycle-{}", app_id.0));
    let lifecycle = SchedulerJobLifecycleCommand::parse(&body.lifecycle)
        .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let mut command = SchedulerLifecycleJobCommand::new(
        trace.clone(),
        AutonomyScope::application(app_id),
        job_id,
        lifecycle,
        body.reason_code
            .unwrap_or_else(|| "web_schedule_lifecycle".into()),
    )
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    command.metadata = body.metadata;
    let result = state
        .scheduler_client
        .transition_job(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(sanitized_scheduler_mutation(result)))
}

/// GET /api/apps/{app_id}/autonomy/scheduler/runs — monitor serviceized runs.
pub async fn list_autonomy_scheduler_runs(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<AutonomyRunsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-autonomy-scheduler-runs-{}", app_id.0));
    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        "web route querying application autonomy scheduler runs"
    );
    let runs = state
        .scheduler_client
        .list_runs(SchedulerQueryCommand {
            trace: trace.clone(),
            scope: AutonomyScope::application(app_id),
            job_id: None,
            run_id: None,
            limit: query.limit.or(Some(20)),
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let count = runs.len();
    Ok(Json(serde_json::json!({
        "runs": sanitized_scheduler_runs(runs),
        "count": count,
        "trace_id": trace.trace_id,
    })))
}

/// POST /api/apps/{app_id}/autonomy/scheduled-agent-tasks — create recurring agent intent.
pub async fn create_scheduled_agent_task(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(body): Json<CreateScheduledAgentTaskRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-scheduled-agent-task-create-{}", app_id.0));
    let mut metadata = body.metadata;
    if let Some(name) = body.name.filter(|value| !value.trim().is_empty()) {
        metadata.insert("schedule.name".into(), name);
    }
    tracing::info!(
        app_id = %app_id.0,
        target_agent = body.target_agent.as_str(),
        trace_id = %trace.trace_id,
        "web route creating scheduled agent task through service boundary"
    );
    let command = CreateScheduledAgentTaskCommand::new(
        trace.clone(),
        AutonomyScope::application(app_id),
        ScheduledAgentTaskSchedule::Every {
            interval_ms: body.interval_secs.unwrap_or(60).max(1) * 1_000,
        },
        body.target_agent,
        body.task_prompt,
    )
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?
    .with_delegated_context(body.delegated_context)
    .with_metadata(metadata);
    let result = state
        .scheduled_agent_task_client
        .create_task(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(sanitized_scheduled_agent_task_mutation(result)))
}

/// GET /api/apps/{app_id}/autonomy/scheduled-agent-tasks — list safe summaries.
pub async fn list_scheduled_agent_tasks(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<AutonomySchedulesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-scheduled-agent-task-list-{}", app_id.0));
    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        "web route listing scheduled agent tasks through service boundary"
    );
    let mut command = ScheduledAgentTaskQueryCommand::new(
        trace.clone(),
        AutonomyScope::application(app_id),
        None,
    )
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    command.limit = query.limit;
    let tasks = state
        .scheduled_agent_task_client
        .list_tasks(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let count = tasks.len();
    Ok(Json(serde_json::json!({
        "tasks": sanitized_scheduled_agent_tasks(tasks),
        "count": count,
        "trace_id": trace.trace_id,
    })))
}

/// GET /api/apps/{app_id}/autonomy/scheduled-agent-tasks/{task_id} — read one safe summary.
pub async fn get_scheduled_agent_task(
    State(state): State<Arc<AppState>>,
    Path((app_id, task_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let task_id = ScheduledAgentTaskId::new(task_id)
        .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let trace = TraceContext::new(format!("web-scheduled-agent-task-get-{}", app_id.0));
    let command = ScheduledAgentTaskQueryCommand::new(
        trace.clone(),
        AutonomyScope::application(app_id),
        Some(task_id),
    )
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let task = state
        .scheduled_agent_task_client
        .get_task(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(serde_json::json!({
        "task": task.map(sanitized_scheduled_agent_task),
        "trace_id": trace.trace_id,
    })))
}

/// DELETE /api/apps/{app_id}/autonomy/scheduled-agent-tasks/{task_id} — cancel one task.
pub async fn cancel_scheduled_agent_task(
    State(state): State<Arc<AppState>>,
    Path((app_id, task_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let task_id = ScheduledAgentTaskId::new(task_id)
        .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let trace = TraceContext::new(format!("web-scheduled-agent-task-cancel-{}", app_id.0));
    let command = CancelScheduledAgentTaskCommand::new(
        trace.clone(),
        AutonomyScope::application(app_id),
        task_id,
        "web_scheduled_agent_task_cancel",
    )
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let result = state
        .scheduled_agent_task_client
        .cancel_task(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(sanitized_scheduled_agent_task_mutation(result)))
}

fn parse_application_id(app_id: &str) -> Result<ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    uuid::Uuid::parse_str(app_id)
        .map(ApplicationId)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))
}

fn autonomy_definition_from_request(
    app_id: ApplicationId,
    name: Option<String>,
    interval_secs: Option<u64>,
    target: Option<AutonomyScheduleTargetRequest>,
    metadata: BTreeMap<String, String>,
) -> Result<SchedulerJobDefinition, (StatusCode, Json<ErrorResponse>)> {
    let mut definition = SchedulerJobDefinition::new(
        AutonomyScope::application(app_id),
        SchedulerScheduleSpec::Every {
            interval_ms: interval_secs.unwrap_or(60).max(1) * 1_000,
            anchor: None,
        },
        autonomy_target_from_request(app_id, target)?,
    )
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    definition.metadata = metadata;
    if let Some(name) = name.filter(|value| !value.trim().is_empty()) {
        definition.metadata.insert("schedule.name".into(), name);
    }
    Ok(definition)
}

fn autonomy_target_from_request(
    _app_id: ApplicationId,
    target: Option<AutonomyScheduleTargetRequest>,
) -> Result<SchedulerTargetCommand, (StatusCode, Json<ErrorResponse>)> {
    let Some(target) = target else {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "Scheduler target is required; heartbeat native cadence is managed separately".into(),
        ));
    };
    Ok(match target {
        AutonomyScheduleTargetRequest::Service {
            service_id,
            command_name,
            metadata,
        } => SchedulerTargetCommand::Service(ServiceTargetCommand {
            service_id: KernelServiceId::new(service_id),
            command_name: ServiceCommandName::new(command_name),
            payload_ref: None,
            metadata,
        }),
    })
}

fn sanitized_scheduler_runs(runs: Vec<SchedulerRunSummary>) -> Vec<serde_json::Value> {
    runs.into_iter()
        .map(|run| {
            serde_json::json!({
                "run_id": run.run_id.as_str(),
                "job_id": run.job_id.as_str(),
                "state": run.state,
                "scheduled_for": run.scheduled_for,
                "started_at": run.started_at,
                "finished_at": run.finished_at,
                "attempt": run.attempt,
                "trace_id": run.trace_id,
                "audit_id": run.audit_id,
                "safe_status": run.safe_status,
                "metadata": run.metadata,
            })
        })
        .collect()
}

fn sanitized_scheduler_jobs(jobs: Vec<SchedulerJobSummary>) -> Vec<serde_json::Value> {
    jobs.into_iter().map(sanitized_scheduler_job).collect()
}

fn sanitized_scheduler_job(job: SchedulerJobSummary) -> serde_json::Value {
    serde_json::json!({
        "job_id": job.job_id.as_str(),
        "scope": job.scope,
        "schedule": job.schedule,
        "target": job.target,
        "lifecycle": job.lifecycle,
        "metadata": job.metadata,
        "created_at": job.created_at,
        "updated_at": job.updated_at,
        "last_scheduled_at": job.last_scheduled_at,
        "trace_id": job.trace_id,
        "audit_id": job.audit_id,
    })
}

fn sanitized_scheduler_mutation(result: macaca_proto::SchedulerCommandResult) -> serde_json::Value {
    serde_json::json!({
        "accepted": result.accepted,
        "job_id": result.job_id.as_ref().map(|id| id.as_str()),
        "run_id": result.run_id.as_ref().map(|id| id.as_str()),
        "lifecycle": result.lifecycle,
        "run_state": result.run_state,
        "audit_id": result.audit_id,
        "trace_id": result.trace.trace_id,
        "metadata": result.metadata,
        "error": result.error,
    })
}

fn sanitized_scheduled_agent_tasks(
    tasks: Vec<ScheduledAgentTaskSummary>,
) -> Vec<serde_json::Value> {
    tasks
        .into_iter()
        .map(sanitized_scheduled_agent_task)
        .collect()
}

fn sanitized_scheduled_agent_task(task: ScheduledAgentTaskSummary) -> serde_json::Value {
    serde_json::json!({
        "task_id": task.task_id.as_str(),
        "scope": task.scope,
        "schedule": task.schedule,
        "target_agent": task.target_agent,
        "execution_intent": task.execution_intent,
        "scheduler_job_id": task.scheduler_job_id.as_ref().map(|id| id.as_str()),
        "payload_ref": task.payload_ref,
        "payload_digest": task.payload_digest,
        "redacted_summary": task.redacted_summary,
        "lifecycle_state": task.lifecycle_state,
        "last_run_id": task.last_run_id.as_ref().map(|id| id.as_str()),
        "last_result_status": task.last_result_status,
        "trace_id": task.trace_id,
        "audit_id": task.audit_id,
        "created_at": task.created_at,
        "updated_at": task.updated_at,
        "metadata": task.metadata,
    })
}

fn sanitized_scheduled_agent_task_mutation(
    result: macaca_proto::ScheduledAgentTaskCommandResult,
) -> serde_json::Value {
    serde_json::json!({
        "accepted": result.accepted,
        "task_id": result.task_id.as_ref().map(|id| id.as_str()),
        "scheduler_job_id": result.scheduler_job_id.as_ref().map(|id| id.as_str()),
        "scheduler_run_id": result.scheduler_run_id.as_ref().map(|id| id.as_str()),
        "payload_ref": result.payload_ref,
        "payload_digest": result.payload_digest,
        "trace_id": result.trace.trace_id,
        "audit_id": result.audit_id,
        "summary": result.summary.map(sanitized_scheduled_agent_task),
        "error": result.error,
        "metadata": result.metadata,
    })
}

// ---------------------------------------------------------------------------
// Event Log API
// ---------------------------------------------------------------------------

/// Query parameters for the events endpoint.
#[derive(Deserialize)]
pub struct EventsQuery {
    pub since: Option<u64>,
    pub limit: Option<usize>,
    pub source: Option<String>,
    pub agent: Option<String>,
    pub event_type: Option<String>,
}

/// Response payload for the events endpoint.
#[derive(Serialize)]
pub struct EventsResponse {
    pub events: Vec<macaca_proto::EventEntry>,
    pub latest_seq: u64,
}

/// GET /api/sessions/:id/events?since={seq}&limit={n}
///
/// Returns persisted events for a session. Clients can poll by passing the
/// last received `latest_seq` as `since` to get only new events.
pub async fn get_session_events(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<EventsQuery>,
) -> Result<Json<EventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let since = params.since.unwrap_or(0);
    let limit = params.limit.unwrap_or(500).clamp(1, 2000);
    let query = EventLogQuery::new(session_id.clone())
        .since(since)
        .limit(limit)
        .source(params.source)
        .agent(params.agent)
        .event_type(params.event_type);
    let events = state.persist.event_log.query_indexed(query).await;
    let latest_seq = state.persist.event_log.latest_seq(&session_id).await;
    Ok(Json(EventsResponse { events, latest_seq }))
}

/// GET /api/sessions/:id/run-trace?since={seq}&limit={n}
///
/// Returns only `event_type == "run_trace"` rows (pipeline checkpoints). Use this
/// to see where execution is without loading full SSE-style event streams.
pub async fn get_session_run_trace(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<EventsQuery>,
) -> Result<Json<EventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let since = params.since.unwrap_or(0);
    let limit_out = params.limit.unwrap_or(500).clamp(1, 2000);
    let query = EventLogQuery::new(session_id.clone())
        .since(since)
        .limit(limit_out)
        .event_type(Some("run_trace".to_string()));
    let events = state.persist.event_log.query_indexed(query).await;
    let latest_seq = state.persist.event_log.latest_seq(&session_id).await;
    Ok(Json(EventsResponse { events, latest_seq }))
}

/// GET /api/sessions/:id/context-reports?since={seq}&limit={n}&agent={agent}
///
/// Returns request-scoped context report summaries. Full prompt content is not
/// persisted by default; payloads contain source counts, token estimates, and hashes.
pub async fn get_session_context_reports(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<EventsQuery>,
) -> Result<Json<EventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let since = params.since.unwrap_or(0);
    let limit_out = params.limit.unwrap_or(100).clamp(1, 500);
    let query = EventLogQuery::new(session_id.clone())
        .since(since)
        .limit(limit_out)
        .agent(params.agent)
        .event_type(Some("context_report".to_string()));
    let events = state.persist.event_log.query_indexed(query).await;
    let latest_seq = state.persist.event_log.latest_seq(&session_id).await;
    Ok(Json(EventsResponse { events, latest_seq }))
}

/// GET /api/sessions/:id/source-artifact?ref={source_ref}
///
/// Follows a context-report `source_ref` back to its canonical payload when the
/// reference is EventLog-backed. Unsupported refs return an explicit reason so
/// diagnostics UI can explain why retrieval is unavailable.
pub async fn get_session_source_artifact(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(params): Query<SourceArtifactQuery>,
) -> Result<Json<SourceArtifactResponse>, (StatusCode, Json<ErrorResponse>)> {
    if params.source_ref.trim().is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "source ref query parameter is required".to_string(),
        ));
    }

    let repository = ContextSourceArtifactRepository::new(Arc::clone(&state.persist.event_log));
    Ok(Json(
        repository.resolve(&session_id, &params.source_ref).await,
    ))
}

#[derive(Debug, Deserialize)]
pub struct ManualCompactRequest {
    #[serde(default)]
    pub focus_topic: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ManualCompactResponse {
    pub root_session_id: String,
    pub source_session_id: String,
    pub successor_session_id: String,
    pub source_segment_id: String,
    pub successor_segment_id: String,
    pub summary: String,
}

pub async fn manual_compact_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<ManualCompactRequest>,
) -> Result<Json<ManualCompactResponse>, (StatusCode, Json<ErrorResponse>)> {
    let lineage_store = SessionLineageStore::new(Arc::clone(&state.persist.session_store));
    let existing = lineage_store
        .load_lineage(&session_id)
        .await
        .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let root_session_id = existing
        .as_ref()
        .map(|lineage| lineage.root_session_id.clone())
        .unwrap_or_else(|| session_id.clone());
    if existing.is_none() {
        lineage_store
            .save_lineage(&SessionLineage::root(&session_id))
            .await
            .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    }

    let successor_session_id = uuid::Uuid::new_v4().to_string();
    let source_segment_id = format!("segment-{session_id}");
    let successor_segment_id = format!("segment-{successor_session_id}");
    let active_task = request
        .focus_topic
        .clone()
        .unwrap_or_else(|| "Continue from latest session state.".to_string());
    let envelope = CompactionSummaryEnvelope {
        root_session_id: root_session_id.clone(),
        source_segment_id: source_segment_id.clone(),
        successor_segment_id: successor_segment_id.clone(),
        resolved: vec!["Manual compaction requested.".into()],
        decisions: vec!["Created successor lineage without deleting source session.".into()],
        current_state: "Original session history remains available for audit.".into(),
        open_questions: Vec::new(),
        active_task,
        important_ids_and_paths: vec![session_id.clone(), successor_session_id.clone()],
    };
    let successor = SessionLineage::successor(
        root_session_id.clone(),
        session_id.clone(),
        successor_session_id.clone(),
    );
    lineage_store
        .save_lineage(&successor)
        .await
        .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    lineage_store
        .save_segment(&TranscriptSegment {
            segment_id: successor_segment_id.clone(),
            session_id: successor_session_id.clone(),
            predecessor_segment_id: Some(source_segment_id.clone()),
            lineage: successor,
        })
        .await
        .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let summary = envelope.render_reference_only();
    state
        .persist
        .event_log
        .append_command(AppendEventCommand::new(
            &session_id,
            "context_compaction",
            "context",
            serde_json::json!({
                "root_session_id": root_session_id,
                "source_session_id": session_id,
                "successor_session_id": successor_session_id,
                "source_segment_id": source_segment_id,
                "successor_segment_id": successor_segment_id,
                "focus_topic": request.focus_topic,
                "summary": summary.clone(),
            }),
        ))
        .await;
    state
        .persist
        .event_log
        .append_command(AppendEventCommand::new(
            &successor_session_id,
            "context_lineage_updated",
            "context",
            serde_json::json!({
                "root_session_id": envelope.root_session_id,
                "parent_session_id": session_id,
                "successor_session_id": successor_session_id,
                "lineage_kind": LineageKind::CompactionSuccessor,
            }),
        ))
        .await;

    Ok(Json(ManualCompactResponse {
        root_session_id: envelope.root_session_id,
        source_session_id: session_id,
        successor_session_id,
        source_segment_id,
        successor_segment_id,
        summary,
    }))
}

#[derive(Debug, Serialize)]
pub struct SessionLineageResponse {
    pub requested_session_id: String,
    pub lineage_tip_session_id: String,
    pub lineage: Vec<SessionLineage>,
}

pub async fn get_session_lineage(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionLineageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let lineage_store = SessionLineageStore::new(Arc::clone(&state.persist.session_store));
    let lineage = lineage_store
        .load_lineage(&session_id)
        .await
        .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let root = lineage
        .as_ref()
        .map(|lineage| lineage.root_session_id.clone())
        .unwrap_or_else(|| session_id.clone());
    let tip = lineage_store
        .resolve_tip(&session_id)
        .await
        .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let lineage = lineage_store
        .list_root_lineage(&root)
        .await
        .map_err(|error| err(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(SessionLineageResponse {
        requested_session_id: session_id,
        lineage_tip_session_id: tip,
        lineage,
    }))
}

// ---------------------------------------------------------------------------
// GET /api/drivers — List loaded drivers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct DriverInfo {
    pub name: String,
    pub version: String,
    pub driver_type: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub tools_count: usize,
}

#[derive(Serialize)]
pub struct DriversResponse {
    pub drivers: Vec<DriverInfo>,
    pub total: usize,
}

pub async fn get_drivers(State(state): State<Arc<AppState>>) -> Json<DriversResponse> {
    let command = DriverInventoryCommand {
        trace: TraceContext::new("web-route-driver-inventory"),
        scope: DriverServiceScope::default(),
        include_tools: true,
    };
    let result = state.driver_client.inventory(command).await;
    let drivers: Vec<DriverInfo> = result
        .map(|inventory| {
            inventory
                .entries
                .into_iter()
                .map(|item| DriverInfo {
                    name: item.name,
                    version: item.version.unwrap_or_default(),
                    driver_type: item.driver_type,
                    description: item.description,
                    capabilities: item.capabilities,
                    tools_count: item.tool_count,
                })
                .collect()
        })
        .unwrap_or_else(|error| {
            tracing::warn!(error = %error, "Driver service inventory failed; returning empty list");
            Vec::new()
        });
    let total = drivers.len();
    Json(DriversResponse { drivers, total })
}

// ---------------------------------------------------------------------------
// POST /api/drivers/reload — Rescan and reload drivers directory
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct DriverReloadResult {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct DriverReloadResponse {
    pub loaded: usize,
    pub failed: usize,
    pub results: Vec<DriverReloadResult>,
}

pub async fn reload_drivers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DriverReloadResponse>, (StatusCode, Json<ErrorResponse>)> {
    let report = state
        .driver_client
        .reload(
            DriverLoadServiceCommand::new(TraceContext::new("web-route-driver-reload"), true)
                .map_err(|e| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?,
        )
        .await
        .map_err(|e| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let mut results = Vec::new();

    for entry in &report.entries {
        match entry.status {
            macaca_driver::DriverLoadStatus::Loaded => {
                tracing::info!(
                    driver = %entry.name,
                    tools = entry.tool_count.unwrap_or_default(),
                    "Driver reloaded; tools will be available to agents on next execution"
                );
                results.push(DriverReloadResult {
                    name: entry.name.clone(),
                    status: "ok".to_string(),
                    error: None,
                });
            }
            macaca_driver::DriverLoadStatus::Failed => {
                results.push(DriverReloadResult {
                    name: entry.name.clone(),
                    status: "error".to_string(),
                    error: entry.error.clone(),
                });
            }
        }
    }

    Ok(Json(DriverReloadResponse {
        loaded: report.loaded,
        failed: report.failed,
        results,
    }))
}

/// Operator snapshot: built-in family descriptors, registry plugins, and rolling health — **no** prompt bodies.
pub async fn get_context_provider_runtime(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let builtin = macaca_context::list_builtin_family_descriptors();
    let builtin_engines = macaca_context::list_builtin_engine_infos();
    let builtin_engine_ids = builtin_engine_id_set(&builtin_engines);
    let registry_rows = state
        .context_provider_registry
        .list_registered_descriptors();
    let registry_engine_rows = state.context_engine_registry.list_engine_infos();
    state
        .external_adapter_runtime_registry
        .sync_registry_overlay_engines(&builtin_engine_ids, &registry_engine_rows)
        .await;
    let external_adapter_installations = state.external_adapter_runtime_registry.snapshot().await;
    let health = state.provider_health_ledger.snapshot();
    let external_adapter_runtime =
        external_adapter_runtime_rows(&external_adapter_installations, &health);
    let context_cfg = &state.config.context;
    Json(serde_json::json!({
        "default_engine": context_cfg.default_engine,
        "fallback_engine": context_cfg.fallback_engine,
        "emit_reports": context_cfg.emit_reports,
        "configured_provider_families": context_cfg.provider_families,
        "knowledge_digest_enabled": context_cfg.knowledge_digest.enabled,
        "active_vector_memory_enabled": context_cfg.active_vector_memory.enabled,
        "preflight_recall_enabled": context_cfg.recall.preflight_recall_enabled,
        "default_external_adapter_safety_policy": ContextAdapterSafetyPolicy::default(),
        "default_external_adapter_fallback_policy": ContextFallbackPolicy::default(),
        "builtin_engine_descriptors": builtin_engines,
        "registry_engine_descriptors": registry_engine_rows,
        "registry_engine_ids": state.context_engine_registry.list_engine_ids(),
        "external_adapter_runtime": external_adapter_runtime,
        "builtin_family_descriptors": builtin,
        "registry_family_descriptors": registry_rows,
        "registry_family_ids": state.context_provider_registry.list_family_ids(),
        "health": health,
    }))
}

fn builtin_engine_id_set(
    builtin_engines: &[ContextEngineInfo],
) -> std::collections::HashSet<String> {
    builtin_engines
        .iter()
        .map(|engine| engine.id.clone())
        .collect()
}

fn external_adapter_runtime_rows(
    installations: &[ExternalAdapterRuntimeInstallation],
    health: &HashMap<String, ProviderHealthSnapshot>,
) -> Vec<serde_json::Value> {
    let mut rows: Vec<_> = installations
        .iter()
        .map(|installation| {
            let last_health = health.get(&installation.engine.id).cloned();
            let runtime_status = if last_health.is_some() {
                "observed_via_health_ledger".to_string()
            } else {
                installation.runtime_state.clone()
            };
            serde_json::json!({
                "engine": installation.engine,
                "transport": installation.transport,
                "installation_source": installation.installation_source,
                "runtime_status": runtime_status,
                "default_safety_policy": installation.default_safety_policy,
                "default_fallback_policy": installation.default_fallback_policy,
                "circuit_breaker": {
                    "configured_failures": installation.default_safety_policy.circuit_breaker_failures,
                    "runtime_state": installation.circuit_breaker_runtime_state,
                },
                "last_sync_epoch_ms": installation.last_sync_epoch_ms,
                "last_health": last_health,
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        let a_id = a["engine"]["id"].as_str().unwrap_or_default();
        let b_id = b["engine"]["id"].as_str().unwrap_or_default();
        a_id.cmp(b_id)
    });
    rows
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::source_artifact::resolve_source_artifact_ref;
    use chrono::Utc;
    use macaca_context::{
        ContextAdapterSafetyPolicy, ContextEngineInfo, ContextFallbackPolicy,
        ProviderHealthSnapshot,
    };
    use macaca_proto::{
        AgentId, AgentManifest, AgentState, Capability, Permission, PermissionLevel,
    };

    use super::{
        external_adapter_runtime_rows, required_session_id, select_app_scoped_agent_manifests,
        SessionQuery,
    };
    use crate::state::ExternalAdapterRuntimeInstallation;

    fn test_agent_manifest(name: &str, capability: &str) -> AgentManifest {
        AgentManifest {
            id: AgentId::new(),
            name: name.to_string(),
            capabilities: vec![Capability {
                name: capability.to_string(),
                description: String::new(),
            }],
            permission: Permission {
                level: PermissionLevel::User,
                allowed_tools: Vec::new(),
                allowed_paths: Vec::new(),
                network_access: false,
            },
            state: AgentState::Created,
            created_at: Utc::now(),
            model: String::new(),
        }
    }

    #[test]
    fn app_scoped_agent_selection_deduplicates_legacy_same_name_agents() {
        let legacy_coordinator = test_agent_manifest("coordinator", "todo_goal_management");
        let app_coordinator = test_agent_manifest("coordinator", "coding_session_coordination");
        let planner = test_agent_manifest("planner", "code_change_planning");
        let coder = test_agent_manifest("coder", "patch_authoring");
        let reviewer = test_agent_manifest("reviewer", "structured_review");
        let legacy_planner = test_agent_manifest("planner", "todo_planning");
        let runtime_ids = vec![app_coordinator.id, planner.id, coder.id, reviewer.id];
        let declared_names = vec![
            "coordinator".to_string(),
            "planner".to_string(),
            "coder".to_string(),
            "reviewer".to_string(),
        ];

        let selected = select_app_scoped_agent_manifests(
            vec![
                legacy_coordinator,
                planner.clone(),
                coder.clone(),
                reviewer.clone(),
                legacy_planner,
                app_coordinator.clone(),
            ],
            &runtime_ids,
            Some(&declared_names),
        );

        assert_eq!(selected.len(), 4);
        assert_eq!(
            selected
                .iter()
                .map(|agent| agent.name.as_str())
                .collect::<Vec<_>>(),
            vec!["coordinator", "planner", "coder", "reviewer"]
        );
        assert_eq!(selected[0].id, app_coordinator.id);
        assert_eq!(
            selected[0].capabilities[0].name,
            "coding_session_coordination"
        );
    }

    #[test]
    fn resolves_short_event_ref_against_requested_session() {
        let resolved = resolve_source_artifact_ref("session-a", "event/42").unwrap();
        assert_eq!(resolved.session_id, "session-a");
        assert_eq!(resolved.seq, 42);
        assert_eq!(resolved.canonical_ref, "events/session-a/00000042");
    }

    #[test]
    fn resolves_canonical_event_ref_when_session_matches() {
        let resolved =
            resolve_source_artifact_ref("session-a", "events/session-a/00000007").unwrap();
        assert_eq!(resolved.session_id, "session-a");
        assert_eq!(resolved.seq, 7);
        assert_eq!(resolved.canonical_ref, "events/session-a/00000007");
    }

    #[test]
    fn rejects_cross_session_refs() {
        let error =
            resolve_source_artifact_ref("session-a", "events/session-b/00000007").unwrap_err();
        assert!(error.contains("cross-session"));
    }

    #[test]
    fn unsupported_provider_refs_return_error_message() {
        let error = resolve_source_artifact_ref("session-a", "workspace-memory").unwrap_err();
        assert!(error.contains("not backed by EventLog retrieval"));
    }

    #[test]
    fn external_adapter_runtime_rows_exclude_builtin_engines() {
        let rows = external_adapter_runtime_rows(
            &[ExternalAdapterRuntimeInstallation {
                engine: ContextEngineInfo::new("custom-external", "Custom External Engine"),
                transport: "registry_overlay".into(),
                installation_source: "context_engine_registry_overlay".into(),
                runtime_state: "installed".into(),
                default_safety_policy: ContextAdapterSafetyPolicy::default(),
                default_fallback_policy: ContextFallbackPolicy::default(),
                circuit_breaker_runtime_state: "not_implemented".into(),
                last_sync_epoch_ms: 7,
            }],
            &HashMap::new(),
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["engine"]["id"], "custom-external");
        assert_eq!(rows[0]["runtime_status"], "installed");
        assert_eq!(
            rows[0]["installation_source"],
            "context_engine_registry_overlay"
        );
        assert_eq!(
            rows[0]["circuit_breaker"]["runtime_state"],
            "not_implemented"
        );
    }

    #[test]
    fn external_adapter_runtime_rows_attach_last_health_when_present() {
        let mut health = HashMap::new();
        health.insert(
            "custom-external".to_string(),
            ProviderHealthSnapshot {
                outcome: "success".into(),
                latency_ms: 12,
                last_seen_epoch_ms: 123,
                implementation_version: Some("1.2.3".into()),
            },
        );
        let rows = external_adapter_runtime_rows(
            &[ExternalAdapterRuntimeInstallation {
                engine: ContextEngineInfo::new("custom-external", "Custom External Engine"),
                transport: "registry_overlay".into(),
                installation_source: "context_engine_registry_overlay".into(),
                runtime_state: "installed".into(),
                default_safety_policy: ContextAdapterSafetyPolicy::default(),
                default_fallback_policy: ContextFallbackPolicy::default(),
                circuit_breaker_runtime_state: "not_implemented".into(),
                last_sync_epoch_ms: 7,
            }],
            &health,
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["runtime_status"], "observed_via_health_ledger");
        assert_eq!(rows[0]["last_health"]["outcome"], "success");
        assert_eq!(rows[0]["last_health"]["implementation_version"], "1.2.3");
    }

    #[test]
    fn required_session_id_rejects_missing_or_blank_values() {
        let missing = required_session_id(&SessionQuery::default(), "list_todos").unwrap_err();
        assert_eq!(missing.0, axum::http::StatusCode::BAD_REQUEST);

        let blank = required_session_id(
            &SessionQuery {
                session_id: Some("  ".into()),
            },
            "list_todos",
        )
        .unwrap_err();
        assert_eq!(blank.0, axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn required_session_id_trims_and_accepts_current_session() {
        let query = SessionQuery {
            session_id: Some(" session-a ".into()),
        };
        let session_id = match required_session_id(&query, "list_todos") {
            Ok(session_id) => session_id,
            Err(_) => panic!("expected required_session_id to accept a non-empty session id"),
        };

        assert_eq!(session_id, "session-a");
    }
}
