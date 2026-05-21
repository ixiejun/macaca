//! In-memory local Scheduler provider.
//!
//! This provider is the first concrete Scheduler implementation slice.  It is
//! intentionally conservative: it stores provider-neutral job definitions,
//! materializes due runs into an auditable run-history memento, and exposes
//! health/snapshot/history.  It does not execute target commands yet.  Dispatch
//! must be added later through service-runtime decorators so policy, resource,
//! trace, and audit gates wrap every side effect.

use std::collections::BTreeMap;
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macaca_proto::{
    AutonomyAuditCorrelation, AutonomyServiceErrorKind, AutonomyStructuredError,
    SchedulerCommandResult, SchedulerDeleteJobCommand, SchedulerGetJobCommand, SchedulerJobCommand,
    SchedulerJobDefinition, SchedulerJobId, SchedulerJobLifecycleState, SchedulerJobSummary,
    SchedulerLifecycleJobCommand, SchedulerListJobsCommand, SchedulerQueryCommand,
    SchedulerRegisterJobCommand, SchedulerRunId, SchedulerRunState, SchedulerRunSummary,
    SchedulerServiceSnapshot, SchedulerTargetCommand, SchedulerUpdateJobCommand, ServiceDescriptor,
    ServiceHealth, ServiceLifecycleState, TraceContext, SCHEDULER_SERVICE_ID,
};
use tracing::{debug, info, warn};

use crate::service_contract::SchedulerService;

mod run_control;
mod schedule;

use schedule::{apply_stagger, DefaultScheduleCalculator};

/// Local provider identifier used in sanitized snapshots.
const LOCAL_PROVIDER_ID: &str = "local.in_memory";

/// Maximum number of due runs materialized in one refresh pass.
///
/// The bound prevents a restarted provider from creating an unbounded burst of
/// run mementos if a very old interval job is resumed.  Job-level
/// `CatchUp.max_runs` may lower this ceiling further.
const MATERIALIZATION_LIMIT: usize = 64;

/// Maximum number of sanitized audit identifiers retained in snapshots.
///
/// Local memory must remain bounded because scheduler snapshots can be read by
/// shells and diagnostics.  The audit id itself is safe metadata; raw command
/// payloads and target details remain outside this memento.
const AUDIT_RETENTION_LIMIT: usize = 128;

/// Local Scheduler provider backed by an in-memory memento store.
///
/// The provider uses the Memento pattern for job/run state, the State pattern
/// for job and run lifecycle values, the Strategy pattern for schedule
/// calculation, and Observer-style `tracing` logs for key execution nodes.
pub struct LocalSchedulerProvider {
    descriptor: ServiceDescriptor,
    store: InMemorySchedulerStore,
    calculator: DefaultScheduleCalculator,
}

/// Leased Scheduler run plus the generic target data required by dispatchers.
///
/// This memento is intentionally payload-reference-only.  Runtime-host can use
/// it to dispatch through service boundaries, but the Scheduler still does not
/// execute application behavior or expose raw target payloads in snapshots.
#[derive(Debug, Clone)]
pub struct LocalSchedulerLeasedRun {
    pub summary: SchedulerRunSummary,
    pub scope: macaca_proto::AutonomyScope,
    pub target: SchedulerTargetCommand,
}

impl LocalSchedulerProvider {
    /// Create an empty local provider with the standard Scheduler descriptor.
    pub fn new() -> Self {
        let unavailable = crate::UnavailableSchedulerProvider::default();
        let mut descriptor = unavailable.descriptor();
        descriptor.health = ServiceHealth::Healthy;
        descriptor.lifecycle_state = ServiceLifecycleState::Running;
        descriptor
            .metadata
            .insert("provider_id".into(), LOCAL_PROVIDER_ID.into());
        Self {
            descriptor,
            store: InMemorySchedulerStore::default(),
            calculator: DefaultScheduleCalculator,
        }
    }

