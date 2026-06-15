//! Contract tests for scheduled-agent-task local provider behavior.
//!
//! Uses neutral fixtures (generic application/agent ids) — no application-specific names.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationId, AutonomyScope, CreateScheduledAgentTaskCommand, MacacaResult,
    RecordScheduledAgentTaskResultCommand, ResolveScheduledAgentTaskPayloadCommand,
    ScheduledAgentTaskQueryCommand, ScheduledAgentTaskSchedule, SchedulerCommandResult,
    SchedulerDeleteJobCommand, SchedulerJobDefinition, SchedulerJobId, SchedulerJobLifecycleState,
    SchedulerRegisterJobCommand, SchedulerRunId, SchedulerRunState, SchedulerTargetCommand,
    ServiceDescriptor, TraceContext,
};
use macaca_scheduler::{SchedulerService, UnavailableSchedulerProvider};
use tokio::sync::Mutex;

use super::LocalScheduledAgentTaskProvider;
use crate::service_contract::ScheduledAgentTaskService;

/// Provider-neutral fixture agent id (Object Mother pattern).
///
/// Must not use application role literals (`worker`, `coordinator`, etc.) so
/// escape-hatch raw inventory and OS test-fixture guards stay green.
const FIXTURE_TARGET_AGENT: &str = "fixture-scheduled-agent";

#[derive(Default)]
struct RecordingScheduler {
    jobs: Mutex<Vec<SchedulerJobDefinition>>,
}

#[async_trait]
impl SchedulerService for RecordingScheduler {
    fn descriptor(&self) -> ServiceDescriptor {
        UnavailableSchedulerProvider::default().descriptor()
    }

    async fn health(
        &self,
        trace: TraceContext,
    ) -> MacacaResult<macaca_proto::SchedulerServiceSnapshot> {
        UnavailableSchedulerProvider::default().health(trace).await
    }

    async fn snapshot(
        &self,
        command: macaca_proto::SchedulerQueryCommand,
    ) -> MacacaResult<macaca_proto::SchedulerServiceSnapshot> {
        UnavailableSchedulerProvider::default()
            .snapshot(command)
            .await
    }

    async fn register_job(
        &self,
        command: SchedulerRegisterJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.jobs.lock().await.push(command.definition);
        Ok(SchedulerCommandResult {
            job_id: Some(SchedulerJobId::new("job-recorded").unwrap()),
            run_id: None,
            lifecycle: Some(SchedulerJobLifecycleState::Active),
            run_state: None,
            accepted: true,
            error: None,
            trace: command.trace,
            audit_id: Some("audit.scheduler.job.registered.test".into()),
            metadata: BTreeMap::new(),
        })
    }

    async fn update_job(
        &self,
        command: macaca_proto::SchedulerUpdateJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .update_job(command)
            .await
    }

    async fn pause_job(
        &self,
        command: macaca_proto::SchedulerLifecycleJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .pause_job(command)
            .await
    }

    async fn resume_job(
        &self,
        command: macaca_proto::SchedulerLifecycleJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .resume_job(command)
            .await
    }

    async fn delete_job(
        &self,
        command: SchedulerDeleteJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        Ok(SchedulerCommandResult {
            job_id: Some(command.job_id),
            run_id: None,
            lifecycle: Some(SchedulerJobLifecycleState::Deleted),
            run_state: Some(SchedulerRunState::Cancelled),
            accepted: true,
            error: None,
            trace: command.trace,
            audit_id: Some("audit.scheduler.job.deleted.test".into()),
            metadata: BTreeMap::new(),
        })
    }

    async fn trigger_job(
        &self,
        command: macaca_proto::SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .trigger_job(command)
            .await
    }

    async fn get_job(
        &self,
        command: macaca_proto::SchedulerGetJobCommand,
    ) -> MacacaResult<Option<macaca_proto::SchedulerJobSummary>> {
        UnavailableSchedulerProvider::default()
            .get_job(command)
            .await
    }

    async fn list_jobs(
        &self,
        command: macaca_proto::SchedulerListJobsCommand,
    ) -> MacacaResult<Vec<macaca_proto::SchedulerJobSummary>> {
        UnavailableSchedulerProvider::default()
            .list_jobs(command)
            .await
    }

    async fn get_run(
        &self,
        command: macaca_proto::SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .get_run(command)
            .await
    }

    async fn list_runs(
        &self,
        command: macaca_proto::SchedulerQueryCommand,
    ) -> MacacaResult<Vec<macaca_proto::SchedulerRunSummary>> {
        UnavailableSchedulerProvider::default()
            .list_runs(command)
            .await
    }
}

fn create_command(raw_prompt: &str) -> CreateScheduledAgentTaskCommand {
    let mut metadata = BTreeMap::new();
    metadata.insert("schedule.name".into(), "Daily safe summary".into());
    CreateScheduledAgentTaskCommand::new(
        TraceContext::new("trace-scheduled-agent-task-provider-test"),
        AutonomyScope::application(ApplicationId::from_name("scheduled-task-provider-test")),
        ScheduledAgentTaskSchedule::Every {
            interval_ms: 60_000,
        },
        FIXTURE_TARGET_AGENT,
        raw_prompt,
    )
    .unwrap()
    .with_metadata(metadata)
}

