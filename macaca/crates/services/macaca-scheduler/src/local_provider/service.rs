//! `SchedulerService` trait implementation for the in-process local provider.
//!
//! This module owns job registration, query, and lifecycle mutations.  Run lease
//! and dispatch-boundary transitions live in `run_control.rs`; due-time
//! materialization lives in `materialization.rs`.

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    AutonomyServiceErrorKind, MacacaResult, SchedulerCommandResult, SchedulerDeleteJobCommand,
    SchedulerGetJobCommand, SchedulerJobCommand, SchedulerJobLifecycleState,
    SchedulerJobSummary, SchedulerLifecycleJobCommand, SchedulerListJobsCommand,
    SchedulerQueryCommand, SchedulerRegisterJobCommand, SchedulerRunState,
    SchedulerRunSummary, SchedulerServiceSnapshot, SchedulerUpdateJobCommand, ServiceDescriptor,
    TraceContext, SCHEDULER_SERVICE_ID,
};
use tracing::{info, warn};

use crate::service_contract::SchedulerService;

use super::store::StoredJob;
use super::store::StoredRun;
use super::support::LOCAL_PROVIDER_ID;

#[async_trait]
impl SchedulerService for super::InProcessSchedulerProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn health(
        &self,
        trace: TraceContext,
    ) -> MacacaResult<SchedulerServiceSnapshot> {
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            trace_id = trace.trace_id.as_str(),
            "local scheduler health requested"
        );
        self.refresh_due_runs(&trace);
        Ok(self.snapshot_inner())
    }

    async fn snapshot(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerServiceSnapshot> {
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            trace_id = command.trace.trace_id.as_str(),
            "local scheduler snapshot requested"
        );
        self.refresh_due_runs(&command.trace);
        Ok(self.snapshot_inner())
    }

    async fn register_job(
        &self,
        mut command: SchedulerRegisterJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        let trace = command.trace.clone();
        let job_id = command
            .definition
            .job_id
            .clone()
            .unwrap_or_else(|| self.store.write(|state| state.next_job_id()));
        command.definition.job_id = Some(job_id.clone());
        let lifecycle = command.definition.lifecycle.clone();
        let audit_id = self.store.write(|state| {
            let audit_id = state.record_audit("job.registered");
            info!(
                service_id = SCHEDULER_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                job_id = job_id.as_str(),
                audit_id = audit_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                "local scheduler registered job definition"
            );
            state.jobs.insert(
                job_id.clone(),
                StoredJob::new(command.definition, Utc::now()),
            );
            audit_id
        });
        let mut result = self.result(trace, Some(job_id), None, Some(lifecycle), None);
        result.audit_id = Some(audit_id);
        Ok(result)
    }

    async fn update_job(
        &self,
        command: SchedulerUpdateJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        let trace = command.trace.clone();
        Ok(self.store.write(|state| {
            let Some(job) = state.jobs.get_mut(&command.job_id) else {
                warn!(
                    service_id = SCHEDULER_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    job_id = command.job_id.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local scheduler job update rejected because job is unknown"
                );
                return self.error_result(
                    trace,
                    AutonomyServiceErrorKind::InvalidRequest,
                    "job_not_found",
                    "scheduler job was not found",
                );
            };
            if job.definition.scope != command.scope {
                return self.error_result(
                    trace,
                    AutonomyServiceErrorKind::Denied,
                    "scope_mismatch",
                    "scheduler job does not belong to the requested scope",
                );
            }
            let mut definition = command.definition;
            definition.job_id = Some(command.job_id.clone());
            definition.scope = job.definition.scope.clone();
            job.definition = definition;
            job.updated_at = Utc::now();
            let lifecycle = job.definition.lifecycle.clone();
            let audit_id = state.record_audit("job.updated");
            info!(
                service_id = SCHEDULER_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                job_id = command.job_id.as_str(),
                audit_id = audit_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                reason_code = command.reason_code.as_str(),
                "local scheduler job update completed"
            );
            let mut result = self.result(trace, Some(command.job_id), None, Some(lifecycle), None);
            result.audit_id = Some(audit_id);
            result
        }))
    }

    async fn pause_job(
        &self,
        command: SchedulerLifecycleJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.mutate_job(command, "paused", |job, _| {
            job.definition.lifecycle = SchedulerJobLifecycleState::Paused;
        })
    }

    async fn resume_job(
        &self,
        command: SchedulerLifecycleJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.mutate_job(command, "resumed", |job, _| {
            job.definition.lifecycle = SchedulerJobLifecycleState::Active;
        })
    }

    async fn delete_job(
        &self,
        command: SchedulerDeleteJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        let trace = command.trace.clone();
        Ok(self.store.write(|state| {
            let Some(job) = state.jobs.get(&command.job_id) else {
                warn!(
                    service_id = SCHEDULER_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    job_id = command.job_id.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local scheduler job delete rejected because job is unknown"
                );
                return self.error_result(
                    trace,
                    AutonomyServiceErrorKind::InvalidRequest,
                    "job_not_found",
                    "scheduler job was not found",
                );
            };
            if job.definition.scope != command.scope {
                return self.error_result(
                    trace,
                    AutonomyServiceErrorKind::Denied,
                    "scope_mismatch",
                    "scheduler job does not belong to the requested scope",
                );
            }
            state.jobs.remove(&command.job_id);
            let audit_id = state.record_audit("job.deleted");
            info!(
                service_id = SCHEDULER_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                job_id = command.job_id.as_str(),
                audit_id = audit_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                reason_code = command.reason_code.as_str(),
                "local scheduler job deleted"
            );
            let mut result = self.result(
                trace,
                Some(command.job_id),
                None,
                Some(SchedulerJobLifecycleState::Deleted),
                None,
            );
            result.audit_id = Some(audit_id);
            result
        }))
    }

    async fn trigger_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        let trace = command.trace.clone();
        Ok(self.store.write(|state| {
            if !state.jobs.contains_key(&command.job_id) {
                warn!(
                    service_id = SCHEDULER_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    job_id = command.job_id.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local scheduler trigger rejected because job is unknown"
                );
                return self.error_result(
                    trace,
                    AutonomyServiceErrorKind::InvalidRequest,
                    "job_not_found",
                    "scheduler job was not found",
                );
            }
            let run_id = state.next_run_id();
            let now = Utc::now();
            let audit_id = state.record_audit("run.triggered");
            let summary = macaca_proto::SchedulerRunSummary {
                run_id: run_id.clone(),
                job_id: command.job_id.clone(),
                state: SchedulerRunState::Queued,
                scheduled_for: now,
                started_at: None,
                finished_at: None,
                attempt: 0,
                trace_id: trace.trace_id.clone(),
                audit_id: Some(audit_id.clone()),
                safe_status: "queued by manual trigger".into(),
                metadata: command.metadata.clone(),
            };
            let lifecycle = {
                let job = state
                    .jobs
                    .get_mut(&command.job_id)
                    .expect("job existence was checked before queuing a manual run");
                job.updated_at = now;
                job.definition.lifecycle.clone()
            };
            state.runs.insert(run_id.clone(), StoredRun { summary });
            info!(
                service_id = SCHEDULER_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                job_id = command.job_id.as_str(),
                run_id = run_id.as_str(),
                audit_id = audit_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                "local scheduler queued manual run"
            );
            let mut result = self.result(
                trace,
                Some(command.job_id),
                Some(run_id),
                Some(lifecycle),
                Some(SchedulerRunState::Queued),
            );
            result.audit_id = Some(audit_id);
            result
        }))
    }

    async fn get_job(
        &self,
        command: SchedulerGetJobCommand,
    ) -> MacacaResult<Option<SchedulerJobSummary>> {
        let trace = command.trace.clone();
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            job_id = command.job_id.as_str(),
            trace_id = trace.trace_id.as_str(),
            "local scheduler job summary requested"
        );
        Ok(self.store.read(|state| {
            state
                .jobs
                .get(&command.job_id)
                .filter(|job| job.definition.scope == command.scope)
                .map(|job| job.summary(command.job_id.clone(), Some(trace.trace_id.clone()), None))
        }))
    }

    async fn list_jobs(
        &self,
        command: SchedulerListJobsCommand,
    ) -> MacacaResult<Vec<SchedulerJobSummary>> {
        self.refresh_due_runs(&command.trace);
        Ok(self.store.read(|state| {
            state
                .jobs
                .iter()
                .filter(|(_, job)| job.definition.scope == command.scope)
                .take(command.limit)
                .map(|(job_id, job)| {
                    job.summary(job_id.clone(), Some(command.trace.trace_id.clone()), None)
                })
                .collect()
        }))
    }

    async fn get_run(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        let trace = command.trace.clone();
        let Some(run_id) = command.run_id else {
            return Ok(self.error_result(
                trace,
                AutonomyServiceErrorKind::InvalidRequest,
                "missing_run_id",
                "scheduler get_run requires run_id",
            ));
        };
        let result = self.store.read(|state| {
            state.runs.get(&run_id).map(|run| {
                self.result(
                    trace.clone(),
                    Some(run.summary.job_id.clone()),
                    Some(run_id.clone()),
                    None,
                    Some(run.summary.state.clone()),
                )
            })
        });
        Ok(result.unwrap_or_else(|| {
            self.error_result(
                trace,
                AutonomyServiceErrorKind::InvalidRequest,
                "run_not_found",
                "scheduler run was not found",
            )
        }))
    }

    async fn list_runs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerRunSummary>> {
        self.refresh_due_runs(&command.trace);
        Ok(self.store.read(|state| {
            state
                .runs
                .values()
                .rev()
                .filter(|run| {
                    state
                        .jobs
                        .get(&run.summary.job_id)
                        .map(|job| job.definition.scope == command.scope)
                        .unwrap_or(false)
                        && command
                            .job_id
                            .as_ref()
                            .map(|job_id| &run.summary.job_id == job_id)
                            .unwrap_or(true)
                })
                .take(command.limit.unwrap_or(100))
                .map(|run| run.summary.clone())
                .collect()
        }))
    }
}

