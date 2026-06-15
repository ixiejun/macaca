//! Contract scenarios for heartbeat agent dispatch strategy.
//!
//! Each test validates one boundary outcome: successful dispatch, skill alias
//! resolution, empty manifests, per-agent wake scoping, and service failures.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_app::application_service_descriptor;
use macaca_proto::{
    AgentExecutionIntent, ApplicationHeartbeatAgentView, ApplicationId, HeartbeatRunState,
    HEARTBEAT_SERVICE_ID,
};

use crate::{
    agent_execution_service_descriptor, AgentExecutionSystemServiceProvider, ServiceRuntime,
    ServiceRuntimeConfig,
};

use super::super::HeartbeatAgentDispatchStrategy;
use super::fixtures::{
    accepted_app_wake, accepted_app_wake_with_metadata, register_skill_alias,
    register_static_service, FakeApplicationHeartbeatService, RecordingExecutionBackend,
};

#[tokio::test]
async fn declaration_driven_dispatch_calls_agent_execution() {
    let application_id = ApplicationId::from_name("generic-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let backend = Arc::new(RecordingExecutionBackend::default());
    let declaration = ApplicationHeartbeatAgentView {
        application_id,
        agent_name: "operator".into(),
        enabled: true,
        profile_id: "default".into(),
        native_profile_id: "profile.application.test.agent.operator.heartbeat".into(),
        wake_scope_key: "application.test.agent:operator.heartbeat".into(),
        fixed_interval_secs: Some(30),
        cooldown_secs: None,
        metadata: BTreeMap::from([(
            "evidence.expected_artifact_path".into(),
            "/workspace/agents/operator/heartbeat.md".into(),
        )]),
        diagnostics: Vec::new(),
    };

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

    let summary = HeartbeatAgentDispatchStrategy::with_timeout(runtime, 30_000)
        .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
        .await
        .unwrap();

    assert_eq!(summary.queried, 1);
    assert_eq!(summary.enabled, 1);
    assert_eq!(summary.dispatched, 1);
    assert_eq!(summary.skipped, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.completion_state, Some(HeartbeatRunState::Succeeded));
    assert_eq!(
        summary.reason_code.as_deref(),
        Some("agent_execution_completed")
    );
    assert_eq!(
        summary
            .metadata
            .get("agent_execution.status")
            .map(String::as_str),
        Some("completed")
    );
    assert_eq!(
        summary.metadata.get("dispatch.failed").map(String::as_str),
        Some("0")
    );
    let commands = backend.commands.lock().unwrap();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].execution_intent,
        AgentExecutionIntent::Heartbeat
    );
    assert_eq!(commands[0].target_agent, "operator");
    assert_eq!(commands[0].metadata["source"], HEARTBEAT_SERVICE_ID);
    assert_eq!(
        commands[0].metadata["heartbeat_audit_id"],
        "audit-heartbeat"
    );
    assert_eq!(
        commands[0].metadata["evidence.expected_artifact_path"],
        "/workspace/agents/operator/heartbeat.md"
    );
    let envelope = commands[0]
        .execution_envelope
        .as_ref()
        .expect("heartbeat dispatch must attach an execution envelope");
    assert_eq!(
        envelope.source_kind,
        macaca_proto::AutonomousExecutionSourceKind::HeartbeatProfile
    );
    assert_eq!(
        envelope.completion_policy.kind,
        macaca_proto::AutonomousCompletionPolicyKind::RequireArtifact
    );
}