fn create_command_with_skill_alias(raw_prompt: &str) -> CreateScheduledAgentTaskCommand {
    let mut command = create_command(raw_prompt);
    command.metadata.insert(
        "skill.alias.requested_id".into(),
        "skill://agent/stable-task-skill".into(),
    );
    command.metadata.insert(
        "prompt.secret".into(),
        "RAW_PROMPT_METADATA_SHOULD_NOT_LEAK".into(),
    );
    command
}

#[tokio::test]
async fn create_stores_prompt_in_payload_store_and_registers_agent_execution_target() {
    let scheduler = Arc::new(RecordingScheduler::default());
    let provider = LocalScheduledAgentTaskProvider::new(scheduler.clone());
    let raw_prompt = "RAW_PROMPT_SHOULD_NOT_LEAK: analyze and record.";

    let result = provider
        .create_task(create_command(raw_prompt))
        .await
        .unwrap();

    assert!(result.accepted);
    assert!(result.audit_id.is_some());
    assert_eq!(
        result.scheduler_job_id.as_ref().unwrap().as_str(),
        "job-recorded"
    );
    let summary_json = serde_json::to_string(&result.summary).unwrap();
    assert!(!summary_json.contains(raw_prompt));
    assert!(summary_json.contains("Daily safe summary"));
    assert!(summary_json.contains("digest.scheduled_agent_task"));

    let jobs = scheduler.jobs.lock().await;
    assert_eq!(jobs.len(), 1);
    match &jobs[0].target {
        SchedulerTargetCommand::AgentExecution(target) => {
            assert_eq!(target.target_agent.as_deref(), Some(FIXTURE_TARGET_AGENT));
            assert!(target.payload_ref.content_digest.is_some());
            let encoded = serde_json::to_string(target).unwrap();
            assert!(!encoded.contains(raw_prompt));
        }
        other => panic!("expected agent execution target, got {other:?}"),
    }
}

#[tokio::test]
async fn create_preserves_sanitized_skill_alias_refs_for_scheduler_dispatch() {
    let scheduler = Arc::new(RecordingScheduler::default());
    let provider = LocalScheduledAgentTaskProvider::new(scheduler.clone());

    let result = provider
        .create_task(create_command_with_skill_alias(
            "RAW_PROMPT_STAYS_IN_PAYLOAD_STORE",
        ))
        .await
        .unwrap();

    assert!(result.accepted);
    let jobs = scheduler.jobs.lock().await;
    match &jobs[0].target {
        SchedulerTargetCommand::AgentExecution(target) => {
            assert_eq!(
                target.metadata["skill.alias.requested_id"],
                "skill://agent/stable-task-skill"
            );
            assert!(!target.metadata.contains_key("prompt.secret"));
        }
        other => panic!("expected agent execution target, got {other:?}"),
    }
}

#[tokio::test]
async fn resolve_payload_returns_prompt_only_through_service_boundary() {
    let scheduler = Arc::new(RecordingScheduler::default());
    let provider = LocalScheduledAgentTaskProvider::new(scheduler);
    let raw_prompt = "RAW_PROMPT_AVAILABLE_ONLY_AFTER_RESOLVE";
    let result = provider
        .create_task(create_command(raw_prompt))
        .await
        .unwrap();

    let resolved = provider
        .resolve_payload(
            ResolveScheduledAgentTaskPayloadCommand::new(
                TraceContext::new("trace-resolve"),
                result.payload_ref.unwrap(),
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(resolved.user_prompt, raw_prompt);
    assert_eq!(resolved.target_agent, FIXTURE_TARGET_AGENT);
    assert!(resolved
        .payload_digest
        .unwrap()
        .starts_with("digest.scheduled_agent_task"));
}

#[tokio::test]
async fn record_result_updates_redacted_summary_last_run_fields() {
    let scheduler = Arc::new(RecordingScheduler::default());
    let provider = LocalScheduledAgentTaskProvider::new(scheduler);
    let raw_prompt = "RAW_PROMPT_MUST_STAY_IN_PAYLOAD_STORE";
    let create = provider
        .create_task(create_command(raw_prompt))
        .await
        .unwrap();
    let task_id = create.task_id.clone().unwrap();
    let run_id = SchedulerRunId::new("run-recorded").unwrap();

    let result = provider
        .record_result(
            RecordScheduledAgentTaskResultCommand::new(
                TraceContext::new("trace-record-result"),
                task_id.clone(),
                run_id.clone(),
                "succeeded",
            )
            .unwrap(),
        )
        .await
        .unwrap();

    assert!(result.accepted);
    let summary = provider
        .get_task(
            ScheduledAgentTaskQueryCommand::new(
                TraceContext::new("trace-get-recorded-result"),
                create.summary.clone().unwrap().scope,
                Some(task_id),
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(summary.last_run_id, Some(run_id));
    assert_eq!(summary.last_result_status.as_deref(), Some("succeeded"));
    assert!(!serde_json::to_string(&summary)
        .unwrap()
        .contains(raw_prompt));
    assert!(result
        .audit_id
        .as_deref()
        .unwrap_or_default()
        .starts_with("audit.scheduled_agent_task."));
}

#[tokio::test]
async fn unavailable_provider_does_not_fake_success() {
    let provider = crate::UnavailableScheduledAgentTaskProvider::default();
    let result = provider
        .create_task(create_command("safe prompt"))
        .await
        .unwrap();

    assert!(!result.accepted);
    assert!(result.error.is_some());
    assert!(result.scheduler_job_id.is_none());
}
