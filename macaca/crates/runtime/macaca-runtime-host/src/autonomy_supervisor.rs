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
use macaca_app::{app_manifest_to_heartbeat_agent_views, AppManifest};
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

    /// Register per-agent native Heartbeat profiles from manifest policy.
    ///
    /// This is a generic Adapter step between Application-owned declarations
    /// and Heartbeat-owned cadence. Each profile represents one manifest agent
    /// declaration; it does not copy agent prompts, branch on app names, choose
    /// models, or embed business workflow semantics. Later ticks still query
    /// Application Service for the sanitized declaration view before dispatching
    /// Agent Execution commands, preserving the service boundary and audit
    /// chain.
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

        let declarations = app_manifest_to_heartbeat_agent_views(manifest);
        let mut registered = 0usize;
        for declaration in declarations
            .iter()
            .filter(|declaration| declaration.enabled && declaration.diagnostics.is_empty())
        {
            let interval_ms = declaration
                .fixed_interval_secs
                .map(seconds_to_millis)
                .unwrap_or(self.config.heartbeat_tick_interval_ms);
            let profile_id = HeartbeatProfileId::new(declaration.native_profile_id.clone())?;
            let mut profile = HeartbeatProfile::new(
                profile_id.clone(),
                HeartbeatScopeIdentity::new(
                    AutonomyScope::application(manifest.id),
                    declaration.wake_scope_key.clone(),
                )?,
                HeartbeatCadencePolicy::FixedInterval {
                    interval_ms,
                    anchor: Some(
                        Utc::now()
                            - chrono::Duration::milliseconds(
                                interval_ms.min(i64::MAX as u64) as i64
                            ),
                    ),
                },
            )?;
            profile.cooldown_ms = declaration.cooldown_secs.map(seconds_to_millis);
            profile
                .metadata
                .insert("application_id".into(), manifest.id.to_string());
            profile
                .metadata
                .insert("agent_name".into(), declaration.agent_name.clone());
            profile
                .metadata
                .insert("manifest_profile_id".into(), declaration.profile_id.clone());
            profile.metadata.insert(
                "native_profile_id".into(),
                declaration.native_profile_id.clone(),
            );
            profile
                .metadata
                .insert("wake_scope_key".into(), declaration.wake_scope_key.clone());
            self.heartbeat.register_native_profile(profile)?;
            registered += 1;
            info!(
                trace_id = trace.trace_id.as_str(),
                app_id = %manifest.id,
                agent_name = %declaration.agent_name,
                profile_id = profile_id.as_str(),
                interval_ms,
                cooldown_ms = declaration.cooldown_secs.map(|seconds| seconds.max(1) * 1_000),
                "application heartbeat agent native profile registered from manifest"
            );
        }
        info!(
            trace_id = trace.trace_id.as_str(),
            app_id = %manifest.id,
            declaration_count = heartbeat.agents.len(),
            registered,
            "application heartbeat native profiles registered from manifest"
        );
        Ok(registered > 0)
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