#[tokio::test]
async fn declaration_driven_dispatch_resolves_skill_alias_before_execution() {
    let application_id = ApplicationId::from_name("generic-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let backend = Arc::new(RecordingExecutionBackend::default());
    let declaration = ApplicationHeartbeatAgentView {
        application_id,
        agent_name: "operator".into(),
        enabled: true,
        profile_id: "default".into(),
        native_profile_id: "profile.application.test.agent.operator.heartbeat".into(),
        wake_scope_key: "application.test.agent:operator.heartbeat".into(),
        fixed_interval_secs: Some(30),
        cooldown_secs: None,
        metadata: BTreeMap::from([(
            "skill.alias.requested_id".into(),
            "skill://agent/superseded-heartbeat".into(),
        )]),
        diagnostics: Vec::new(),
    };

    register_static_service(
        &runtime,
        application_service_descriptor(),
        Arc::new(FakeApplicationHeartbeatService::new(vec![declaration])),
    )
    .await;
    register_skill_alias(
        &runtime,
        "skill://agent/superseded-heartbeat",
        "skill://agent/current-heartbeat",
    )
    .await;
    register_static_service(
        &runtime,
        agent_execution_service_descriptor(),
        Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
    )
    .await;

    let summary = HeartbeatAgentDispatchStrategy::with_timeout(runtime, 30_000)
        .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
        .await
        .unwrap();

    assert_eq!(summary.dispatched, 1);
    let commands = backend.commands.lock().unwrap();
    let metadata = &commands[0].metadata;
    assert_eq!(
        metadata["skill.alias.requested_id"],
        "skill://agent/superseded-heartbeat"
    );
    assert_eq!(metadata["skill.alias.resolved"], "true");
    assert_eq!(metadata["skill.alias.status"], "redirected");
    assert_eq!(
        metadata["skill.alias.effective_id"],
        "skill://agent/current-heartbeat"
    );
    assert_eq!(metadata["skill.alias.kind"], "absorbed_into");
    assert_eq!(metadata["skill.alias.policy"], "redirect");
}

#[tokio::test]
async fn absent_declarations_return_empty_structured_summary() {
    let application_id = ApplicationId::from_name("generic-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    register_static_service(
        &runtime,
        application_service_descriptor(),
        Arc::new(FakeApplicationHeartbeatService::new(Vec::new())),
    )
    .await;

    let summary = HeartbeatAgentDispatchStrategy::with_timeout(runtime, 30_000)
        .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
        .await
        .unwrap();

    assert_eq!(summary.queried, 0);
    assert_eq!(summary.enabled, 0);
    assert_eq!(summary.dispatched, 0);
    assert_eq!(summary.skipped, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.completion_state, Some(HeartbeatRunState::Skipped));
    assert_eq!(
        summary.reason_code.as_deref(),
        Some("no_eligible_heartbeat_declaration")
    );
    assert_eq!(
        summary.metadata.get("dispatch.queried").map(String::as_str),
        Some("0")
    );
}

#[tokio::test]
async fn per_agent_wake_dispatches_only_matching_declaration() {
    let application_id = ApplicationId::from_name("generic-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let backend = Arc::new(RecordingExecutionBackend::default());
    let operator = ApplicationHeartbeatAgentView {
        application_id,
        agent_name: "operator".into(),
        enabled: true,
        profile_id: "default".into(),
        native_profile_id: "profile.application.test.agent.operator.heartbeat".into(),
        wake_scope_key: "application:test.agent:operator.heartbeat".into(),
        fixed_interval_secs: Some(30),
        cooldown_secs: Some(15),
        metadata: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    let reviewer = ApplicationHeartbeatAgentView {
        application_id,
        agent_name: "reviewer".into(),
        enabled: true,
        profile_id: "default".into(),
        native_profile_id: "profile.application.test.agent.reviewer.heartbeat".into(),
        wake_scope_key: "application:test.agent:reviewer.heartbeat".into(),
        fixed_interval_secs: Some(60),
        cooldown_secs: Some(30),
        metadata: BTreeMap::new(),
        diagnostics: Vec::new(),
    };

    register_static_service(
        &runtime,
        application_service_descriptor(),
        Arc::new(FakeApplicationHeartbeatService::new(vec![
            operator, reviewer,
        ])),
    )
    .await;
    register_static_service(
        &runtime,
        agent_execution_service_descriptor(),
        Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
    )
    .await;
    let wake = accepted_app_wake_with_metadata(
        application_id,
        BTreeMap::from([
            (
                "native_profile_id".into(),
                "profile.application.test.agent.reviewer.heartbeat".into(),
            ),
            (
                "scope_key".into(),
                "application:test.agent:reviewer.heartbeat".into(),
            ),
        ]),
    );

    let summary = HeartbeatAgentDispatchStrategy::with_timeout(runtime, 30_000)
        .dispatch_after_accepted_wake(&wake)
        .await
        .unwrap();

    assert_eq!(summary.queried, 2);
    assert_eq!(summary.enabled, 1);
    assert_eq!(summary.dispatched, 1);
    assert_eq!(summary.skipped, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.completion_state, Some(HeartbeatRunState::Succeeded));
    assert_eq!(
        summary.reason_code.as_deref(),
        Some("agent_execution_completed")
    );
    let commands = backend.commands.lock().unwrap();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].target_agent, "reviewer");
    assert_eq!(
        commands[0].metadata["native_profile_id"],
        "profile.application.test.agent.reviewer.heartbeat"
    );
    assert_eq!(
        commands[0].metadata["wake_scope_key"],
        "application:test.agent:reviewer.heartbeat"
    );
}

#[tokio::test]
async fn unavailable_application_service_returns_failure_evidence() {
    let application_id = ApplicationId::from_name("generic-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));

    let summary = HeartbeatAgentDispatchStrategy::with_timeout(runtime, 30_000)
        .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
        .await
        .unwrap();

    assert_eq!(summary.failed, 1);
    assert_eq!(summary.completion_state, Some(HeartbeatRunState::Failed));
    assert_eq!(
        summary.reason_code.as_deref(),
        Some("declaration_query_failed")
    );
}

#[tokio::test]
async fn unavailable_agent_execution_service_returns_failure_evidence() {
    let application_id = ApplicationId::from_name("generic-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let declaration = ApplicationHeartbeatAgentView {
        application_id,
        agent_name: "operator".into(),
        enabled: true,
        profile_id: "default".into(),
        native_profile_id: "profile.application.test.agent.operator.heartbeat".into(),
        wake_scope_key: "application.test.agent:operator.heartbeat".into(),
        fixed_interval_secs: Some(30),
        cooldown_secs: None,
        metadata: BTreeMap::new(),
        diagnostics: Vec::new(),
    };

    register_static_service(
        &runtime,
        application_service_descriptor(),
        Arc::new(FakeApplicationHeartbeatService::new(vec![declaration])),
    )
    .await;

    let summary = HeartbeatAgentDispatchStrategy::with_timeout(runtime, 30_000)
        .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
        .await
        .unwrap();

    assert_eq!(summary.queried, 1);
    assert_eq!(summary.enabled, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.completion_state, Some(HeartbeatRunState::Failed));
    assert_eq!(
        summary.reason_code.as_deref(),
        Some("agent_execution_failed")
    );
    assert_eq!(
        summary.metadata.get("dispatch.failed").map(String::as_str),
        Some("1")
    );
}

#[tokio::test]
async fn invalid_declarations_are_skipped_without_dispatch() {
    let application_id = ApplicationId::from_name("generic-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let backend = Arc::new(RecordingExecutionBackend::default());
    let declaration = ApplicationHeartbeatAgentView {
        application_id,
        agent_name: "operator".into(),
        enabled: true,
        profile_id: "default".into(),
        native_profile_id: "profile.application.test.agent.operator.heartbeat".into(),
        wake_scope_key: "application.test.agent:operator.heartbeat".into(),
        fixed_interval_secs: Some(30),
        cooldown_secs: None,
        metadata: BTreeMap::new(),
        diagnostics: vec!["heartbeat_agent_unknown".into()],
    };

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

    let summary = HeartbeatAgentDispatchStrategy::with_timeout(runtime, 30_000)
        .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
        .await
        .unwrap();

    assert_eq!(summary.queried, 1);
    assert_eq!(summary.enabled, 0);
    assert_eq!(summary.dispatched, 0);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.completion_state, Some(HeartbeatRunState::Skipped));
    assert_eq!(
        summary.reason_code.as_deref(),
        Some("no_eligible_heartbeat_declaration")
    );
    assert!(backend.commands.lock().unwrap().is_empty());
}
