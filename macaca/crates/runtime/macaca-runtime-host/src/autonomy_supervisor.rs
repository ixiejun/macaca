//! Lifecycle-managed local autonomy supervisor.
//!
//! The supervisor is intentionally narrow.  It owns timer-loop coordination,
//! bounded Scheduler lane ticks, native Heartbeat lane ticks, recovery wakes,
//! and shutdown cancellation. Scheduler still owns due-run state and leases.
//! Heartbeat now owns native cadence, profile evaluation, wake coalescing, and
//! gates. The supervisor never branches on application, workflow, provider,
//! driver, model, gateway, chain, payment, or business-domain names.

mod heartbeat_agent_dispatch;
mod heartbeat_lane;
mod scheduler_lane;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use chrono::Utc;
use heartbeat_lane::HeartbeatLane;
use macaca_app::AppManifest;
use macaca_heartbeat::LocalHeartbeatProvider;
use macaca_proto::{
    AutonomyScope, HeartbeatCadencePolicy, HeartbeatProfile, HeartbeatProfileId,
    HeartbeatScopeIdentity, MacacaResult, SchedulerRunState, TraceContext,
};
use macaca_scheduler::LocalSchedulerProvider;
use scheduler_lane::SchedulerLane;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, info, warn};

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

    /// Register an application-scoped native Heartbeat profile from manifest policy.
    ///
    /// This is a generic Adapter step between Application-owned declarations
    /// and Heartbeat-owned cadence. The profile only says "this application has
    /// heartbeat declarations that should be evaluated on the native cadence";
    /// it does not copy agent prompts, branch on app names, choose models, or
    /// embed business workflow semantics. Later ticks still query Application
    /// Service for the sanitized declaration view before dispatching Agent
    /// Execution commands, preserving the service boundary and audit chain.
    pub fn register_application_heartbeat_profile(
        &self,
        manifest: &AppManifest,
        trace: TraceContext,
    ) -> MacacaResult<bool> {
        let Some(heartbeat) = manifest
            .autonomy
            .as_ref()
            .and_then(|autonomy| autonomy.heartbeat.as_ref())
        else {
            return Ok(false);
        };
        if !heartbeat.enabled || heartbeat.agents.is_empty() {
            info!(
                trace_id = trace.trace_id.as_str(),
                app_id = %manifest.id,
                enabled = heartbeat.enabled,
                declaration_count = heartbeat.agents.len(),
                "application heartbeat native profile registration skipped by manifest policy"
            );
            return Ok(false);
        }

        let profile_id =
            HeartbeatProfileId::new(format!("profile.application.{}.heartbeat", manifest.id))?;
        let scope_key = format!("application:{}.heartbeat", manifest.id);
        let mut profile = HeartbeatProfile::new(
            profile_id.clone(),
            HeartbeatScopeIdentity::new(AutonomyScope::application(manifest.id), scope_key)?,
            HeartbeatCadencePolicy::FixedInterval {
                interval_ms: self.config.heartbeat_tick_interval_ms,
                anchor: Some(
                    Utc::now()
                        - chrono::Duration::milliseconds(
                            self.config.heartbeat_tick_interval_ms.min(i64::MAX as u64) as i64,
                        ),
                ),
            },
        )?;
        profile.metadata.insert(
            "declaration_count".into(),
            heartbeat.agents.len().to_string(),
        );
        self.heartbeat.register_native_profile(profile)?;
        info!(
            trace_id = trace.trace_id.as_str(),
            app_id = %manifest.id,
            profile_id = profile_id.as_str(),
            declaration_count = heartbeat.agents.len(),
            "application heartbeat native profile registered from manifest"
        );
        Ok(true)
    }

    /// Execute one bounded Scheduler tick for deterministic tests and manual wakeups.
    pub async fn run_scheduler_tick_once(&self, trace: TraceContext) -> MacacaResult<usize> {
        SchedulerLane::new(
            Arc::clone(&self.runtime),
            Arc::clone(&self.scheduler),
            self.config.clone(),
        )
        .tick_once(trace)
        .await
    }

    /// Execute one native Heartbeat cadence tick.
    pub async fn run_heartbeat_tick_once(&self, trace: TraceContext) -> MacacaResult<bool> {
        HeartbeatLane::new(
            Arc::clone(&self.runtime),
            Arc::clone(&self.heartbeat),
            self.config.recovery_wake_enabled,
            self.config.dispatch_timeout_ms,
        )
        .tick_once(trace)
        .await
    }

    /// Emit one provider-neutral recovery wake when enabled.
    pub async fn run_recovery_wake_once(&self, trace: TraceContext) -> MacacaResult<bool> {
        HeartbeatLane::new(
            Arc::clone(&self.runtime),
            Arc::clone(&self.heartbeat),
            self.config.recovery_wake_enabled,
            self.config.dispatch_timeout_ms,
        )
        .recovery_wake_once(trace)
        .await
    }

    async fn run_loop(self, trace: TraceContext) {
        while self.running.load(Ordering::SeqCst) {
            sleep(Duration::from_millis(
                self.config.scheduler_tick_interval_ms,
            ))
            .await;
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