/// Convert manifest seconds into provider milliseconds without panicking.
///
/// Manifest policy is application-owned input, so runtime-host treats `0` as
/// the minimum supported one-second interval and saturates extreme values at
/// `u64::MAX`. The Heartbeat service remains responsible for accepting or
/// rejecting profile policy during native profile registration.
fn seconds_to_millis(seconds: u64) -> u64 {
    seconds.max(1).saturating_mul(1_000)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use macaca_app::model::{
        AgentSource, AppAutonomyConfig, AppHeartbeatAgentConfig, AppHeartbeatCadenceConfig,
        AppHeartbeatConfig, AppHeartbeatGateConfig, InlineAgentConfig,
    };
    use macaca_app::{AppLayer, AppManifest};
    use macaca_heartbeat::HeartbeatService;
    use macaca_proto::{ApplicationId, TraceContext};
    use macaca_scheduler::LocalSchedulerProvider;

    use super::*;
    use crate::{ServiceRuntime, ServiceRuntimeConfig};

    #[test]
    fn manifest_seconds_to_millis_is_minimum_bounded_and_saturating() {
        assert_eq!(seconds_to_millis(0), 1_000);
        assert_eq!(seconds_to_millis(7), 7_000);
        assert_eq!(seconds_to_millis(u64::MAX), u64::MAX);
    }

    #[tokio::test]
    async fn register_application_heartbeat_profile_creates_one_native_profile_per_agent() {
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let scheduler = Arc::new(LocalSchedulerProvider::new());
        let heartbeat = Arc::new(LocalHeartbeatProvider::new());
        let mut config = AutonomyRuntimeConfig::default();
        config.heartbeat_tick_interval_ms = 60_000;
        let supervisor =
            AutonomySupervisor::new(runtime, scheduler, Arc::clone(&heartbeat), config);
        let app_id = ApplicationId::from_name("generic-heartbeat-app");
        let manifest = AppManifest {
            id: app_id,
            name: "Generic Heartbeat App".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L3Declarative,
            ui_type: None,
            agents: vec![
                inline_agent("operator"),
                inline_agent("reviewer"),
                inline_agent("disabled"),
            ],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
            service_contract: None,
            execution_control: None,
            execution_profile: None,
            workbench: None,
            autonomy: Some(AppAutonomyConfig {
                heartbeat: Some(AppHeartbeatConfig {
                    enabled: true,
                    agents: vec![
                        heartbeat_agent("operator", true, Some(11), Some(5)),
                        heartbeat_agent("reviewer", true, Some(17), Some(9)),
                        heartbeat_agent("disabled", false, Some(3), Some(2)),
                    ],
                }),
            }),
            ui: None,
        };

        let registered = supervisor
            .register_application_heartbeat_profile(
                &manifest,
                TraceContext::new("test-register-per-agent-heartbeat-profiles"),
            )
            .unwrap();
        let snapshot = heartbeat
            .snapshot(macaca_proto::HeartbeatQueryCommand {
                trace: TraceContext::new("test-register-per-agent-heartbeat-snapshot"),
                scope: macaca_proto::AutonomyScope::application(app_id),
                run_id: None,
                wake_scope_key: None,
                limit: Some(10),
            })
            .await
            .unwrap();
        let agent_profiles = snapshot
            .native_profiles
            .iter()
            .filter(|profile| profile.scope_key.contains(".agent:"))
            .collect::<Vec<_>>();

        assert!(registered);
        assert_eq!(agent_profiles.len(), 2);
        assert!(agent_profiles.iter().any(|profile| {
            profile.scope_key.ends_with(".agent:operator.heartbeat")
                && profile.fixed_interval_ms == 11_000
                && profile.cooldown_ms == Some(5_000)
        }));
        assert!(agent_profiles.iter().any(|profile| {
            profile.scope_key.ends_with(".agent:reviewer.heartbeat")
                && profile.fixed_interval_ms == 17_000
                && profile.cooldown_ms == Some(9_000)
        }));
    }

    fn inline_agent(name: &str) -> AgentSource {
        AgentSource::Inline(InlineAgentConfig {
            name: name.into(),
            capabilities: Vec::new(),
            prompt_template: String::new(),
            model: "generic-model".into(),
            permission_level: "user".into(),
            allowed_tools: Vec::new(),
            max_tokens: None,
            temperature: None,
            skills: None,
            context_engine: None,
        })
    }

    fn heartbeat_agent(
        name: &str,
        enabled: bool,
        interval_secs: Option<u64>,
        cooldown_secs: Option<u64>,
    ) -> AppHeartbeatAgentConfig {
        AppHeartbeatAgentConfig {
            name: name.into(),
            enabled,
            profile_id: "default".into(),
            cadence: Some(AppHeartbeatCadenceConfig {
                fixed_interval_secs: interval_secs,
            }),
            gates: Some(AppHeartbeatGateConfig { cooldown_secs }),
            metadata: Default::default(),
        }
    }
}
