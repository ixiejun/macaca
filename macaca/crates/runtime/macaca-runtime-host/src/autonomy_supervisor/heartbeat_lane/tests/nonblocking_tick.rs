//! Contract test: HeartbeatLane tick must not block on slow agent execution.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use macaca_app::application_service_descriptor;
use macaca_proto::{ApplicationHeartbeatAgentView, ApplicationId, TraceContext};
use tokio::time::timeout;

use crate::{
    agent_execution_service_descriptor, AgentExecutionSystemServiceProvider, ServiceRuntime,
    ServiceRuntimeConfig,
};

use super::fixtures::{due_application_profile, register_heartbeat_service, register_static_service};
use super::super::HeartbeatLane;
use super::test_doubles::{FakeApplicationHeartbeatService, SlowExecutionBackend};

#[tokio::test]
async fn heartbeat_tick_hands_off_agent_dispatch_without_blocking_scheduler_lane() {
    let application_id = ApplicationId::from_name("generic-heartbeat-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let heartbeat = Arc::new(macaca_heartbeat::InProcessHeartbeatProvider::new());
    let backend = Arc::new(SlowExecutionBackend::default());
    let declaration = ApplicationHeartbeatAgentView {
        application_id,
        agent_name: "operator".into(),
        enabled: true,
        profile_id: "default".into(),
        native_profile_id: "profile.application.test.agent.operator.heartbeat".into(),
        wake_scope_key: "application.test.agent:operator.heartbeat".into(),
        fixed_interval_secs: Some(1),
        cooldown_secs: None,
        metadata: Default::default(),
        diagnostics: Vec::new(),
    };

    heartbeat
        .register_native_profile(due_application_profile(application_id))
        .unwrap();
    register_heartbeat_service(&runtime, Arc::clone(&heartbeat)).await;
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
