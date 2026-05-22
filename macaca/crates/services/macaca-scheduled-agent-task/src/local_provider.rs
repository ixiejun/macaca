//! In-memory local Scheduled Agent Task provider.
//!
//! The local provider is the first concrete implementation of
//! `service.scheduled_agent_task`.  It uses the Memento pattern for prompt
//! payload storage, the Facade pattern to register Scheduler jobs through the
//! `SchedulerService` trait, and Observer-style tracing logs for key audit
//! nodes.  The provider is intentionally application-agnostic: it never branches
//! on application names, workflow names, provider names, model names, driver
//! names, gateway names, chains, payment names, or business domains.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    AgentExecutionIntent, AgentExecutionPolicyContext, AgentExecutionTargetCommand, ApplicationId,
    AutonomyAuditCorrelation, AutonomyPayloadRef, AutonomyServiceErrorKind,
    AutonomyStructuredError, CancelScheduledAgentTaskCommand, CreateScheduledAgentTaskCommand,
    MacacaResult, RecordScheduledAgentTaskResultCommand, ResolveScheduledAgentTaskPayloadCommand,
    ScheduledAgentTaskCommandResult, ScheduledAgentTaskId, ScheduledAgentTaskQueryCommand,
    ScheduledAgentTaskResolvedPayload, ScheduledAgentTaskServiceSnapshot,
    ScheduledAgentTaskSummary, SchedulerDeleteJobCommand, SchedulerJobDefinition, SchedulerJobId,
    SchedulerRegisterJobCommand, SchedulerTargetCommand, ServiceDescriptor, ServiceHealth,
    ServiceLifecycleState, TaskId, TraceContext, SCHEDULED_AGENT_TASK_SERVICE_ID,
};
use macaca_scheduler::{SchedulerService, UnavailableSchedulerProvider};
use tracing::{info, warn};

use crate::audit::{LocalAuditRecorder, ScheduledAgentTaskAuditRecord};
use crate::payload_store::InMemoryPayloadStore;
use crate::service_contract::ScheduledAgentTaskService;

/// Local provider identifier used in sanitized snapshots.
const LOCAL_PROVIDER_ID: &str = "local.in_memory";

/// Local Scheduled Agent Task provider backed by in-memory mementos.
pub struct LocalScheduledAgentTaskProvider {
    descriptor: ServiceDescriptor,
    scheduler: Arc<dyn SchedulerService>,
    store: RwLock<LocalScheduledAgentTaskState>,
}

impl LocalScheduledAgentTaskProvider {
    /// Create a provider with an explicit Scheduler service dependency.
    ///
    /// The dependency is a trait object rather than a concrete provider so this
    /// crate stays serviceized.  Runtime Host decides which scheduler provider
    /// is installed at composition time.
    pub fn new(scheduler: Arc<dyn SchedulerService>) -> Self {
        let unavailable = crate::UnavailableScheduledAgentTaskProvider::default();
        let mut descriptor = unavailable.descriptor();
        descriptor.health = ServiceHealth::Healthy;
        descriptor.lifecycle_state = ServiceLifecycleState::Running;
        descriptor
            .metadata
            .insert("provider_id".into(), LOCAL_PROVIDER_ID.into());
        Self {
            descriptor,
            scheduler,
            store: RwLock::new(LocalScheduledAgentTaskState::default()),
        }
    }

    fn read<R>(&self, f: impl FnOnce(&LocalScheduledAgentTaskState) -> R) -> R {
        let guard = self
            .store
            .read()
            .expect("local scheduled agent task state lock poisoned");
        f(&guard)
    }

    fn write<R>(&self, f: impl FnOnce(&mut LocalScheduledAgentTaskState) -> R) -> R {
        let mut guard = self
            .store
            .write()
            .expect("local scheduled agent task state lock poisoned");
        f(&mut guard)
    }

    fn snapshot_inner(&self) -> ScheduledAgentTaskServiceSnapshot {
        self.read(|state| ScheduledAgentTaskServiceSnapshot {
            service_id: SCHEDULED_AGENT_TASK_SERVICE_ID.into(),
            provider_id: LOCAL_PROVIDER_ID.into(),
            healthy: true,
            lifecycle_state: "running".into(),
            active_tasks: state
                .tasks
                .values()
                .filter(|task| task.summary.lifecycle_state == "active")
                .count(),
            stored_payloads: state.payloads.len(),
            recent_audit_ids: state.audit.recent_ids(25),
            captured_at: Utc::now(),
            metadata: BTreeMap::new(),
        })
    }

    fn structured_error(
        trace: TraceContext,
        kind: AutonomyServiceErrorKind,
        reason_code: impl Into<String>,
        safe_message: impl Into<String>,
    ) -> MacacaResult<AutonomyStructuredError> {
        Ok(AutonomyStructuredError {
            kind,
            reason_code: reason_code.into(),
            safe_message: safe_message.into(),
            correlation: AutonomyAuditCorrelation::from_trace(trace)?,
            metadata: BTreeMap::new(),
        })
    }

