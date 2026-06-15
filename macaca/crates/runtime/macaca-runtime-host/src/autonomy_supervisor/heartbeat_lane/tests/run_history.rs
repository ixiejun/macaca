//! Contract tests: Heartbeat run memento records dispatch completion outcomes.

use std::sync::Arc;

use chrono::{Duration as ChronoDuration, Utc};
use macaca_proto::{ApplicationId, HeartbeatRunState, TraceContext};

use crate::{ServiceRuntime, ServiceRuntimeConfig};

use super::super::HeartbeatLane;
use super::fixtures::{
    agent_application_profile, heartbeat_declaration, register_lane_services, wait_for_run_state,
};
use super::test_doubles::{NoEvidenceExecutionBackend, RecordingExecutionBackend};

#[tokio::test]
async fn heartbeat_run_history_records_successful_agent_dispatch_completion() {
    let application_id = ApplicationId::from_name("generic-heartbeat-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let heartbeat = Arc::new(macaca_heartbeat::InProcessHeartbeatProvider::new());
    let backend = Arc::new(RecordingExecutionBackend::default());
    let profile_id = "profile.application.test.agent.operator.success";
    let scope_key = "application.test.agent:operator.success";

    heartbeat
        .register_native_profile(agent_application_profile(
            application_id,
            "operator",
            profile_id,
            scope_key,
            1,
            Utc::now() - ChronoDuration::milliseconds(5),
        ))
        .unwrap();
    register_lane_services(
        &runtime,
        Arc::clone(&heartbeat),
        vec![heartbeat_declaration(
            application_id,
            "operator",
            profile_id,
            scope_key,
            1,
        )],
        backend,
    )
    .await;

    let lane = HeartbeatLane::new(Arc::clone(&runtime), Arc::clone(&heartbeat), true, 1_000);
    lane.tick_once(TraceContext::new("trace-heartbeat-success-memento"))
        .await
        .unwrap();
    let run = wait_for_run_state(&heartbeat, scope_key, HeartbeatRunState::Succeeded)
        .await
        .expect("dispatch completion should mark heartbeat run succeeded");

    assert_eq!(
        run.metadata.get("dispatch.reason_code").map(String::as_str),
        Some("agent_execution_completed")
    );
    assert_eq!(
        run.metadata
            .get("agent_execution.status")
            .map(String::as_str),
        Some("completed")
    );
    assert_eq!(
        run.metadata.get("dispatch.failed").map(String::as_str),
        Some("0")
    );
}

#[tokio::test]
async fn heartbeat_run_history_records_failed_agent_dispatch_completion() {
    let application_id = ApplicationId::from_name("generic-heartbeat-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let heartbeat = Arc::new(macaca_heartbeat::InProcessHeartbeatProvider::new());
    let profile_id = "profile.application.test.agent.operator.failure";
    let scope_key = "application.test.agent:operator.failure";

    heartbeat
        .register_native_profile(agent_application_profile(
            application_id,
            "operator",
            profile_id,
            scope_key,
            1,
            Utc::now() - ChronoDuration::milliseconds(5),
        ))
        .unwrap();
    register_lane_services(
        &runtime,
        Arc::clone(&heartbeat),
        vec![heartbeat_declaration(
            application_id,
            "operator",
            profile_id,
            scope_key,
            1,
        )],
        Arc::new(NoEvidenceExecutionBackend),
    )
    .await;

    let lane = HeartbeatLane::new(Arc::clone(&runtime), Arc::clone(&heartbeat), true, 1_000);
    lane.tick_once(TraceContext::new("trace-heartbeat-failure-memento"))
        .await
        .unwrap();
    let run = wait_for_run_state(&heartbeat, scope_key, HeartbeatRunState::Failed)
        .await
        .expect("dispatch completion should mark heartbeat run failed");

    assert_eq!(
        run.metadata.get("dispatch.reason_code").map(String::as_str),
        Some("agent_execution_missing_evidence")
    );
    assert_eq!(
        run.metadata.get("dispatch.failed").map(String::as_str),
        Some("1")
    );
}
