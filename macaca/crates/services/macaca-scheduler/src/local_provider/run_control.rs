//! Run-control state transitions for the local Scheduler provider.
//!
//! This module keeps lease, dispatch-boundary, retry, cancellation, and expiry
//! behavior separate from job registration/query code.  The split is deliberate:
//! run control is a State + Memento concern, while schedule calculation is a
//! Strategy concern and service dispatch will be a future Decorator-wrapped
//! runtime concern.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use macaca_proto::{
    AutonomyServiceErrorKind, MacacaError, MacacaResult, SchedulerCommandResult,
    SchedulerRetryPolicy, SchedulerRunId, SchedulerRunState, SchedulerRunSummary, TraceContext,
    SCHEDULER_SERVICE_ID,
};
use tracing::{info, warn};

use super::{
    InProcessSchedulerLeasedRun, InProcessSchedulerProvider, LocalSchedulerState, LOCAL_PROVIDER_ID,
};

const LEASE_OWNER_KEY: &str = "lease_owner";
const LEASE_EXPIRES_AT_KEY: &str = "lease_expires_at";
const RETRY_FOR_RUN_ID_KEY: &str = "retry_for_run_id";
const RUN_REASON_CODE_KEY: &str = "reason_code";

impl InProcessSchedulerProvider {
    /// Acquire the next queued run without dispatching its target command.
    ///
    /// This method is a provider-neutral lease primitive.  Future dispatchers
    /// can call it before crossing into service-runtime side effects, but this
    /// provider itself only records the lease memento and emits sanitized logs.
    pub fn acquire_next_run_lease(
        &self,
        trace: TraceContext,
        lease_owner: impl Into<String>,
    ) -> MacacaResult<Option<SchedulerRunSummary>> {
        Ok(self
            .acquire_next_run_lease_with_target(trace, lease_owner)?
            .map(|leased| leased.summary))
    }

    /// Acquire the next queued run and return its provider-neutral target.
    ///
    /// The returned target is cloned from the job definition after the lease is
    /// recorded.  This keeps runtime-host dispatch decoupled from Scheduler
    /// internals while avoiding raw payload exposure: targets carry only typed
    /// command identities, metadata, and optional payload references.
    pub fn acquire_next_run_lease_with_target(
        &self,
        trace: TraceContext,
        lease_owner: impl Into<String>,
    ) -> MacacaResult<Option<InProcessSchedulerLeasedRun>> {
        let lease_owner = normalize_label(lease_owner.into(), "scheduler lease_owner is required")?;
        self.refresh_due_runs(&trace);
        Ok(self.store.write(|state| {
            expire_locked(state, Utc::now(), &trace);
            let now = Utc::now();
            let Some((run_id, lease_timeout_ms)) = next_lease_candidate(state) else {
                info!(
                    service_id = SCHEDULER_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    trace_id = trace.trace_id.as_str(),
                    "local scheduler found no queued run eligible for lease acquisition"
                );
                return None;
            };
            let job_id = state
                .runs
                .get(&run_id)
                .expect("lease candidate was selected from existing runs")
                .summary
                .job_id
                .clone();
            let Some(job) = state.jobs.get(&job_id) else {
                warn!(
                    service_id = SCHEDULER_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    run_id = run_id.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local scheduler lease candidate skipped because job definition is missing"
                );
                return None;
            };
            let scope = job.definition.scope.clone();
            let target = job.definition.target.clone();
            let audit_id = state.record_audit("run.leased");
            let lease_timeout_ms = lease_timeout_ms.min(i64::MAX as u64) as i64;
            let lease_expires_at = now + Duration::milliseconds(lease_timeout_ms);
            let run = state
                .runs
                .get_mut(&run_id)
                .expect("lease candidate was selected from existing runs");
            run.summary.state = SchedulerRunState::Leased;
            run.summary.started_at = Some(now);
            run.summary.attempt = run.summary.attempt.saturating_add(1);
            run.summary.audit_id = Some(audit_id.clone());
            run.summary.safe_status = "leased by local scheduler run control".into();
            run.summary
                .metadata
                .insert(LEASE_OWNER_KEY.into(), lease_owner.clone());
            run.summary
                .metadata
                .insert(LEASE_EXPIRES_AT_KEY.into(), lease_expires_at.to_rfc3339());
            info!(
                service_id = SCHEDULER_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                run_id = run_id.as_str(),
                lease_owner = lease_owner.as_str(),
                audit_id = audit_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                "local scheduler acquired run lease"
            );
            Some(InProcessSchedulerLeasedRun {
                summary: run.summary.clone(),
                scope,
                target,
            })
        }))
    }

    /// Mark a leased run as entering the dispatch boundary.
    ///
    /// The method is intentionally named around the boundary, not the target
    /// service.  Scheduler must not know whether a future dispatcher calls an
    /// agent, application capability, plugin, or another system service.
    pub fn mark_run_running(
        &self,
        trace: TraceContext,
        run_id: SchedulerRunId,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.transition_run(
            trace,
            run_id,
            SchedulerRunState::Running,
            "running",
            |run, now| {
                run.summary.started_at.get_or_insert(now);
                run.summary.safe_status = "dispatch boundary entered by local scheduler".into();
            },
        )
    }

