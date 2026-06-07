//! Serviceized autonomy schedule and scheduled-agent-task HTTP routes.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use macaca_proto::{
    ApplicationId, AutonomyScope, CancelScheduledAgentTaskCommand,
    CreateScheduledAgentTaskCommand, ScheduledAgentTaskId, ScheduledAgentTaskQueryCommand,
    ScheduledAgentTaskSchedule, SchedulerDeleteJobCommand, SchedulerGetJobCommand,
    SchedulerJobId, SchedulerJobLifecycleCommand, SchedulerLifecycleJobCommand,
    SchedulerListJobsCommand, SchedulerQueryCommand, SchedulerRegisterJobCommand,
    SchedulerUpdateJobCommand, TraceContext,
};

use crate::state::AppState;

use super::autonomy_sanitizers::{
    autonomy_definition_from_request, parse_application_id, sanitized_scheduled_agent_task,
    sanitized_scheduled_agent_task_mutation, sanitized_scheduled_agent_tasks,
    sanitized_scheduler_job, sanitized_scheduler_jobs, sanitized_scheduler_mutation,
    sanitized_scheduler_runs, AutonomyScheduleTargetRequest,
};
use super::shared::{err, proto_err, ErrorResponse};

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