impl super::InProcessSchedulerProvider {
    /// Apply a lifecycle mutation under scope checks and auditable state transition.
    pub(super) fn mutate_job<F>(
        &self,
        command: SchedulerLifecycleJobCommand,
        action: &'static str,
        mutate: F,
    ) -> MacacaResult<SchedulerCommandResult>
    where
        F: FnOnce(&mut StoredJob, &str),
    {
        let trace = command.trace.clone();
        Ok(self.store.write(|state| {
            let Some(job) = state.jobs.get_mut(&command.job_id) else {
                warn!(
                    service_id = SCHEDULER_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    job_id = command.job_id.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local scheduler job mutation rejected because job is unknown"
                );
                return self.error_result(
                    trace,
                    AutonomyServiceErrorKind::InvalidRequest,
                    "job_not_found",
                    "scheduler job was not found",
                );
            };
            if job.definition.scope != command.scope {
                return self.error_result(
                    trace,
                    AutonomyServiceErrorKind::Denied,
                    "scope_mismatch",
                    "scheduler job does not belong to the requested scope",
                );
            }
            mutate(job, &command.reason_code);
            job.updated_at = Utc::now();
            let lifecycle = job.definition.lifecycle.clone();
            let audit_id = state.record_audit(match action {
                "updated" => "job.updated",
                "paused" => "job.paused",
                "resumed" => "job.resumed",
                "deleted" => "job.deleted",
                _ => "job.mutated",
            });
            info!(
                service_id = SCHEDULER_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                job_id = command.job_id.as_str(),
                action,
                audit_id = audit_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                "local scheduler job lifecycle mutation completed"
            );
            let mut result = self.result(trace, Some(command.job_id), None, Some(lifecycle), None);
            result.audit_id = Some(audit_id);
            result
        }))
    }
}
