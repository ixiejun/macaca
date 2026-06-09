//! Skills catalog, per-app skill snapshots, and MCP runtime status routes.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use macaca_sdk::app::AppLoader;
use macaca_proto::{
    ApplicationId, McpProbeCommand, McpRuntimeStatusView, McpToolPolicySnapshot, ProtoErrorAdapter,
    TraceContext,
};
use macaca_sdk::skill::{SkillPolicy, SkillRuntimeFacade, SkillSnapshotRequest};

use crate::skill_mcp::SkillMcpStatus;
use crate::state::AppState;

use super::shared::{err, proto_err, ErrorResponse};

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
        let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
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
