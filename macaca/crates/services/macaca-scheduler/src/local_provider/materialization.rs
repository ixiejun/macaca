//! Due-run materialization and snapshot assembly for the local Scheduler provider.
//!
//! This module owns the Scheduler-side half of the supervisor loop: it converts
//! active job definitions into queued run mementos with sanitized audit ids.
//! It deliberately does not acquire leases, dispatch targets, or interpret
//! application behavior.

use std::collections::BTreeMap;

use chrono::Utc;
use macaca_proto::{
    MacacaResult, SchedulerJobLifecycleState, SchedulerRunState, SchedulerServiceSnapshot,
    TraceContext, SCHEDULER_SERVICE_ID,
};
use tracing::{debug, info};

use super::schedule::apply_stagger;
use super::store::StoredRun;
use super::support::LOCAL_PROVIDER_ID;

impl super::InProcessSchedulerProvider {
    /// Refresh due jobs before read operations or dispatch ticks.
    ///
    /// The method only materializes queued run mementos.  It does not dispatch
    /// target commands, acquire external resources, or call application code.
    pub(super) fn refresh_due_runs(&self, trace: &TraceContext) {
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
                    let summary = macaca_proto::SchedulerRunSummary {
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

    /// Materialize due runs for a runtime-host dispatch tick.
    ///
    /// Runtime Host calls this before lease acquisition so background ticks can
    /// advance jobs without depending on a Web list/read request to refresh
    /// due-time state.
    pub fn materialize_due_runs_for_dispatch(&self, trace: TraceContext) -> MacacaResult<()> {
        self.refresh_due_runs(&trace);
        Ok(())
    }

    /// Build a sanitized service snapshot from the current in-memory memento.
    ///
    /// Exposed to contract tests as `pub(super)` so modules can verify boundary
    /// behavior without widening the public crate API.
    pub(super) fn snapshot_inner(&self) -> SchedulerServiceSnapshot {
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