    fn error_result(
        trace: TraceContext,
        kind: AutonomyServiceErrorKind,
        reason_code: impl Into<String>,
        safe_message: impl Into<String>,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        let error = Self::structured_error(trace.clone(), kind, reason_code, safe_message)?;
        Ok(ScheduledAgentTaskCommandResult::rejected(trace, error))
    }
}

impl Default for LocalScheduledAgentTaskProvider {
    fn default() -> Self {
        Self::new(Arc::new(UnavailableSchedulerProvider::default()))
    }
}

#[async_trait]
impl ScheduledAgentTaskService for LocalScheduledAgentTaskProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<ScheduledAgentTaskServiceSnapshot> {
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            trace_id = trace.trace_id.as_str(),
            "scheduled agent task health requested"
        );
        Ok(self.snapshot_inner())
    }

    async fn create_task(
        &self,
        command: CreateScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        let trace = command.trace.clone();
        let Some(application_id) = command.scope.application_id else {
            warn!(
                service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                trace_id = trace.trace_id.as_str(),
                "scheduled agent task create rejected because application scope is missing"
            );
            return Self::error_result(
                trace,
                AutonomyServiceErrorKind::InvalidRequest,
                "missing_application_scope",
                "scheduled agent task requires application scope",
            );
        };
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            application_id = %application_id.0,
            target_agent = command.target_agent.as_str(),
            trace_id = trace.trace_id.as_str(),
            "scheduled agent task create requested"
        );

        let prepared = self.write(|state| state.prepare_task(application_id, command))?;
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            task_id = prepared.task_id.as_str(),
            payload_digest = prepared.payload_digest.as_deref().unwrap_or("none"),
            trace_id = trace.trace_id.as_str(),
            "scheduled agent task payload persisted"
        );

        let scheduler_result = self
            .scheduler
            .register_job(SchedulerRegisterJobCommand::new(
                trace.clone(),
                prepared.definition,
            )?)
            .await?;
        if !scheduler_result.accepted {
            warn!(
                service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                task_id = prepared.task_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                "scheduled agent task scheduler registration failed"
            );
            return Ok(ScheduledAgentTaskCommandResult {
                accepted: false,
                task_id: Some(prepared.task_id),
                scheduler_job_id: scheduler_result.job_id,
                scheduler_run_id: scheduler_result.run_id,
                payload_ref: Some(prepared.payload_ref),
                payload_digest: prepared.payload_digest,
                trace,
                audit_id: scheduler_result.audit_id,
                summary: None,
                error: scheduler_result.error,
                metadata: BTreeMap::new(),
            });
        }

        let scheduler_job_id = scheduler_result.job_id.clone();
        let audit_id = self.write(|state| {
            state.attach_scheduler_job(
                &prepared.task_id,
                scheduler_job_id.clone(),
                scheduler_result.audit_id.clone(),
                &trace,
            )
        });
        let summary = self.read(|state| {
            state
                .tasks
                .get(&prepared.task_id)
                .map(|task| task.summary.clone())
        });
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            task_id = prepared.task_id.as_str(),
            scheduler_job_id = scheduler_job_id
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("none"),
            audit_id = audit_id.as_deref().unwrap_or("none"),
            trace_id = trace.trace_id.as_str(),
            "scheduled agent task create completed"
        );

        Ok(ScheduledAgentTaskCommandResult {
            accepted: true,
            task_id: Some(prepared.task_id),
            scheduler_job_id,
            scheduler_run_id: scheduler_result.run_id,
            payload_ref: Some(prepared.payload_ref),
            payload_digest: prepared.payload_digest,
            trace,
            audit_id,
            summary,
            error: None,
            metadata: BTreeMap::new(),
        })
    }

    async fn get_task(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskSummary>> {
        let Some(task_id) = command.task_id else {
            return Ok(None);
        };
        Ok(self.read(|state| {
            state
                .tasks
                .get(&task_id)
                .filter(|task| task.summary.scope == command.scope)
                .map(|task| task.summary.clone())
        }))
    }

    async fn list_tasks(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Vec<ScheduledAgentTaskSummary>> {
        Ok(self.read(|state| {
            state
                .tasks
                .values()
                .filter(|task| task.summary.scope == command.scope)
                .take(command.limit.unwrap_or(100))
                .map(|task| task.summary.clone())
                .collect()
        }))
    }

    async fn cancel_task(
        &self,
        command: CancelScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        let trace = command.trace.clone();
        let (job_id, payload_ref, payload_digest) = match self.read(|state| {
            state.tasks.get(&command.task_id).map(|task| {
                (
                    task.summary.scheduler_job_id.clone(),
                    task.summary.payload_ref.clone(),
                    task.summary.payload_digest.clone(),
                )
            })
        }) {
            Some(values) => values,
            None => {
                return Self::error_result(
                    trace,
                    AutonomyServiceErrorKind::InvalidRequest,
                    "scheduled_task_not_found",
                    "scheduled agent task was not found",
                );
            }
        };

        let scheduler_result = if let Some(job_id) = job_id.clone() {
            Some(
                self.scheduler
                    .delete_job(SchedulerDeleteJobCommand::new(
                        trace.clone(),
                        command.scope.clone(),
                        job_id,
                        command.reason_code.clone(),
                    )?)
                    .await?,
            )
        } else {
            None
        };
        let audit_id = self.write(|state| state.cancel_task(&command.task_id, &trace));
        Ok(ScheduledAgentTaskCommandResult {
            accepted: scheduler_result
                .as_ref()
                .map(|result| result.accepted)
                .unwrap_or(true),
            task_id: Some(command.task_id),
            scheduler_job_id: job_id,
            scheduler_run_id: None,
            payload_ref: Some(payload_ref),
            payload_digest,
            trace,
            audit_id,
            summary: None,
            error: scheduler_result.and_then(|result| result.error),
            metadata: BTreeMap::new(),
        })
    }

    async fn resolve_payload(
        &self,
        command: ResolveScheduledAgentTaskPayloadCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskResolvedPayload>> {
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            payload_digest = command
                .payload_ref
                .content_digest
                .as_deref()
                .unwrap_or("none"),
            trace_id = command.trace.trace_id.as_str(),
            "scheduled agent task payload resolve requested"
        );
        Ok(self.read(|state| state.resolve_payload(command)))
    }

    async fn record_result(
        &self,
        command: RecordScheduledAgentTaskResultCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        let trace = command.trace.clone();
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            task_id = command.task_id.as_str(),
            scheduler_run_id = command.scheduler_run_id.as_str(),
            result_status = command.result_status.as_str(),
            trace_id = trace.trace_id.as_str(),
            "scheduled agent task result record requested"
        );
        let result = self.write(|state| state.record_result(command))?;
        if !result.accepted {
            warn!(
                service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                trace_id = trace.trace_id.as_str(),
                "scheduled agent task result record rejected"
            );
        }
        Ok(result)
    }
}

