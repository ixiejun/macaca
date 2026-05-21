//! Scheduler lane for the local autonomy supervisor.
//!
//! The lane owns only Scheduler-specific loop work: lease expiry, due-run lease
//! acquisition, and provider-neutral target dispatch through runtime-host
//! strategies. It is deliberately separate from HeartbeatLane so Scheduler can
//! continue operating without becoming the owner of heartbeat cadence.

use std::sync::Arc;

use macaca_proto::{MacacaResult, TraceContext};
use macaca_scheduler::LocalSchedulerProvider;
use tracing::{debug, info};

use crate::autonomy_dispatch::AutonomyDispatchStrategies;
use crate::autonomy_runtime_config::AutonomyRuntimeConfig;
use crate::ServiceRuntime;

/// Runtime-host Strategy object for one bounded Scheduler supervisor tick.
pub(crate) struct SchedulerLane {
    runtime: Arc<ServiceRuntime>,
    scheduler: Arc<LocalSchedulerProvider>,
    config: AutonomyRuntimeConfig,
}

impl SchedulerLane {
    /// Build a Scheduler lane from approved runtime-host composition inputs.
    pub(crate) fn new(
        runtime: Arc<ServiceRuntime>,
        scheduler: Arc<LocalSchedulerProvider>,
        config: AutonomyRuntimeConfig,
    ) -> Self {
        Self {
            runtime,
            scheduler,
            config,
        }
    }

    /// Execute one bounded Scheduler tick.
    ///
    /// Scheduler owns due-run state and leases; this lane only coordinates the
    /// host loop and dispatch Strategy. It never evaluates Heartbeat native
    /// cadence and never branches on application-specific semantics.
    pub(crate) async fn tick_once(&self, trace: TraceContext) -> MacacaResult<usize> {
        let mut dispatched = 0usize;
        self.scheduler.expire_leases(trace.clone())?;
        for _ in 0..self.config.max_leases_per_tick {
            let Some(leased) = self
                .scheduler
                .acquire_next_run_lease_with_target(trace.clone(), "runtime.autonomy_supervisor")?
            else {
                debug!(
                    trace_id = trace.trace_id.as_str(),
                    "autonomy scheduler lane found no eligible run"
                );
                break;
            };
            self.scheduler
                .mark_run_running(trace.clone(), leased.summary.run_id.clone())?;
            let outcome = AutonomyDispatchStrategies::new(
                self.runtime.as_ref(),
                self.config.dispatch_timeout_ms,
            )
            .dispatch(trace.clone(), leased.scope, leased.target)
            .await?;
            if outcome.succeeded {
                self.scheduler
                    .mark_run_succeeded(trace.clone(), leased.summary.run_id)?;
            } else if outcome.retryable {
                self.scheduler.mark_run_failed(
                    trace.clone(),
                    leased.summary.run_id,
                    true,
                    outcome.reason_code,
                )?;
            } else {
                self.scheduler.cancel_run(
                    trace.clone(),
                    leased.summary.run_id,
                    outcome.reason_code,
                )?;
            }
            dispatched += 1;
        }
        info!(
            trace_id = trace.trace_id.as_str(),
            dispatched, "autonomy scheduler lane tick completed"
        );
        Ok(dispatched)
    }
}