    /// Mark a run as completed successfully and clear lease metadata.
    pub fn mark_run_succeeded(
        &self,
        trace: TraceContext,
        run_id: SchedulerRunId,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.transition_run(
            trace,
            run_id,
            SchedulerRunState::Succeeded,
            "succeeded",
            |run, now| {
                run.summary.finished_at = Some(now);
                run.summary.safe_status = "completed by local scheduler run control".into();
                clear_lease_metadata(&mut run.summary.metadata);
            },
        )
    }

    /// Mark a run as failed and optionally queue a bounded retry memento.
    ///
    /// Retry calculation is provider-neutral: it uses the job's retry strategy
    /// and creates another queued run record.  It does not re-enter business
    /// execution by itself.
    pub fn mark_run_failed(
        &self,
        trace: TraceContext,
        run_id: SchedulerRunId,
        retryable: bool,
        reason_code: impl Into<String>,
    ) -> MacacaResult<SchedulerCommandResult> {
        let reason_code =
            normalize_label(reason_code.into(), "scheduler failure reason is required")?;
        Ok(self.store.write(|state| {
            let now = Utc::now();
            let failure_audit_id = state.record_audit("run.failed");
            let retry_plan = {
                let Some(run) = state.runs.get_mut(&run_id) else {
                    return self.error_result(
                        trace,
                        AutonomyServiceErrorKind::InvalidRequest,
                        "run_not_found",
                        "scheduler run was not found",
                    );
                };
                run.summary.state = SchedulerRunState::Failed;
                run.summary.finished_at = Some(now);
                run.summary.audit_id = Some(failure_audit_id.clone());
                run.summary.safe_status = "failed by local scheduler run control".into();
                run.summary
                    .metadata
                    .insert(RUN_REASON_CODE_KEY.into(), reason_code.clone());
                clear_lease_metadata(&mut run.summary.metadata);
                let job_id = run.summary.job_id.clone();
                let attempt = run.summary.attempt;
                let scheduled_for = state
                    .jobs
                    .get(&job_id)
                    .and_then(|job| retry_due_at(&job.definition.retry, attempt, retryable, now));
                scheduled_for.map(|scheduled_for| (job_id, attempt, scheduled_for))
            };
            if let Some((job_id, attempt, scheduled_for)) = retry_plan {
                let retry_run_id = state.next_run_id();
                let retry_audit_id = state.record_audit("run.retry_queued");
                let mut metadata = BTreeMap::new();
                metadata.insert(RETRY_FOR_RUN_ID_KEY.into(), run_id.as_str().into());
                metadata.insert(RUN_REASON_CODE_KEY.into(), reason_code.clone());
                let retry_summary = SchedulerRunSummary {
                    run_id: retry_run_id.clone(),
                    job_id: job_id.clone(),
                    state: SchedulerRunState::Queued,
                    scheduled_for,
                    started_at: None,
                    finished_at: None,
                    attempt,
                    trace_id: trace.trace_id.clone(),
                    audit_id: Some(retry_audit_id.clone()),
                    safe_status: "queued by retry backoff".into(),
                    metadata,
                };
                state.runs.insert(
                    retry_run_id.clone(),
                    super::StoredRun {
                        summary: retry_summary,
                    },
                );
                info!(
                    service_id = SCHEDULER_SERVICE_ID,
                    provider_id = LOCAL_PROVIDER_ID,
                    run_id = run_id.as_str(),
                    retry_run_id = retry_run_id.as_str(),
                    audit_id = retry_audit_id.as_str(),
                    trace_id = trace.trace_id.as_str(),
                    "local scheduler queued retry run after failure"
                );
                let mut result = self.result(
                    trace,
                    Some(job_id),
                    Some(retry_run_id),
                    None,
                    Some(SchedulerRunState::Queued),
                );
                result.audit_id = Some(retry_audit_id);
                return result;
            }
            warn!(
                service_id = SCHEDULER_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                run_id = run_id.as_str(),
                reason_code = reason_code.as_str(),
                trace_id = trace.trace_id.as_str(),
                "local scheduler marked run failed without retry"
            );
            let mut result = self.result(
                trace,
                None,
                Some(run_id),
                None,
                Some(SchedulerRunState::Failed),
            );
            result.audit_id = Some(failure_audit_id);
            result
        }))
    }