#[derive(Debug)]
struct PreparedScheduledAgentTask {
    task_id: ScheduledAgentTaskId,
    definition: SchedulerJobDefinition,
    payload_ref: AutonomyPayloadRef,
    payload_digest: Option<String>,
}

#[derive(Default)]
struct LocalScheduledAgentTaskState {
    next_task_sequence: u64,
    tasks: BTreeMap<ScheduledAgentTaskId, StoredScheduledAgentTask>,
    payloads: InMemoryPayloadStore,
    audit: LocalAuditRecorder,
}

impl LocalScheduledAgentTaskState {
    fn prepare_task(
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
            metadata: scheduler_target_metadata(&task_id, &payload_digest),
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

    fn attach_scheduler_job(
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

    fn cancel_task(
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

    fn resolve_payload(
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

    fn record_result(
        &mut self,
        command: RecordScheduledAgentTaskResultCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        let Some(task) = self.tasks.get_mut(&command.task_id) else {
            return LocalScheduledAgentTaskProvider::error_result(
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

struct StoredScheduledAgentTask {
    application_id: ApplicationId,
    session_id: String,
    task_ref: Option<TaskId>,
    policy: AgentExecutionPolicyContext,
    summary: ScheduledAgentTaskSummary,
}

fn scheduler_target_metadata(
    task_id: &ScheduledAgentTaskId,
    payload_digest: &Option<String>,
) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert("source".into(), SCHEDULED_AGENT_TASK_SERVICE_ID.into());
    metadata.insert("scheduled_agent_task_id".into(), task_id.as_str().into());
    metadata.insert("scheduler_run_source".into(), "service.scheduler".into());
    if let Some(digest) = payload_digest {
        metadata.insert("payload_digest".into(), digest.clone());
    }
    metadata
}

fn scheduler_job_metadata(
    task_id: &ScheduledAgentTaskId,
    payload_digest: &Option<String>,
    caller_metadata: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut metadata = sanitize_metadata(caller_metadata.clone());
    metadata.insert("source".into(), SCHEDULED_AGENT_TASK_SERVICE_ID.into());
    metadata.insert("scheduled_agent_task_id".into(), task_id.as_str().into());
    if let Some(digest) = payload_digest {
        metadata.insert("payload_digest".into(), digest.clone());
    }
    metadata
}

fn sanitize_metadata(metadata: BTreeMap<String, String>) -> BTreeMap<String, String> {
    metadata
        .into_iter()
        .filter(|(key, _)| {
            let lowered = key.to_ascii_lowercase();
            !lowered.contains("prompt")
                && !lowered.contains("secret")
                && !lowered.contains("token")
                && !lowered.contains("credential")
                && !lowered.contains("private_key")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{
        AutonomyScope, RecordScheduledAgentTaskResultCommand, ScheduledAgentTaskSchedule,
        SchedulerCommandResult, SchedulerJobLifecycleState, SchedulerRunId, SchedulerRunState,
    };
    use tokio::sync::Mutex;

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
            "worker",
            raw_prompt,
        )
        .unwrap()
        .with_metadata(metadata)
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
                assert_eq!(target.target_agent.as_deref(), Some("worker"));
                assert!(target.payload_ref.content_digest.is_some());
                let encoded = serde_json::to_string(target).unwrap();
                assert!(!encoded.contains(raw_prompt));
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
        assert_eq!(resolved.target_agent, "worker");
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
}
