//! In-memory local Scheduled Agent Task provider (**Facade** module root).
//!
//! The local provider is the first concrete implementation of
//! `service.scheduled_agent_task`.  It uses the Memento pattern for prompt
//! payload storage, the Facade pattern to register Scheduler jobs through the
//! `SchedulerService` trait, and Observer-style tracing logs for key audit
//! nodes.  The provider is intentionally application-agnostic: it never branches
//! on application names, workflow names, provider names, model names, driver
//! names, gateway names, chains, payment names, or business domains.
//!
//! # Module tree (P5 iteration 92)
//! - `support` — constants + structured error builders
//! - `state` — in-memory memento/state mutations
//! - `metadata` — scheduler metadata sanitization
//! - `tests` — contract tests

mod metadata;
mod state;
mod support;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    AutonomyServiceErrorKind, CancelScheduledAgentTaskCommand, CreateScheduledAgentTaskCommand,
    MacacaResult, RecordScheduledAgentTaskResultCommand, ResolveScheduledAgentTaskPayloadCommand,
    ScheduledAgentTaskCommandResult, ScheduledAgentTaskQueryCommand,
    ScheduledAgentTaskServiceSnapshot, ScheduledAgentTaskSummary, SchedulerDeleteJobCommand,
    SchedulerRegisterJobCommand, ServiceDescriptor, ServiceHealth, ServiceLifecycleState,
    TraceContext, SCHEDULED_AGENT_TASK_SERVICE_ID,
};
use macaca_scheduler::{SchedulerService, UnavailableSchedulerProvider};
use tracing::{info, warn};

use crate::service_contract::ScheduledAgentTaskService;

use state::LocalScheduledAgentTaskState;
use support::{error_result, LOCAL_PROVIDER_ID};

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

    /// Read-only access to in-memory state under the provider lock.
    fn read<R>(&self, f: impl FnOnce(&LocalScheduledAgentTaskState) -> R) -> R {
        let guard = self
            .store
            .read()
            .expect("local scheduled agent task state lock poisoned");
        f(&guard)
    }

    /// Mutable access to in-memory state under the provider lock.
    fn write<R>(&self, f: impl FnOnce(&mut LocalScheduledAgentTaskState) -> R) -> R {
        let mut guard = self
            .store
            .write()
            .expect("local scheduled agent task state lock poisoned");
        f(&mut guard)
    }

    /// Build a sanitized health/snapshot view for diagnostics consumers.
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
            return error_result(
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

        // Build the scheduler registration command. On any failure below
        // (command construction, transport error, or a non-accepted result) we
        // roll back the prepared task + payload (2026-07-08 audit S15) so a
        // failed registration never leaves a permanently "active" zombie task.
        let register_command =
            match SchedulerRegisterJobCommand::new(trace.clone(), prepared.definition) {
                Ok(command) => command,
                Err(error) => {
                    self.write(|state| {
                        state.rollback_prepared_task(
                            &prepared.task_id,
                            &prepared.payload_ref.reference,
                        )
                    });
                    return Err(error);
                }
            };
        let scheduler_result = match self.scheduler.register_job(register_command).await {
            Ok(result) => result,
            Err(error) => {
                self.write(|state| {
                    state
                        .rollback_prepared_task(&prepared.task_id, &prepared.payload_ref.reference)
                });
                return Err(error);
            }
        };
        if !scheduler_result.accepted {
            warn!(
                service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                task_id = prepared.task_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                "scheduled agent task scheduler registration rejected; rolling back prepared task"
            );
            self.write(|state| {
                state.rollback_prepared_task(&prepared.task_id, &prepared.payload_ref.reference)
            });
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
                return error_result(
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
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            task_id = command.task_id.as_str(),
            trace_id = trace.trace_id.as_str(),
            "scheduled agent task cancel completed"
        );
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
    ) -> MacacaResult<Option<macaca_proto::ScheduledAgentTaskResolvedPayload>> {
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