    /// Cancel a queued, leased, or running run through an auditable transition.
    pub fn cancel_run(
        &self,
        trace: TraceContext,
        run_id: SchedulerRunId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<SchedulerCommandResult> {
        let reason_code =
            normalize_label(reason_code.into(), "scheduler cancel reason is required")?;
        self.transition_run(
            trace,
            run_id,
            SchedulerRunState::Cancelled,
            "cancelled",
            |run, now| {
                run.summary.finished_at = Some(now);
                run.summary.safe_status = "cancelled by local scheduler run control".into();
                run.summary
                    .metadata
                    .insert(RUN_REASON_CODE_KEY.into(), reason_code.clone());
                clear_lease_metadata(&mut run.summary.metadata);
            },
        )
    }

    /// Expire all leases whose timeout has elapsed.
    pub fn expire_leases(&self, trace: TraceContext) -> MacacaResult<usize> {
        Ok(self
            .store
            .write(|state| expire_locked(state, Utc::now(), &trace)))
    }

    fn transition_run<F>(
        &self,
        trace: TraceContext,
        run_id: SchedulerRunId,
        next_state: SchedulerRunState,
        action: &'static str,
        mutate: F,
    ) -> MacacaResult<SchedulerCommandResult>
    where
        F: FnOnce(&mut super::StoredRun, DateTime<Utc>),
    {
        Ok(self.store.write(|state| {
            let audit_id = state.record_audit(match next_state {
                SchedulerRunState::Running => "run.running",
                SchedulerRunState::Succeeded => "run.succeeded",
                SchedulerRunState::Cancelled => "run.cancelled",
                SchedulerRunState::Skipped => "run.skipped",
                SchedulerRunState::Expired => "run.expired",
                SchedulerRunState::Failed => "run.failed",
                SchedulerRunState::Queued => "run.queued",
                SchedulerRunState::Leased => "run.leased",
            });
            let Some(run) = state.runs.get_mut(&run_id) else {
                return self.error_result(
                    trace,
                    AutonomyServiceErrorKind::InvalidRequest,
                    "run_not_found",
                    "scheduler run was not found",
                );
            };
            let job_id = run.summary.job_id.clone();
            let now = Utc::now();
            run.summary.state = next_state.clone();
            run.summary.audit_id = Some(audit_id.clone());
            mutate(run, now);
            info!(
                service_id = SCHEDULER_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                run_id = run_id.as_str(),
                action,
                audit_id = audit_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                "local scheduler run lifecycle transition completed"
            );
            let mut result = self.result(trace, Some(job_id), Some(run_id), None, Some(next_state));
            result.audit_id = Some(audit_id);
            result
        }))
    }
}

fn next_lease_candidate(state: &LocalSchedulerState) -> Option<(SchedulerRunId, u64)> {
    state.runs.iter().find_map(|(run_id, run)| {
        if run.summary.state != SchedulerRunState::Queued {
            return None;
        }
        let job = state.jobs.get(&run.summary.job_id)?;
        let max_active_runs = job.definition.concurrency.max_active_runs.max(1);
        let active_runs = state
            .runs
            .values()
            .filter(|candidate| {
                candidate.summary.job_id == run.summary.job_id
                    && matches!(
                        candidate.summary.state,
                        SchedulerRunState::Leased | SchedulerRunState::Running
                    )
            })
            .count() as u32;
        if active_runs >= max_active_runs {
            return None;
        }
        Some((run_id.clone(), job.definition.concurrency.lease_timeout_ms))
    })
}

fn expire_locked(
    state: &mut LocalSchedulerState,
    now: DateTime<Utc>,
    trace: &TraceContext,
) -> usize {
    let mut expired = 0;
    for run in state.runs.values_mut() {
        if !matches!(
            run.summary.state,
            SchedulerRunState::Leased | SchedulerRunState::Running
        ) {
            continue;
        }
        let Some(expires_at) = run
            .summary
            .metadata
            .get(LEASE_EXPIRES_AT_KEY)
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
        else {
            continue;
        };
        if expires_at > now {
            continue;
        }
        run.summary.state = SchedulerRunState::Expired;
        run.summary.finished_at = Some(now);
        run.summary.safe_status = "lease expired by local scheduler recovery".into();
        clear_lease_metadata(&mut run.summary.metadata);
        expired += 1;
        warn!(
            service_id = SCHEDULER_SERVICE_ID,
            provider_id = LOCAL_PROVIDER_ID,
            run_id = run.summary.run_id.as_str(),
            trace_id = trace.trace_id.as_str(),
            "local scheduler expired stale run lease"
        );
    }
    expired
}

fn retry_due_at(
    policy: &SchedulerRetryPolicy,
    attempt: u32,
    retryable: bool,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if !retryable || attempt >= policy.max_attempts {
        return None;
    }
    Some(now + retry_delay(policy, attempt))
}

fn retry_delay(policy: &SchedulerRetryPolicy, attempt: u32) -> Duration {
    let mut delay = policy.initial_backoff_ms.max(1);
    for _ in 1..attempt.max(1) {
        delay = delay
            .saturating_mul(policy.multiplier_basis_points.max(1) as u64)
            .saturating_div(10_000)
            .max(1)
            .min(policy.max_backoff_ms.max(1));
    }
    Duration::milliseconds(delay.min(i64::MAX as u64) as i64)
}

fn clear_lease_metadata(metadata: &mut BTreeMap<String, String>) {
    metadata.remove(LEASE_OWNER_KEY);
    metadata.remove(LEASE_EXPIRES_AT_KEY);
}

fn normalize_label(value: String, message: &'static str) -> MacacaResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(value)
}
