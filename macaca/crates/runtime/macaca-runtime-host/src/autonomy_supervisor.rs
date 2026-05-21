//! Lifecycle-managed local autonomy supervisor.
//!
//! The supervisor is intentionally narrow.  It owns timer-loop coordination,
//! bounded lease acquisition, generic dispatch, heartbeat wake ticks, recovery
//! wakes, and shutdown cancellation.  Scheduler still owns due-run state and
//! leases.  Heartbeat still owns wake coalescing and gates.  The supervisor
//! never branches on application, workflow, provider, driver, model, gateway,
//! chain, payment, or business-domain names.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use macaca_heartbeat::{HeartbeatService, LocalHeartbeatProvider};
use macaca_proto::{
    AutonomyScope, HeartbeatWakeCommand, HeartbeatWakeIntent, MacacaResult, SchedulerRunState,
    TraceContext,
};
use macaca_scheduler::LocalSchedulerProvider;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::autonomy_dispatch::AutonomyDispatchStrategies;
use crate::autonomy_runtime_config::AutonomyRuntimeConfig;
use crate::ServiceRuntime;

/// Host-owned supervisor for explicit local autonomy activation.
#[derive(Clone)]
pub struct AutonomySupervisor {
    runtime: Arc<ServiceRuntime>,
    scheduler: Arc<LocalSchedulerProvider>,
    heartbeat: Arc<LocalHeartbeatProvider>,
    config: AutonomyRuntimeConfig,
    running: Arc<AtomicBool>,
    worker: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl AutonomySupervisor {
    /// Create a supervisor without starting the background loop.
    pub fn new(
        runtime: Arc<ServiceRuntime>,
        scheduler: Arc<LocalSchedulerProvider>,
        heartbeat: Arc<LocalHeartbeatProvider>,
        config: AutonomyRuntimeConfig,
    ) -> Self {
        Self {
            runtime,
            scheduler,
            heartbeat,
            config: config.normalized(),
            running: Arc::new(AtomicBool::new(false)),
            worker: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the background loop if configured.
    ///
    /// The loop waits one interval before the first tick.  This avoids a hidden
    /// immediate side effect during runtime-host bootstrap while still making
    /// the supervisor a real lifecycle-managed daemon.
    pub async fn start(&self, trace: TraceContext) -> MacacaResult<()> {
        if !self.config.supervisor_enabled {
            info!(
                trace_id = trace.trace_id.as_str(),
                mode = self.config.mode_label(),
                "autonomy supervisor background loop disabled by config"
            );
            return Ok(());
        }
        if self.running.swap(true, Ordering::SeqCst) {
            debug!(
                trace_id = trace.trace_id.as_str(),
                "autonomy supervisor start requested while already running"
            );
            return Ok(());
        }
        let supervisor = self.clone();
        let mut worker = self.worker.lock().await;
        *worker = Some(tokio::spawn(async move {
            supervisor.run_loop(trace).await;
        }));
        info!("autonomy supervisor background loop started");
        Ok(())
    }

    /// Stop the background loop and abort any sleeping worker after grace.
    pub async fn stop(&self, trace: TraceContext) {
        self.running.store(false, Ordering::SeqCst);
        let mut worker = self.worker.lock().await;
        if let Some(handle) = worker.take() {
            let grace = Duration::from_millis(self.config.shutdown_grace_ms);
            match tokio::time::timeout(grace, handle).await {
                Ok(Ok(())) => {
                    info!(
                        trace_id = trace.trace_id.as_str(),
                        "autonomy supervisor stopped cleanly"
                    );
                }
                Ok(Err(error)) => {
                    warn!(
                        trace_id = trace.trace_id.as_str(),
                        error = %error,
                        "autonomy supervisor worker ended with join error"
                    );
                }
                Err(_) => {
                    warn!(
                        trace_id = trace.trace_id.as_str(),
                        "autonomy supervisor stop exceeded shutdown grace"
                    );
                }
            }
        }
    }

    /// Return whether the background loop is currently marked running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Execute one bounded Scheduler tick for deterministic tests and manual wakeups.
    pub async fn run_scheduler_tick_once(&self, trace: TraceContext) -> MacacaResult<usize> {
        let mut dispatched = 0usize;
        self.scheduler.expire_leases(trace.clone())?;
        for _ in 0..self.config.max_leases_per_tick {
            let Some(leased) = self.scheduler.acquire_next_run_lease_with_target(
                trace.clone(),
                "runtime.autonomy_supervisor",
            )?
            else {
                debug!(
                    trace_id = trace.trace_id.as_str(),
                    "autonomy supervisor scheduler tick found no eligible run"
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
            dispatched,
            "autonomy supervisor scheduler tick completed"
        );
        Ok(dispatched)
    }

    /// Emit one provider-neutral Heartbeat scheduled tick.
    pub async fn run_heartbeat_tick_once(&self, trace: TraceContext) -> MacacaResult<bool> {
        let wake = HeartbeatWakeCommand::scheduled_tick(
            trace.clone(),
            AutonomyScope::global(),
            "runtime.autonomy_supervisor",
        )?;
        let result = self.heartbeat.wake(wake).await?;
        if let Some(run_id) = result.run_id.clone() {
            if result.accepted {
                self.heartbeat.mark_run_running(trace.clone(), run_id.clone())?;
                self.heartbeat.mark_run_succeeded(trace.clone(), run_id)?;
            }
        }
        info!(
            trace_id = trace.trace_id.as_str(),
            accepted = result.accepted,
            disposition = ?result.disposition,
            "autonomy supervisor heartbeat tick completed"
        );
        Ok(result.accepted)
    }

    /// Emit one provider-neutral recovery wake when enabled.
    pub async fn run_recovery_wake_once(&self, trace: TraceContext) -> MacacaResult<bool> {
        if !self.config.recovery_wake_enabled {
            info!(
                trace_id = trace.trace_id.as_str(),
                "autonomy supervisor recovery wake skipped by config"
            );
            return Ok(false);
        }
        let wake = HeartbeatWakeCommand::new(
            trace.clone(),
            AutonomyScope::global(),
            "runtime.autonomy_supervisor.recovery",
            HeartbeatWakeIntent::Recovery {
                reason_code: "runtime_host_startup".into(),
            },
        )?;
        let result = self.heartbeat.wake(wake).await?;
        info!(
            trace_id = trace.trace_id.as_str(),
            accepted = result.accepted,
            "autonomy supervisor recovery wake completed"
        );
        Ok(result.accepted)
    }

    async fn run_loop(self, trace: TraceContext) {
        while self.running.load(Ordering::SeqCst) {
            sleep(Duration::from_millis(self.config.scheduler_tick_interval_ms)).await;
            if !self.running.load(Ordering::SeqCst) {
                break;
            }
            if let Err(error) = self.run_scheduler_tick_once(trace.clone()).await {
                warn!(
                    trace_id = trace.trace_id.as_str(),
                    error = %error,
                    "autonomy supervisor scheduler tick failed"
                );
            }
            if let Err(error) = self.run_heartbeat_tick_once(trace.clone()).await {
                warn!(
                    trace_id = trace.trace_id.as_str(),
                    error = %error,
                    "autonomy supervisor heartbeat tick failed"
                );
            }
        }
        info!(
            trace_id = trace.trace_id.as_str(),
            final_state = ?SchedulerRunState::Skipped,
            "autonomy supervisor background loop exited"
        );
    }
}
