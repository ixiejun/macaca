//! Skills catalog, per-app skill snapshots, and MCP runtime status routes.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use macaca_proto::{
    MacacaError, McpProbeCommand, McpRuntimeStatusView, McpToolPolicySnapshot, TraceContext,
};

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
    let skills = state
        .config
        .catalog_entries
        .iter()
        .into_iter()
        .map(|e| SkillInfo {
            name: e.name.clone(),
            description: e.description.clone(),
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
    let app_id = macaca_proto::ApplicationId(app_uuid);
    let snapshots = state
        .app_skill_status_snapshots(app_id, query.agent)
        .await
        .map_err(|error| {
            let status = match error {
                MacacaError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            proto_err(status, &error)
        })?;
    let statuses = snapshots
        .into_iter()
        .map(|snapshot| AppSkillStatus {
            agent: snapshot.agent,
            visible: snapshot
                .visible
                .into_iter()
                .map(|skill| AppSkillInfo {
                    name: skill.name,
                    description: skill.description,
                    location: skill.location,
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
            mcp: snapshot.mcp,
            truncated: snapshot.truncated,
            compact: snapshot.compact,
        })
        .collect();

    Ok(Json(statuses))
}
