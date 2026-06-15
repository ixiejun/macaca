//! Request parsing and response sanitizers for autonomy schedule routes.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use axum::Json;

use macaca_proto::{
    ApplicationId, AutonomyScope, KernelServiceId, ScheduledAgentTaskSummary,
    SchedulerJobDefinition, SchedulerJobSummary, SchedulerRunSummary, SchedulerScheduleSpec,
    SchedulerTargetCommand, ServiceCommandName, ServiceTargetCommand,
};
use serde::Deserialize;

use super::shared::{err, proto_err, ErrorResponse};

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

pub(crate) fn parse_application_id(
    app_id: &str,
) -> Result<ApplicationId, (StatusCode, Json<ErrorResponse>)> {
    uuid::Uuid::parse_str(app_id)
        .map(ApplicationId)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "Invalid app_id".into()))
}

pub(crate) fn autonomy_definition_from_request(
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

pub(crate) fn autonomy_target_from_request(
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

pub(crate) fn sanitized_scheduler_runs(runs: Vec<SchedulerRunSummary>) -> Vec<serde_json::Value> {
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

pub(crate) fn sanitized_scheduler_jobs(jobs: Vec<SchedulerJobSummary>) -> Vec<serde_json::Value> {
    jobs.into_iter().map(sanitized_scheduler_job).collect()
}

pub(crate) fn sanitized_scheduler_job(job: SchedulerJobSummary) -> serde_json::Value {
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

pub(crate) fn sanitized_scheduler_mutation(
    result: macaca_proto::SchedulerCommandResult,
) -> serde_json::Value {
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

pub(crate) fn sanitized_scheduled_agent_tasks(
    tasks: Vec<ScheduledAgentTaskSummary>,
) -> Vec<serde_json::Value> {
    tasks
        .into_iter()
        .map(sanitized_scheduled_agent_task)
        .collect()
}

pub(crate) fn sanitized_scheduled_agent_task(task: ScheduledAgentTaskSummary) -> serde_json::Value {
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

pub(crate) fn sanitized_scheduled_agent_task_mutation(
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
