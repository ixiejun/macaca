//! Object Mother fixtures and polling helpers for HeartbeatLane contract tests.
//!
//! Builders in this module construct provider-neutral heartbeat profiles,
//! application declarations, and service-runtime wiring so individual test
//! cases stay focused on one behavioral assertion each.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use macaca_app::application_service_descriptor;
use macaca_heartbeat::HeartbeatService;
use macaca_kernel::SystemService;
use macaca_proto::{
    ApplicationHeartbeatAgentView, ApplicationId, AutonomyScope, HeartbeatCadencePolicy,
    HeartbeatProfile, HeartbeatProfileId, HeartbeatQueryCommand, HeartbeatRunState,
    HeartbeatScopeIdentity, TraceContext,
};

use crate::{
    agent_execution_service_descriptor, AgentExecutionBackend, AgentExecutionSystemServiceProvider,
    HostHeartbeatServiceAdapter, ServiceProviderFactoryContext, ServiceProviderInstance,
    ServiceRuntime, StaticServiceProviderFactory,
};

use super::test_doubles::FakeApplicationHeartbeatService;

/// Register one static SystemService into the in-memory ServiceRuntime under test.
pub(super) async fn register_static_service(
    runtime: &ServiceRuntime,
    descriptor: macaca_proto::ServiceDescriptor,
    service: Arc<dyn SystemService>,
) {
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .unwrap();
}

/// Register the local in-process Heartbeat provider through the host adapter.
pub(super) async fn register_heartbeat_service(
    runtime: &ServiceRuntime,
    heartbeat: Arc<macaca_heartbeat::InProcessHeartbeatProvider>,
) {
    register_static_service(
        runtime,
        heartbeat.descriptor(),
        Arc::new(HostHeartbeatServiceAdapter::new(heartbeat)),
    )
    .await;
}

/// Register Application + Agent Execution services for one lane exercise.
pub(super) async fn register_lane_services(
    runtime: &ServiceRuntime,
    heartbeat: Arc<macaca_heartbeat::InProcessHeartbeatProvider>,
    declarations: Vec<ApplicationHeartbeatAgentView>,
    backend: Arc<dyn AgentExecutionBackend>,
) {
    register_heartbeat_service(runtime, Arc::clone(&heartbeat)).await;
    register_static_service(
        runtime,
        application_service_descriptor(),
        Arc::new(FakeApplicationHeartbeatService::new(declarations)),
    )
    .await;
    register_static_service(
        runtime,
        agent_execution_service_descriptor(),
        Arc::new(AgentExecutionSystemServiceProvider::new(backend)),
    )
    .await;
}

/// Build one due application-level heartbeat profile for cadence tick tests.
pub(super) fn due_application_profile(application_id: ApplicationId) -> HeartbeatProfile {
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

/// Build one per-agent native profile with explicit cadence metadata refs.
pub(super) fn agent_application_profile(
    application_id: ApplicationId,
    agent_name: &str,
    native_profile_id: &str,
    wake_scope_key: &str,
    interval_ms: u64,
    anchor: chrono::DateTime<Utc>,
) -> HeartbeatProfile {
    let mut profile = HeartbeatProfile::new(
        HeartbeatProfileId::new(native_profile_id).unwrap(),
        HeartbeatScopeIdentity::new(AutonomyScope::application(application_id), wake_scope_key)
            .unwrap(),
        HeartbeatCadencePolicy::FixedInterval {
            interval_ms,
            anchor: Some(anchor),
        },
    )
    .unwrap();
    // Keep profile metadata bounded and provider-neutral. The dispatcher uses
    // these identifiers to match the accepted wake to the sanitized Application
    // Service declaration, without branching on app behavior.
    profile
        .metadata
        .insert("application_id".into(), application_id.to_string());
    profile
        .metadata
        .insert("agent_name".into(), agent_name.into());
    profile
        .metadata
        .insert("native_profile_id".into(), native_profile_id.into());
    profile
        .metadata
        .insert("wake_scope_key".into(), wake_scope_key.into());
    profile
}

/// Build one sanitized Application Service heartbeat declaration view.
pub(super) fn heartbeat_declaration(
    application_id: ApplicationId,
    agent_name: &str,
    native_profile_id: &str,
    wake_scope_key: &str,
    fixed_interval_secs: u64,
) -> ApplicationHeartbeatAgentView {
    ApplicationHeartbeatAgentView {
        application_id,
        agent_name: agent_name.into(),
        enabled: true,
        profile_id: "default".into(),
        native_profile_id: native_profile_id.into(),
        wake_scope_key: wake_scope_key.into(),
        fixed_interval_secs: Some(fixed_interval_secs),
        cooldown_secs: None,
        metadata: BTreeMap::new(),
        diagnostics: Vec::new(),
    }
}

use macaca_proto::AgentExecutionCommand;

use super::test_doubles::RecordingExecutionBackend;

/// Poll until the recording backend captures the expected command count.
pub(super) async fn wait_for_recorded_commands(
    backend: &RecordingExecutionBackend,
    expected: usize,
) -> Vec<AgentExecutionCommand> {
    for _ in 0..20 {
        let commands = backend.commands.lock().unwrap().clone();
        if commands.len() >= expected {
            return commands;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    backend.commands.lock().unwrap().clone()
}

/// Poll heartbeat snapshot until one run reaches the expected terminal state.
pub(super) async fn wait_for_run_state(
    heartbeat: &macaca_heartbeat::InProcessHeartbeatProvider,
    scope_key: &str,
    expected: HeartbeatRunState,
) -> Option<macaca_proto::HeartbeatRunSummary> {
    for _ in 0..40 {
        let snapshot = HeartbeatService::snapshot(
            heartbeat,
            HeartbeatQueryCommand {
                trace: TraceContext::new("trace-heartbeat-test-snapshot"),
                scope: AutonomyScope::global(),
                run_id: None,
                wake_scope_key: None,
                limit: Some(25),
            },
        )
        .await
        .expect("heartbeat snapshot should be readable");
        let run = snapshot
            .recent_runs
            .into_iter()
            .find(|run| run.wake_scope_key == scope_key && run.state == expected);
        if run.is_some() {
            return run;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    None
}
