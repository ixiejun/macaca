//! Contract test: per-agent heartbeat cadences dispatch on separate ticks.

use std::sync::Arc;

use chrono::{Duration as ChronoDuration, Utc};
use macaca_proto::{ApplicationId, TraceContext};

use crate::{ServiceRuntime, ServiceRuntimeConfig};

use super::fixtures::{
    agent_application_profile, heartbeat_declaration, register_lane_services,
    wait_for_recorded_commands,
};
use super::test_doubles::RecordingExecutionBackend;
use super::super::HeartbeatLane;

#[tokio::test]
async fn per_agent_profiles_with_distinct_cadences_dispatch_at_separate_ticks() {
    let application_id = ApplicationId::from_name("generic-heartbeat-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let heartbeat = Arc::new(macaca_heartbeat::InProcessHeartbeatProvider::new());
    let backend = Arc::new(RecordingExecutionBackend::default());
    let operator_profile_id = "profile.application.test.agent.operator.heartbeat";
    let reviewer_profile_id = "profile.application.test.agent.reviewer.heartbeat";
    let operator_scope = "application.test.agent:operator.heartbeat";
    let reviewer_scope = "application.test.agent:reviewer.heartbeat";
    let now = Utc::now();

    heartbeat
        .register_native_profile(agent_application_profile(
            application_id,
            "operator",
            operator_profile_id,
            operator_scope,
            60_000,
            now - ChronoDuration::milliseconds(60_001),
        ))
        .unwrap();
    heartbeat
        .register_native_profile(agent_application_profile(
            application_id,
            "reviewer",
            reviewer_profile_id,
            reviewer_scope,
            120_000,
            now,
        ))
        .unwrap();
    register_lane_services(
        &runtime,
        Arc::clone(&heartbeat),
        vec![
            heartbeat_declaration(
                application_id,
                "operator",
                operator_profile_id,
                operator_scope,
                60,
            ),
            heartbeat_declaration(
                application_id,
                "reviewer",
                reviewer_profile_id,
                reviewer_scope,
                120,
            ),
        ],
        backend.clone(),
    )
    .await;

    let lane = HeartbeatLane::new(Arc::clone(&runtime), Arc::clone(&heartbeat), true, 1_000);
    let first_tick = lane
        .tick_once(TraceContext::new("trace-heartbeat-operator-due"))
        .await
        .unwrap();
    let commands = wait_for_recorded_commands(&backend, 1).await;

    assert!(first_tick);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].target_agent, "operator");
    assert_eq!(
        commands[0].metadata["native_profile_id"],
        operator_profile_id
    );
    assert_eq!(commands[0].metadata["wake_scope_key"], operator_scope);

    heartbeat
        .register_native_profile(agent_application_profile(
            application_id,
            "reviewer",
            reviewer_profile_id,
            reviewer_scope,
            120_000,
            Utc::now() - ChronoDuration::milliseconds(120_001),
        ))
        .unwrap();
    let second_tick = lane
        .tick_once(TraceContext::new("trace-heartbeat-reviewer-due"))
        .await
        .unwrap();
    let commands = wait_for_recorded_commands(&backend, 2).await;

    assert!(second_tick);
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[1].target_agent, "reviewer");
    assert_eq!(
        commands[1].metadata["native_profile_id"],
        reviewer_profile_id
    );
    assert_eq!(commands[1].metadata["wake_scope_key"], reviewer_scope);
    assert_eq!(
        commands
            .iter()
            .map(|command| command.target_agent.as_str())
            .collect::<Vec<_>>(),
        vec!["operator", "reviewer"]
    );
}