    /// Refresh due jobs before read operations.
    ///
    /// The method only materializes queued run mementos.  It does not dispatch
    /// target commands, acquire external resources, or call application code.
    fn refresh_due_runs(&self, trace: &TraceContext) {
        let now = Utc::now();
        self.store.write(|state| {
            let jobs = state.jobs.keys().cloned().collect::<Vec<_>>();
            for job_id in jobs {
                let due = {
                    let Some(job) = state.jobs.get_mut(&job_id) else {
                        continue;
                    };
                    if job.definition.lifecycle != SchedulerJobLifecycleState::Active {
                        continue;
                    }
                    let due = self.calculator.due_times(job, now);
                    let due = apply_stagger(&job.definition, due);
                    debug!(
                        service_id = SCHEDULER_SERVICE_ID,
                        provider_id = LOCAL_PROVIDER_ID,
                        job_id = job_id.as_str(),
                        due_count = due.len(),
                        trace_id = trace.trace_id.as_str(),
                        "local scheduler due-work calculation completed"
                    );
                    if let Some(last_due) = due.last().cloned() {
                        job.last_scheduled_at = Some(last_due);
                        job.updated_at = now;
                    }
                    due
                };
                for scheduled_for in due {
                    let run_id = state.next_run_id();
                    let audit_id = state.record_audit("run.materialized");
                    let summary = SchedulerRunSummary {
                        run_id: run_id.clone(),
                        job_id: job_id.clone(),
                        state: SchedulerRunState::Queued,
                        scheduled_for,
                        started_at: None,
                        finished_at: None,
                        attempt: 0,
                        trace_id: trace.trace_id.clone(),
                        audit_id: Some(audit_id),
                        safe_status: "queued by local scheduler due-time materialization".into(),
                        metadata: BTreeMap::new(),
                    };
                    info!(
                        service_id = SCHEDULER_SERVICE_ID,
                        provider_id = LOCAL_PROVIDER_ID,
                        job_id = job_id.as_str(),
                        run_id = run_id.as_str(),
                        trace_id = trace.trace_id.as_str(),
                        "local scheduler materialized due run"
                    );
                    state.runs.insert(run_id, StoredRun { summary });
                }
            }
        });
    }

    fn result(
        &self,
        trace: TraceContext,
        job_id: Option<SchedulerJobId>,
        run_id: Option<SchedulerRunId>,
        lifecycle: Option<SchedulerJobLifecycleState>,
        run_state: Option<SchedulerRunState>,
    ) -> SchedulerCommandResult {
        SchedulerCommandResult {
            job_id,
            run_id,
            lifecycle,
            run_state,
            accepted: true,
            error: None,
            trace,
            audit_id: None,
            metadata: BTreeMap::new(),
        }
    }

    fn error_result(
        &self,
        trace: TraceContext,
        kind: AutonomyServiceErrorKind,
        reason_code: &'static str,
        safe_message: impl Into<String>,
    ) -> SchedulerCommandResult {
        let correlation = AutonomyAuditCorrelation::from_trace(trace.clone()).ok();
        SchedulerCommandResult {
            job_id: None,
            run_id: None,
            lifecycle: None,
            run_state: None,
            accepted: false,
            error: correlation.map(|correlation| AutonomyStructuredError {
                kind,
                reason_code: reason_code.into(),
                safe_message: safe_message.into(),
                correlation,
                metadata: BTreeMap::new(),
            }),
            trace,
            audit_id: None,
            metadata: BTreeMap::new(),
        }
    }

    fn snapshot_inner(&self) -> SchedulerServiceSnapshot {
        self.store.read(|state| {
            let active_jobs = state
                .jobs
                .values()
                .filter(|job| job.definition.lifecycle == SchedulerJobLifecycleState::Active)
                .count();
            let paused_jobs = state
                .jobs
                .values()
                .filter(|job| job.definition.lifecycle == SchedulerJobLifecycleState::Paused)
                .count();
            let queued_runs = state
                .runs
                .values()
                .filter(|run| run.summary.state == SchedulerRunState::Queued)
                .count();
            let active_runs = state
                .runs
                .values()
                .filter(|run| {
                    matches!(
                        run.summary.state,
                        SchedulerRunState::Leased | SchedulerRunState::Running
                    )
                })
                .count();
            let recent_runs = state
                .runs
                .values()
                .rev()
                .take(25)
                .map(|run| run.summary.clone())
                .collect();
            let last_audit_ids = state.audit_ids.iter().rev().take(25).cloned().collect();
            SchedulerServiceSnapshot {
                service_id: SCHEDULER_SERVICE_ID.into(),
                provider_id: LOCAL_PROVIDER_ID.into(),
                healthy: true,
                lifecycle_state: "running".into(),
                active_jobs,
                paused_jobs,
                queued_runs,
                active_runs,
                recent_runs,
                last_audit_ids,
                captured_at: Utc::now(),
            }
        })
    }
}

