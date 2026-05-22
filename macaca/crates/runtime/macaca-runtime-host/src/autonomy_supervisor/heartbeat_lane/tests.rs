use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use macaca_app::application_service_descriptor;
use macaca_heartbeat::HeartbeatService;
use macaca_kernel::SystemService;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionResult, ApplicationHeartbeatAgentView,
    ApplicationHeartbeatAgentsResult, ApplicationId, AutonomyScope, CleanupPolicy,
    HeartbeatCadencePolicy, HeartbeatProfile, HeartbeatProfileId, HeartbeatQueryCommand,
    HeartbeatRunState, HeartbeatScopeIdentity, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, TraceContext,
    APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND,
};
use tokio::time::timeout;

use super::HeartbeatLane;
use crate::{
    agent_execution_service_descriptor, AgentExecutionBackend, AgentExecutionSystemServiceProvider,
    HeartbeatSystemServiceProvider, ServiceProviderFactoryContext, ServiceProviderInstance,
    ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};

/// Minimal Application Service test double for heartbeat declaration lookup.
///
/// The fake intentionally speaks only the generic `SystemService` contract
/// so the test exercises the same service-runtime boundary that production
/// Runtime Host uses. It does not embed application names, workflow behavior,
/// or filesystem proof logic.
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
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
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

/// Agent Execution backend that captures commands crossing the service boundary.
///
/// The lane under test still calls `service.agent_execution`; this recorder
/// only stores sanitized command envelopes so assertions can prove which
/// manifest agent was dispatched after each native cadence tick.
#[derive(Default)]
struct RecordingExecutionBackend {
    commands: Mutex<Vec<AgentExecutionCommand>>,
}

#[async_trait]
impl AgentExecutionBackend for RecordingExecutionBackend {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
        self.commands.lock().unwrap().push(command.clone());
        let mut result =
            AgentExecutionResult::completed(&command, serde_json::json!({"accepted": true}));
        result.metadata.insert(
            "result_evidence_ref".into(),
            "event/heartbeat-result/1".into(),
        );
        Ok(result)
    }
}

/// Agent Execution backend that completes without durable evidence.
///
/// The runtime-host evidence gate should reject this result and the Heartbeat
/// run memento should become failed through the generic completion command.
#[derive(Default)]
struct NoEvidenceExecutionBackend;

#[async_trait]
impl AgentExecutionBackend for NoEvidenceExecutionBackend {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
        Ok(AgentExecutionResult::completed(
            &command,
            serde_json::json!({"accepted": true}),
        ))
    }
}

async fn register_static_service(
    runtime: &ServiceRuntime,
    descriptor: ServiceDescriptor,
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

async fn register_heartbeat_service(
    runtime: &ServiceRuntime,
    heartbeat: Arc<macaca_heartbeat::LocalHeartbeatProvider>,
) {
    register_static_service(
        runtime,
        heartbeat.descriptor(),
        Arc::new(HeartbeatSystemServiceProvider::new(heartbeat)),
    )
    .await;
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

fn agent_application_profile(
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

fn heartbeat_declaration(
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

async fn wait_for_recorded_commands(
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

async fn wait_for_run_state(
    heartbeat: &macaca_heartbeat::LocalHeartbeatProvider,
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

#[tokio::test]
async fn heartbeat_tick_hands_off_agent_dispatch_without_blocking_scheduler_lane() {
    let application_id = ApplicationId::from_name("generic-heartbeat-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let heartbeat = Arc::new(macaca_heartbeat::LocalHeartbeatProvider::new());
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
        metadata: BTreeMap::new(),
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

#[tokio::test]
async fn per_agent_profiles_with_distinct_cadences_dispatch_at_separate_ticks() {
    let application_id = ApplicationId::from_name("generic-heartbeat-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let heartbeat = Arc::new(macaca_heartbeat::LocalHeartbeatProvider::new());
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
    register_heartbeat_service(&runtime, Arc::clone(&heartbeat)).await;
    register_static_service(
        &runtime,
        application_service_descriptor(),
        Arc::new(FakeApplicationHeartbeatService::new(vec![
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
        ])),
    )
    .await;
    register_static_service(
        &runtime,
        agent_execution_service_descriptor(),
        Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
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

#[tokio::test]
async fn heartbeat_run_history_records_successful_agent_dispatch_completion() {
    let application_id = ApplicationId::from_name("generic-heartbeat-app");
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let heartbeat = Arc::new(macaca_heartbeat::LocalHeartbeatProvider::new());
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
    register_heartbeat_service(&runtime, Arc::clone(&heartbeat)).await;
    register_static_service(
        &runtime,
        application_service_descriptor(),
        Arc::new(FakeApplicationHeartbeatService::new(vec![
            heartbeat_declaration(application_id, "operator", profile_id, scope_key, 1),
        ])),
    )
    .await;
    register_static_service(
        &runtime,
        agent_execution_service_descriptor(),
        Arc::new(AgentExecutionSystemServiceProvider::new(backend)),
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
    let heartbeat = Arc::new(macaca_heartbeat::LocalHeartbeatProvider::new());
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
    register_heartbeat_service(&runtime, Arc::clone(&heartbeat)).await;
    register_static_service(
        &runtime,
        application_service_descriptor(),
        Arc::new(FakeApplicationHeartbeatService::new(vec![
            heartbeat_declaration(application_id, "operator", profile_id, scope_key, 1),
        ])),
    )
    .await;
    register_static_service(
        &runtime,
        agent_execution_service_descriptor(),
        Arc::new(AgentExecutionSystemServiceProvider::new(Arc::new(
            NoEvidenceExecutionBackend,
        ))),
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
