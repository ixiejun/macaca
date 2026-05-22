//! Heartbeat lane for the local autonomy supervisor.
//!
//! HeartbeatLane owns native heartbeat cadence coordination. It calls the
//! Heartbeat service's profile tick API directly through the injected local
//! provider and does not require Scheduler jobs, due-run materialization, or
//! Scheduler leases. This keeps agent/system heartbeat liveness independent
//! from application schedule management.

use std::sync::Arc;

use macaca_heartbeat::{HeartbeatService, LocalHeartbeatProvider};
use macaca_proto::{
    AutonomyScope, HeartbeatWakeCommand, HeartbeatWakeIntent, MacacaResult, TraceContext,
};
use tracing::{info, warn};

use super::heartbeat_agent_dispatch::{
    HeartbeatAgentDispatchStrategy, HeartbeatAgentDispatchSummary,
};
use crate::ServiceRuntime;

/// Runtime-host Strategy object for one bounded Heartbeat supervisor tick.
pub(crate) struct HeartbeatLane {
    runtime: Arc<ServiceRuntime>,
    heartbeat: Arc<LocalHeartbeatProvider>,
    recovery_wake_enabled: bool,
    dispatch_timeout_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use chrono::{Duration as ChronoDuration, Utc};
    use macaca_app::application_service_descriptor;
    use macaca_kernel::SystemService;
    use macaca_proto::{
        AgentExecutionCommand, AgentExecutionResult, ApplicationHeartbeatAgentView,
        ApplicationHeartbeatAgentsResult, ApplicationId, CleanupPolicy, HeartbeatCadencePolicy,
        HeartbeatProfile, HeartbeatProfileId, HeartbeatScopeIdentity, ServiceCallResult,
        ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
        APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND,
    };
    use tokio::time::timeout;

    use crate::{
        agent_execution_service_descriptor, AgentExecutionBackend,
        AgentExecutionSystemServiceProvider, ServiceProviderFactoryContext,
        ServiceProviderInstance, ServiceRuntimeConfig, StaticServiceProviderFactory,
    };

    /// Minimal Application Service test double for heartbeat declaration lookup.
    ///
    /// The fake intentionally speaks only the generic `SystemService` contract
    /// so the test exercises the same service-runtime boundary that production
    /// Runtime Host uses.  It does not embed application names, workflow
    /// behavior, or filesystem proof logic.
    struct FakeApplicationHeartbeatService {
        descriptor: ServiceDescriptor,
        declarations: ApplicationHeartbeatAgentsResult,
    }

    impl FakeApplicationHeartbeatService {
        fn new(declarations: ApplicationHeartbeatAgentsResult) -> Self {
            Self {
                descriptor: application_service_descriptor(),
                declarations,
            }
        }
    }

    #[async_trait]
    impl SystemService for FakeApplicationHeartbeatService {
        fn descriptor(&self) -> ServiceDescriptor {
            self.descriptor.clone()
        }

        async fn start(&self) -> ServiceResult<()> {
            Ok(())
        }

        async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
            let trace = command
                .trace
                .clone()
                .ok_or(ServiceError::MissingTraceContext)?;
            if command.name.as_str() != APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND {
                return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
            }
            Ok(ServiceCallResult {
                status: "ok".into(),
                output: serde_json::to_value(&self.declarations)
                    .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
                trace,
                metadata: BTreeMap::new(),
                cleanup_hint: Some(CleanupPolicy::None),
            })
        }

        async fn stop(&self) -> ServiceResult<()> {
            Ok(())
        }

        async fn cleanup(&self) -> ServiceResult<()> {
            Ok(())
        }

