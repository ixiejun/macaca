//! In-memory task state and memento mutations (**Memento** + **State** patterns).
//!
//! `LocalScheduledAgentTaskState` owns task summaries, payload mementos, and audit
//! history.  Mutations are synchronous and invoked under the provider RwLock.

use std::collections::BTreeMap;

use chrono::Utc;
use macaca_proto::{
    AgentExecutionIntent, AgentExecutionPolicyContext, AgentExecutionTargetCommand, ApplicationId,
    AutonomyPayloadRef, AutonomyServiceErrorKind, CreateScheduledAgentTaskCommand, MacacaResult,
    RecordScheduledAgentTaskResultCommand, ResolveScheduledAgentTaskPayloadCommand,
    ScheduledAgentTaskCommandResult, ScheduledAgentTaskId, ScheduledAgentTaskResolvedPayload,
    ScheduledAgentTaskSummary, SchedulerJobDefinition, SchedulerJobId, SchedulerTargetCommand,
    TaskId, TraceContext,
};

use crate::audit::{LocalAuditRecorder, ScheduledAgentTaskAuditRecord};
use crate::payload_store::InMemoryPayloadStore;

use super::metadata::{sanitize_metadata, scheduler_job_metadata, scheduler_target_metadata};
use super::support::error_result;

/// Intermediate value produced during task creation before scheduler registration.
#[derive(Debug)]
pub(super) struct PreparedScheduledAgentTask {
    pub(super) task_id: ScheduledAgentTaskId,
    pub(super) definition: SchedulerJobDefinition,
    pub(super) payload_ref: AutonomyPayloadRef,
    pub(super) payload_digest: Option<String>,
}

/// In-memory aggregate root for scheduled-agent-task mementos and audit history.
#[derive(Default)]
pub(super) struct LocalScheduledAgentTaskState {
    pub(super) tasks: BTreeMap<ScheduledAgentTaskId, StoredScheduledAgentTask>,
    pub(super) payloads: InMemoryPayloadStore,
    pub(super) audit: LocalAuditRecorder,
    next_task_sequence: u64,
}

impl LocalScheduledAgentTaskState {
    /// Persist payload memento and build scheduler job definition for a new task.
    pub(super) fn prepare_task(
        &mut self,
        application_id: ApplicationId,
        command: CreateScheduledAgentTaskCommand,
    ) -> MacacaResult<PreparedScheduledAgentTask> {
        self.next_task_sequence += 1;
        let task_id =
            ScheduledAgentTaskId::new(format!("scheduled-agent-task-{}", self.next_task_sequence))?;
        let safe_name = command.metadata.get("schedule.name").map(String::as_str);
        let payload_ref = self.payloads.insert(
            task_id.as_str(),
            command.user_prompt,
            command.delegated_context,
            safe_name,
        )?;
        let payload_digest = payload_ref.content_digest.clone();
        let session_id = command
            .scope
            .session_id
            .clone()
            .unwrap_or_else(|| format!("scheduled-agent-task:{}", task_id.as_str()));
        let target = SchedulerTargetCommand::AgentExecution(AgentExecutionTargetCommand {
            application_id,
            session_id,
            task_id: command.scope.task_id,
            target_agent: Some(command.target_agent.clone()),
            execution_intent: AgentExecutionIntent::TaskWorker,
            payload_ref: payload_ref.clone(),
            metadata: scheduler_target_metadata(&task_id, &payload_digest, &command.metadata),
        });
        let mut definition = SchedulerJobDefinition::new(
            command.scope.clone(),
            command.schedule.clone().into_scheduler_spec(),
            target,
        )?;
        definition.metadata = scheduler_job_metadata(&task_id, &payload_digest, &command.metadata);
        let now = Utc::now();
        let summary = ScheduledAgentTaskSummary {
            task_id: task_id.clone(),
            scope: command.scope,
            schedule: command.schedule,
            target_agent: command.target_agent.clone(),
            execution_intent: AgentExecutionIntent::TaskWorker,
            scheduler_job_id: None,
            payload_ref: payload_ref.clone(),
            payload_digest: payload_digest.clone(),
            redacted_summary: payload_ref.redacted_summary.clone(),
            lifecycle_state: "active".into(),
            last_run_id: None,
            last_result_status: None,
            trace_id: Some(command.trace.trace_id.clone()),
            audit_id: None,
            created_at: now,
            updated_at: now,
            metadata: sanitize_metadata(command.metadata),
        };
        let mut audit_record =
            ScheduledAgentTaskAuditRecord::new("created", command.trace.trace_id, "created");
        audit_record.task_id = Some(task_id.as_str().into());
        audit_record.payload_digest = payload_digest.clone();
        audit_record.target_agent = Some(command.target_agent);
        let audit_id = self.audit.record(audit_record);
        let mut summary = summary;
        summary.audit_id = Some(audit_id);
        self.tasks.insert(
            task_id.clone(),
            StoredScheduledAgentTask {
                application_id,
                session_id: match &definition.target {
                    SchedulerTargetCommand::AgentExecution(target) => target.session_id.clone(),
                    _ => String::new(),
                },
                task_ref: definition.scope.task_id,
                policy: command.policy.execution_policy,
                summary,
            },
        );
        Ok(PreparedScheduledAgentTask {
            task_id,
            definition,
            payload_ref,
            payload_digest,
        })
    }

