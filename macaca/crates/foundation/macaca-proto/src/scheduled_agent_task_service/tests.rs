//! Contract tests for scheduled-agent-task service DTOs and command adapters.
//!
//! Extracted from the monolithic module so provider-neutral autonomy contracts stay under
//! the OS 500-line constitution while prompt-redaction invariants remain guarded.

use chrono::Utc;
use std::collections::BTreeMap;

use super::*;

/// Object Mother helper that builds a redacted summary without ever accepting a raw prompt.
///
/// Production providers should build summaries from payload refs; tests use this fixture to
/// assert serialization boundaries without leaking prompt material into audit surfaces.
fn redacted_fixture_for_tests(
    task_id: &str,
    payload_digest: &str,
    redacted_summary: &str,
) -> ScheduledAgentTaskSummary {
    let task_id = ScheduledAgentTaskId::new(task_id).unwrap();
    let mut payload_ref = AutonomyPayloadRef::new(
        format!("scheduled-agent-task://{}", task_id.as_str()),
        redacted_summary,
    )
    .unwrap();
    payload_ref.content_digest = Some(payload_digest.into());
    ScheduledAgentTaskSummary {
        task_id,
        scope: AutonomyScope::global(),
        schedule: ScheduledAgentTaskSchedule::Every {
            interval_ms: 60_000,
        },
        target_agent: "agent".into(),
        execution_intent: AgentExecutionIntent::TaskWorker,
        scheduler_job_id: None,
        payload_ref,
        payload_digest: Some(payload_digest.into()),
        redacted_summary: redacted_summary.into(),
        lifecycle_state: "active".into(),
        last_run_id: None,
        last_result_status: None,
        trace_id: Some("trace-summary".into()),
        audit_id: Some("audit.summary.1".into()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        metadata: BTreeMap::new(),
    }
}

#[test]
fn create_command_requires_trace_prompt_and_target_agent() {
    let command = CreateScheduledAgentTaskCommand::new(
        TraceContext::new("trace-scheduled-agent-task"),
        AutonomyScope::application(ApplicationId::from_name("scheduled-task-test")),
        ScheduledAgentTaskSchedule::Every {
            interval_ms: 60_000,
        },
        "technical_analyst",
        "Analyze the latest market state and record an auditable summary.",
    );
    assert!(command.is_ok());

    let missing_prompt = CreateScheduledAgentTaskCommand::new(
        TraceContext::new("trace-scheduled-agent-task"),
        AutonomyScope::application(ApplicationId::from_name("scheduled-task-test")),
        ScheduledAgentTaskSchedule::Every {
            interval_ms: 60_000,
        },
        "technical_analyst",
        " ",
    );
    assert!(missing_prompt.is_err());

    let missing_agent = CreateScheduledAgentTaskCommand::new(
        TraceContext::new("trace-scheduled-agent-task"),
        AutonomyScope::application(ApplicationId::from_name("scheduled-task-test")),
        ScheduledAgentTaskSchedule::Every {
            interval_ms: 60_000,
        },
        " ",
        "Analyze the latest market state and record an auditable summary.",
    );
    assert!(missing_agent.is_err());
}

#[test]
fn safe_summary_never_contains_raw_prompt() {
    let summary = redacted_fixture_for_tests(
        "task-1",
        "digest.prompt.123",
        "Daily analysis task",
    );
    let encoded = serde_json::to_string(&summary).unwrap();
    assert!(!encoded.contains("Analyze the latest market state"));
    assert!(encoded.contains("digest.prompt.123"));
    assert!(encoded.contains("Daily analysis task"));
}

#[test]
fn create_command_round_trips_through_service_command() {
    let command = CreateScheduledAgentTaskCommand::new(
        TraceContext::new("trace-scheduled-agent-task-roundtrip"),
        AutonomyScope::application(ApplicationId::from_name("scheduled-task-test")),
        ScheduledAgentTaskSchedule::Every {
            interval_ms: 120_000,
        },
        "task-runner",
        "Prepare a status digest.",
    )
    .unwrap()
    .with_delegated_context(serde_json::json!({"format": "summary"}));

    let service_command = command.clone().into_service_command().unwrap();
    assert_eq!(
        service_command.name.as_str(),
        SCHEDULED_AGENT_TASK_CREATE_COMMAND
    );
    assert!(service_command.trace.is_some());

    let decoded: CreateScheduledAgentTaskCommand =
        serde_json::from_value(service_command.payload).unwrap();
    assert_eq!(decoded.target_agent, "task-runner");
    assert_eq!(decoded.user_prompt, "Prepare a status digest.");
    assert_eq!(decoded.delegated_context["format"], "summary");
}

#[test]
fn record_result_command_round_trips_through_service_command() {
    let command = RecordScheduledAgentTaskResultCommand::new(
        TraceContext::new("trace-record-result"),
        ScheduledAgentTaskId::new("scheduled-agent-task-1").unwrap(),
        SchedulerRunId::new("run-1").unwrap(),
        "succeeded",
    )
    .unwrap()
    .with_scheduler_job_id(Some(SchedulerJobId::new("job-1").unwrap()))
    .with_result_evidence(
        Some("trace-agent-execution".into()),
        Some("audit.scheduler.run.succeeded.1".into()),
    );

    let service_command = command.clone().into_service_command().unwrap();
    assert_eq!(
        service_command.name.as_str(),
        SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND
    );
    let decoded: RecordScheduledAgentTaskResultCommand =
        serde_json::from_value(service_command.payload).unwrap();
    assert_eq!(decoded.task_id, command.task_id);
    assert_eq!(decoded.scheduler_run_id, command.scheduler_run_id);
    assert_eq!(decoded.result_status, "succeeded");
}