impl Default for LocalSchedulerProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SchedulerService for LocalSchedulerProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn health(
        &self,
        trace: TraceContext,
    ) -> macaca_proto::MacacaResult<SchedulerServiceSnapshot> {
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
    ) -> macaca_proto::MacacaResult<SchedulerServiceSnapshot> {
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
    ) -> macaca_proto::MacacaResult<SchedulerCommandResult> {
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
    ) -> macaca_proto::MacacaResult<SchedulerCommandResult> {
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
    ) -> macaca_proto::MacacaResult<SchedulerCommandResult> {
        self.mutate_job(command, "paused", |job, _| {
            job.definition.lifecycle = SchedulerJobLifecycleState::Paused;
        })
    }

    async fn resume_job(
        &self,
        command: SchedulerLifecycleJobCommand,
    ) -> macaca_proto::MacacaResult<SchedulerCommandResult> {
        self.mutate_job(command, "resumed", |job, _| {
            job.definition.lifecycle = SchedulerJobLifecycleState::Active;
        })
    }

    async fn delete_job(
        &self,
        command: SchedulerDeleteJobCommand,
    ) -> macaca_proto::MacacaResult<SchedulerCommandResult> {
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
    ) -> macaca_proto::MacacaResult<SchedulerCommandResult> {
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
            let summary = SchedulerRunSummary {
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
    ) -> macaca_proto::MacacaResult<Option<SchedulerJobSummary>> {
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
    ) -> macaca_proto::MacacaResult<Vec<SchedulerJobSummary>> {
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
    ) -> macaca_proto::MacacaResult<SchedulerCommandResult> {
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
    ) -> macaca_proto::MacacaResult<Vec<SchedulerRunSummary>> {
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

impl LocalSchedulerProvider {
    fn mutate_job<F>(
        &self,
        command: SchedulerLifecycleJobCommand,
        action: &'static str,
        mutate: F,
    ) -> macaca_proto::MacacaResult<SchedulerCommandResult>
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

#[derive(Default)]
struct InMemorySchedulerStore {
    state: RwLock<LocalSchedulerState>,
}

impl InMemorySchedulerStore {
    fn read<R>(&self, f: impl FnOnce(&LocalSchedulerState) -> R) -> R {
        let guard = self
            .state
            .read()
            .expect("local scheduler state lock poisoned");
        f(&guard)
    }

    fn write<R>(&self, f: impl FnOnce(&mut LocalSchedulerState) -> R) -> R {
        let mut guard = self
            .state
            .write()
            .expect("local scheduler state lock poisoned");
        f(&mut guard)
    }
}

#[derive(Default)]
struct LocalSchedulerState {
    next_job_sequence: u64,
    next_run_sequence: u64,
    next_audit_sequence: u64,
    jobs: BTreeMap<SchedulerJobId, StoredJob>,
    runs: BTreeMap<SchedulerRunId, StoredRun>,
    audit_ids: Vec<String>,
}

impl LocalSchedulerState {
    fn next_job_id(&mut self) -> SchedulerJobId {
        self.next_job_sequence += 1;
        SchedulerJobId::new(format!("job-{}", self.next_job_sequence))
            .expect("generated scheduler job id is always non-empty")
    }

    fn next_run_id(&mut self) -> SchedulerRunId {
        self.next_run_sequence += 1;
        SchedulerRunId::new(format!("run-{}", self.next_run_sequence))
            .expect("generated scheduler run id is always non-empty")
    }

    /// Append a bounded sanitized audit id and return it to the caller.
    ///
    /// The local provider does not own a durable audit sink yet, so it records
    /// only replay-safe correlation identifiers in memory.  This memento is
    /// intentionally payload-free and can later be replaced by a persistent
    /// audit repository without changing the public Scheduler DTOs.
    fn record_audit(&mut self, action: &'static str) -> String {
        self.next_audit_sequence += 1;
        let audit_id = format!("audit.scheduler.{action}.{}", self.next_audit_sequence);
        self.audit_ids.push(audit_id.clone());
        if self.audit_ids.len() > AUDIT_RETENTION_LIMIT {
            let overflow = self.audit_ids.len() - AUDIT_RETENTION_LIMIT;
            self.audit_ids.drain(0..overflow);
        }
        audit_id
    }
}

struct StoredJob {
    definition: SchedulerJobDefinition,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_scheduled_at: Option<DateTime<Utc>>,
}

impl StoredJob {
    fn new(definition: SchedulerJobDefinition, now: DateTime<Utc>) -> Self {
        Self {
            definition,
            created_at: now,
            updated_at: now,
            last_scheduled_at: None,
        }
    }

    /// Convert provider-owned job state into the sanitized read model used by
    /// SDK and shell clients.
    fn summary(
        &self,
        job_id: SchedulerJobId,
        trace_id: Option<String>,
        audit_id: Option<String>,
    ) -> SchedulerJobSummary {
        SchedulerJobSummary {
            job_id,
            scope: self.definition.scope.clone(),
            schedule: self.definition.schedule.clone(),
            target: self.definition.target.clone(),
            lifecycle: self.definition.lifecycle.clone(),
            metadata: self.definition.metadata.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_scheduled_at: self.last_scheduled_at,
            trace_id,
            audit_id,
        }
    }
}

struct StoredRun {
    summary: SchedulerRunSummary,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use macaca_proto::{
        AutonomyScope, KernelServiceId, SchedulerRegisterJobCommand, SchedulerScheduleSpec,
        ServiceCommandName, ServiceTargetCommand,
    };

    fn trace() -> TraceContext {
        TraceContext::new("trace-local-scheduler-boundary-test")
    }

    fn service_target() -> SchedulerTargetCommand {
        SchedulerTargetCommand::Service(ServiceTargetCommand {
            service_id: KernelServiceId::new("service.boundary.test"),
            command_name: ServiceCommandName::new("boundary.test.command"),
            payload_ref: None,
            metadata: BTreeMap::new(),
        })
    }

    #[tokio::test]
    async fn scheduler_does_not_materialize_native_heartbeat_cadence() {
        let provider = LocalSchedulerProvider::new();
        let definition = SchedulerJobDefinition::new(
            AutonomyScope::global(),
            SchedulerScheduleSpec::Every {
                interval_ms: 1_000,
                anchor: Some(Utc::now() - Duration::seconds(2)),
            },
            service_target(),
        )
        .unwrap();
        provider
            .register_job(SchedulerRegisterJobCommand::new(trace(), definition).unwrap())
            .await
            .unwrap();

        let snapshot = provider.snapshot_inner();

        assert!(snapshot.recent_runs.iter().all(|run| {
            run.safe_status != "native heartbeat cadence accepted"
                && run.safe_status != "native heartbeat cadence gated"
        }));
    }

    #[tokio::test]
    async fn scheduler_preserves_generic_service_target_dispatch() {
        let provider = LocalSchedulerProvider::new();
        let definition = SchedulerJobDefinition::new(
            AutonomyScope::global(),
            SchedulerScheduleSpec::At {
                run_at: Utc::now() - Duration::seconds(1),
            },
            service_target(),
        )
        .unwrap();
        provider
            .register_job(SchedulerRegisterJobCommand::new(trace(), definition).unwrap())
            .await
            .unwrap();

        let leased = provider
            .acquire_next_run_lease_with_target(trace(), "scheduler.boundary.test")
            .unwrap()
            .expect("due service target should lease");

        assert!(matches!(leased.target, SchedulerTargetCommand::Service(_)));
    }
}