    pub(super) fn attach_scheduler_job(
        &mut self,
        task_id: &ScheduledAgentTaskId,
        scheduler_job_id: Option<SchedulerJobId>,
        scheduler_audit_id: Option<String>,
        trace: &TraceContext,
    ) -> Option<String> {
        let task = self.tasks.get_mut(task_id)?;
        task.summary.scheduler_job_id = scheduler_job_id.clone();
        task.summary.updated_at = Utc::now();
        let mut audit_record = ScheduledAgentTaskAuditRecord::new(
            "scheduler_registered",
            &trace.trace_id,
            "registered",
        );
        audit_record.task_id = Some(task_id.as_str().into());
        audit_record.scheduler_job_id = scheduler_job_id.as_ref().map(|id| id.as_str().into());
        audit_record.payload_digest = task.summary.payload_digest.clone();
        audit_record.target_agent = Some(task.summary.target_agent.clone());
        if let Some(scheduler_audit_id) = scheduler_audit_id {
            audit_record
                .metadata
                .insert("scheduler_audit_id".into(), scheduler_audit_id);
        }
        let audit_id = self.audit.record(audit_record);
        task.summary.audit_id = Some(audit_id.clone());
        Some(audit_id)
    }

    pub(super) fn cancel_task(
        &mut self,
        task_id: &ScheduledAgentTaskId,
        trace: &TraceContext,
    ) -> Option<String> {
        let task = self.tasks.get_mut(task_id)?;
        task.summary.lifecycle_state = "cancelled".into();
        task.summary.updated_at = Utc::now();
        let mut audit_record =
            ScheduledAgentTaskAuditRecord::new("cancelled", &trace.trace_id, "cancelled");
        audit_record.task_id = Some(task_id.as_str().into());
        audit_record.scheduler_job_id = task
            .summary
            .scheduler_job_id
            .as_ref()
            .map(|id| id.as_str().into());
        audit_record.payload_digest = task.summary.payload_digest.clone();
        audit_record.target_agent = Some(task.summary.target_agent.clone());
        let audit_id = self.audit.record(audit_record);
        task.summary.audit_id = Some(audit_id.clone());
        Some(audit_id)
    }

    pub(super) fn resolve_payload(
        &self,
        command: ResolveScheduledAgentTaskPayloadCommand,
    ) -> Option<ScheduledAgentTaskResolvedPayload> {
        let payload = self.payloads.get(&command.payload_ref.reference)?;
        let (task_id, task) = self.tasks.iter().find(|(_, task)| {
            task.summary.payload_ref.reference == command.payload_ref.reference
        })?;
        Some(ScheduledAgentTaskResolvedPayload {
            task_id: task_id.clone(),
            application_id: task.application_id,
            session_id: task.session_id.clone(),
            task_ref: task.task_ref,
            target_agent: task.summary.target_agent.clone(),
            execution_intent: task.summary.execution_intent.clone(),
            user_prompt: payload.prompt.clone(),
            delegated_context: payload.delegated_context.clone(),
            policy: task.policy.clone(),
            payload_ref: command.payload_ref,
            payload_digest: Some(payload.digest.clone()),
            trace: command.trace,
            audit_id: task.summary.audit_id.clone(),
            metadata: task.summary.metadata.clone(),
        })
    }

    pub(super) fn record_result(
        &mut self,
        command: RecordScheduledAgentTaskResultCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        let Some(task) = self.tasks.get_mut(&command.task_id) else {
            return error_result(
                command.trace,
                AutonomyServiceErrorKind::InvalidRequest,
                "scheduled_task_not_found",
                "scheduled agent task was not found",
            );
        };
        if let Some(scheduler_job_id) = command.scheduler_job_id.clone() {
            task.summary
                .scheduler_job_id
                .get_or_insert(scheduler_job_id);
        }
        task.summary.last_run_id = Some(command.scheduler_run_id.clone());
        task.summary.last_result_status = Some(command.result_status.clone());
        task.summary.trace_id = Some(command.trace.trace_id.clone());
        task.summary.updated_at = Utc::now();
        let mut audit_record = ScheduledAgentTaskAuditRecord::new(
            "result_recorded",
            &command.trace.trace_id,
            command.result_status.clone(),
        );
        audit_record.task_id = Some(command.task_id.as_str().into());
        audit_record.scheduler_job_id = task
            .summary
            .scheduler_job_id
            .as_ref()
            .map(|id| id.as_str().into());
        audit_record.scheduler_run_id = Some(command.scheduler_run_id.as_str().into());
        audit_record.payload_digest = task.summary.payload_digest.clone();
        audit_record.target_agent = Some(task.summary.target_agent.clone());
        if let Some(agent_execution_trace_id) = command.agent_execution_trace_id {
            audit_record
                .metadata
                .insert("agent_execution_trace_id".into(), agent_execution_trace_id);
        }
        if let Some(result_audit_id) = command.result_audit_id {
            audit_record
                .metadata
                .insert("result_audit_id".into(), result_audit_id);
        }
        audit_record
            .metadata
            .extend(sanitize_metadata(command.metadata));
        let audit_id = self.audit.record(audit_record);
        task.summary.audit_id = Some(audit_id.clone());
        Ok(ScheduledAgentTaskCommandResult {
            accepted: true,
            task_id: Some(command.task_id),
            scheduler_job_id: task.summary.scheduler_job_id.clone(),
            scheduler_run_id: Some(command.scheduler_run_id),
            payload_ref: Some(task.summary.payload_ref.clone()),
            payload_digest: task.summary.payload_digest.clone(),
            trace: command.trace,
            audit_id: Some(audit_id),
            summary: Some(task.summary.clone()),
            error: None,
            metadata: BTreeMap::new(),
        })
    }
}

pub(super) struct StoredScheduledAgentTask {
    application_id: ApplicationId,
    session_id: String,
    task_ref: Option<TaskId>,
    policy: AgentExecutionPolicyContext,
    pub(super) summary: ScheduledAgentTaskSummary,
}