        async fn health(&self) -> ServiceResult<ServiceHealth> {
            Ok(ServiceHealth::Healthy)
        }
    }

    /// Slow Agent Execution backend used to prove HeartbeatLane does not await
    /// long model/tool work inside the supervisor tick path.
    #[derive(Default)]
    struct SlowExecutionBackend {
        started: AtomicUsize,
    }

    #[async_trait]
    impl AgentExecutionBackend for SlowExecutionBackend {
        async fn execute(
            &self,
            command: AgentExecutionCommand,
        ) -> ServiceResult<AgentExecutionResult> {
            self.started.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_secs(5)).await;
            let mut result =
                AgentExecutionResult::completed(&command, serde_json::json!({"accepted": true}));
            result.metadata.insert(
                "result_evidence_ref".into(),
                "event/heartbeat-result/1".into(),
            );
            Ok(result)
        }
    }

    async fn register_static_service(
        runtime: &ServiceRuntime,
        descriptor: ServiceDescriptor,
        service: Arc<dyn SystemService>,
    ) {
        runtime
            .register_provider(
                &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                    descriptor, service,
                )),
                ServiceProviderFactoryContext::new(),
            )
            .await
            .unwrap();
    }

    fn due_application_profile(application_id: ApplicationId) -> HeartbeatProfile {
        HeartbeatProfile::new(
            HeartbeatProfileId::new("profile.application.test.heartbeat").unwrap(),
            HeartbeatScopeIdentity::new(
                AutonomyScope::application(application_id),
                "application.test.heartbeat",
            )
            .unwrap(),
            HeartbeatCadencePolicy::FixedInterval {
                interval_ms: 1,
                anchor: Some(Utc::now() - ChronoDuration::milliseconds(5)),
            },
        )
        .unwrap()
    }

    #[tokio::test]
    async fn heartbeat_tick_hands_off_agent_dispatch_without_blocking_scheduler_lane() {
        let application_id = ApplicationId::from_name("generic-heartbeat-app");
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let heartbeat = Arc::new(LocalHeartbeatProvider::new());
        let backend = Arc::new(SlowExecutionBackend::default());
        let declaration = ApplicationHeartbeatAgentView {
            application_id,
            agent_name: "operator".into(),
            enabled: true,
            profile_id: "default".into(),
            metadata: BTreeMap::new(),
            diagnostics: Vec::new(),
        };

        heartbeat
            .register_native_profile(due_application_profile(application_id))
            .unwrap();
        register_static_service(
            &runtime,
            application_service_descriptor(),
            Arc::new(FakeApplicationHeartbeatService::new(vec![declaration])),
        )
        .await;
        register_static_service(
            &runtime,
            agent_execution_service_descriptor(),
            Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
        )
        .await;

        let lane = HeartbeatLane::new(runtime, heartbeat, true, 1_000);
        let accepted = timeout(
            Duration::from_millis(250),
            lane.tick_once(TraceContext::new("trace-heartbeat-nonblocking")),
        )
        .await
        .expect("heartbeat tick should not await slow agent execution")
        .unwrap();

        assert!(accepted);
        for _ in 0..20 {
            if backend.started.load(Ordering::SeqCst) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(backend.started.load(Ordering::SeqCst), 1);
    }
}

impl HeartbeatLane {
    /// Build a Heartbeat lane from approved runtime-host composition inputs.
    pub(crate) fn new(
        runtime: Arc<ServiceRuntime>,
        heartbeat: Arc<LocalHeartbeatProvider>,
        recovery_wake_enabled: bool,
        dispatch_timeout_ms: u64,
    ) -> Self {
        Self {
            runtime,
            heartbeat,
            recovery_wake_enabled,
            dispatch_timeout_ms: dispatch_timeout_ms.max(1),
        }
    }

    /// Execute one native Heartbeat cadence tick.
    ///
    /// The tick evaluates Heartbeat-owned profiles and gates. It never creates
    /// Scheduler jobs and never interprets application business payloads.
    pub(crate) async fn tick_once(&self, trace: TraceContext) -> MacacaResult<bool> {
        let results = self
            .heartbeat
            .tick_native_profiles_once(trace.clone())
            .await?;
        let accepted = results.iter().any(|result| result.accepted);
        let mut dispatch_summary = HeartbeatAgentDispatchSummary::default();
        for result in results.iter().filter(|result| result.accepted) {
            let runtime = Arc::clone(&self.runtime);
            let wake = result.clone();
            let timeout_ms = self.dispatch_timeout_ms;
            dispatch_summary.dispatched += 1;
            info!(
                trace_id = trace.trace_id.as_str(),
                heartbeat_run_id = result
                    .run_id
                    .as_ref()
                    .map(|run_id| run_id.as_str())
                    .unwrap_or("none"),
                "autonomy heartbeat lane handing off accepted wake to background agent dispatch"
            );
            tokio::spawn(async move {
                let dispatcher = HeartbeatAgentDispatchStrategy::with_timeout(runtime, timeout_ms);
                match dispatcher.dispatch_after_accepted_wake(&wake).await {
                    Ok(summary) => {
                        info!(
                            trace_id = wake.trace.trace_id.as_str(),
                            queried = summary.queried,
                            enabled = summary.enabled,
                            dispatched = summary.dispatched,
                            skipped = summary.skipped,
                            failed = summary.failed,
                            "heartbeat background agent dispatch completed"
                        );
                    }
                    Err(error) => {
                        warn!(
                            trace_id = wake.trace.trace_id.as_str(),
                            error = %error,
                            "heartbeat background agent dispatch failed"
                        );
                    }
                }
            });
        }
        info!(
            trace_id = trace.trace_id.as_str(),
            accepted,
            processed = results.len(),
            handoff_count = dispatch_summary.dispatched,
            "autonomy heartbeat lane native cadence tick completed"
        );
        Ok(accepted)
    }

    /// Emit one provider-neutral recovery wake when enabled.
    pub(crate) async fn recovery_wake_once(&self, trace: TraceContext) -> MacacaResult<bool> {
        if !self.recovery_wake_enabled {
            info!(
                trace_id = trace.trace_id.as_str(),
                "autonomy heartbeat lane recovery wake skipped by config"
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
            "autonomy heartbeat lane recovery wake completed"
        );
        Ok(result.accepted)
    }
}
