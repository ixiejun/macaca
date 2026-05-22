//! Application-scoped Heartbeat Operations route adapters.
//!
//! These handlers are intentionally thin shell adapters. They aggregate
//! sanitized Application Service heartbeat declaration views and native
//! Heartbeat Service mementos, then delegate profile mutations through the
//! focused SDK client. They never read raw manifests, edit HEARTBEAT.md, create
//! Scheduler jobs, or encode application-specific workflow semantics.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use macaca_proto::{
    ApplicationHeartbeatAgentsQueryCommand, ApplicationId, AutonomyScope, HeartbeatProfileId,
    HeartbeatQueryCommand, HeartbeatUpdateProfileCommand, TraceContext,
};
use serde::Deserialize;

use crate::routes::{err, proto_err, ErrorResponse};
use crate::state::AppState;

mod view;

/// Request body accepted by the profile policy update route.
///
/// Every field is optional except the route profile id. This lets operators
/// patch enabled state, fixed interval cadence, or metadata independently
/// while preserving Heartbeat-owned profile state for fields they did not edit.
#[derive(Debug, Deserialize)]
pub struct UpdateHeartbeatProfileRequest {
    pub enabled: Option<bool>,
    pub interval_secs: Option<u64>,
    pub cooldown_secs: Option<u64>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    pub reason_code: Option<String>,
}

/// GET /api/apps/{app_id}/autonomy/heartbeat/operations
/// returns a bounded Heartbeat Operations snapshot for one application.
pub async fn get_heartbeat_operations(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let trace = TraceContext::new(format!("web-heartbeat-operations-{}", app_id.0));
    let scope = AutonomyScope::application(app_id);

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        "web route aggregating application heartbeat operations"
    );

    let declarations = state
        .application_client
        .heartbeat_agents(
            ApplicationHeartbeatAgentsQueryCommand::application(trace.clone(), app_id)
                .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?,
        )
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let agent_scope_keys = declarations
        .iter()
        .map(|declaration| declaration.wake_scope_key.clone())
        .filter(|scope_key| !scope_key.is_empty())
        .collect::<BTreeSet<_>>();
    let snapshot = state
        .heartbeat_client
        .snapshot(HeartbeatQueryCommand {
            trace: trace.clone(),
            scope: scope.clone(),
            run_id: None,
            wake_scope_key: None,
            limit: Some(20),
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    let runs = state
        .heartbeat_client
        .list_runs(HeartbeatQueryCommand {
            trace: trace.clone(),
            scope,
            run_id: None,
            wake_scope_key: None,
            limit: Some(100),
        })
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;

    let profile_views = view::profiles(snapshot.native_profiles.clone(), &agent_scope_keys);
    let run_views = view::runs_for_scopes(runs, &agent_scope_keys, 20);
    let declaration_count = declarations.len();
    let profile_count = profile_views.len();
    let run_count = run_views.len();

    Ok(Json(serde_json::json!({
        "declarations": declarations,
        "profiles": profile_views,
        "runs": run_views,
        "snapshot": view::snapshot(snapshot, &agent_scope_keys),
        "count": {
            "declarations": declaration_count,
            "profiles": profile_count,
            "runs": run_count,
        },
        "trace_id": trace.trace_id,
    })))
}

/// PATCH /api/apps/{app_id}/autonomy/heartbeat/profiles/{profile_id}
/// updates Heartbeat-owned runtime profile policy.
pub async fn update_heartbeat_profile(
    State(state): State<Arc<AppState>>,
    Path((app_id, profile_id)): Path<(String, String)>,
    Json(body): Json<UpdateHeartbeatProfileRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let app_id = parse_application_id(&app_id)?;
    let profile_id = HeartbeatProfileId::new(profile_id)
        .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    let trace = TraceContext::new(format!("web-heartbeat-profile-update-{}", app_id.0));
    let mut command = HeartbeatUpdateProfileCommand::new(
        trace.clone(),
        AutonomyScope::application(app_id),
        profile_id,
        body.reason_code
            .unwrap_or_else(|| "web_heartbeat_profile_update".into()),
    )
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?
    .with_fixed_interval_ms(body.interval_secs.map(seconds_to_millis))
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?
    .with_cooldown_ms(body.cooldown_secs.map(seconds_to_millis))
    .map_err(|error| proto_err(StatusCode::BAD_REQUEST, &error))?;
    command.enabled = body.enabled;
    command.metadata = body.metadata;

    tracing::info!(
        app_id = %app_id.0,
        trace_id = %trace.trace_id,
        "web route updating native heartbeat profile through Heartbeat service"
    );
    let result = state
        .heartbeat_client
        .update_profile(command)
        .await
        .map_err(|error| proto_err(StatusCode::BAD_GATEWAY, &error))?;
    Ok(Json(view::mutation(result)))
}

fn parse_application_id(app_id: &str) -> Result<ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    uuid::Uuid::parse_str(app_id)
        .map(ApplicationId)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))
}

/// Convert operator seconds into Heartbeat command milliseconds safely.
///
/// The route is a shell Adapter, so it only normalizes transport input before
/// handing the typed command to `service.heartbeat`. `0` becomes one second for
/// ergonomic form handling, while extreme values saturate instead of panicking
/// inside the Web process.
fn seconds_to_millis(seconds: u64) -> u64 {
    seconds.max(1).saturating_mul(1_000)
}
