//! Test fixtures — provider-neutral doubles for heartbeat dispatch contract tests.
//!
//! Fakes implement only the `SystemService` / `AgentExecutionBackend` contracts
//! so tests prove dispatch metadata, intent, and target agents survive the
//! ServiceRuntime hop without application-specific branches in production code.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_app::application_service_descriptor;
use macaca_kernel::SystemService;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, ApplicationHeartbeatAgentsResult,
    ApplicationId, CleanupPolicy, HeartbeatCommandResult, HeartbeatRunId, HeartbeatWakeDisposition,
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    TraceContext, APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND,
};
use macaca_skill::{
    SkillAliasKind, SkillAliasRecord, SkillAliasResolutionPolicy, SkillAliasUpsertCommand,
    SkillServiceScope, SKILL_ALIAS_UPSERT_COMMAND,
};

use crate::{
    agent_execution_service_descriptor, AgentExecutionBackend, AgentExecutionSystemServiceProvider,
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    SkillSystemServiceProvider, StaticServiceProviderFactory,
};

/// Application Service test double that returns prebuilt manifest projections.
///
/// The fake deliberately implements only the provider-neutral `SystemService`
/// contract. Tests therefore exercise the same ServiceRuntime bus, command
/// decoding, and output shaping used by production dispatch without reaching
/// into concrete application registries or app-specific fixtures.
pub(super) struct FakeApplicationHeartbeatService {
    descriptor: ServiceDescriptor,
    declarations: ApplicationHeartbeatAgentsResult,
}

impl FakeApplicationHeartbeatService {
    pub(super) fn new(declarations: ApplicationHeartbeatAgentsResult) -> Self {
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
            return Err(ServiceError::UnsupportedCommand(format!(
                "unsupported fake application command {}",
                command.name
            )));
        }
        Ok(ServiceCallResult {
            status: "ok".into(),
            output: serde_json::to_value(&self.declarations)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            metadata: Default::default(),
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

/// Agent Execution backend that records typed commands for assertions.
///
/// The Strategy under test only sees the service boundary. Capturing the
/// decoded command here proves the dispatch metadata, intent, and target
/// agent survive the ServiceRuntime hop without exposing test-only branches
/// in production code.
#[derive(Default)]
pub(super) struct RecordingExecutionBackend {
    pub(super) commands: Mutex<Vec<AgentExecutionCommand>>,
}

#[async_trait]
impl AgentExecutionBackend for RecordingExecutionBackend {
    async fn execute(
        &self,
        command: AgentExecutionCommand,
    ) -> ServiceResult<AgentExecutionResult> {
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

/// Register a static service provider on the test runtime bus.
pub(super) async fn register_static_service(
    runtime: &ServiceRuntime,
    descriptor: ServiceDescriptor,
    service: Arc<dyn SystemService>,
) {
    let factory =
        StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service));
    runtime
        .register_provider(&factory, ServiceProviderFactoryContext::new())
        .await
        .unwrap();
}

/// Register a skill alias redirect used by skill-alias resolution contract tests.
pub(super) async fn register_skill_alias(
    runtime: &ServiceRuntime,
    source_skill_id: &str,
    target_skill_id: &str,
) {
    let provider = Arc::new(SkillSystemServiceProvider::new());
    let trace = TraceContext::new("trace-heartbeat-skill-alias-upsert");
    provider
        .call(ServiceCommand::with_trace(
            macaca_proto::ServiceCommandName::new(SKILL_ALIAS_UPSERT_COMMAND),
            serde_json::to_value(SkillAliasUpsertCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                record: SkillAliasRecord {
                    source_skill_id: source_skill_id.into(),
                    source_name: "source-skill".into(),
                    target_skill_id: target_skill_id.into(),
                    target_name: "target-skill".into(),
                    kind: SkillAliasKind::AbsorbedInto,
                    resolution_policy: SkillAliasResolutionPolicy::Redirect,
                    valid_from: chrono::Utc::now(),
                    valid_until: None,
                    rationale: "test alias for heartbeat dispatch boundary".into(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    evidence_ids: vec!["evidence://skill-alias/heartbeat".into()],
                },
            })
            .unwrap(),
            trace,
        ))
        .await
        .unwrap();
    register_static_service(runtime, provider.descriptor(), provider).await;
}

/// Build an accepted application-scoped wake with default session metadata.
pub(super) fn accepted_app_wake(application_id: ApplicationId) -> HeartbeatCommandResult {
    let mut metadata = BTreeMap::new();
    metadata.insert("application_id".into(), application_id.to_string());
    metadata.insert("session_id".into(), "session-heartbeat".into());
    accepted_app_wake_with_metadata(application_id, metadata)
}

/// Build an accepted application-scoped wake with caller-supplied metadata.
pub(super) fn accepted_app_wake_with_metadata(
    application_id: ApplicationId,
    mut metadata: BTreeMap<String, String>,
) -> HeartbeatCommandResult {
    metadata
        .entry("application_id".into())
        .or_insert_with(|| application_id.to_string());
    HeartbeatCommandResult {
        run_id: Some(HeartbeatRunId::new("run-heartbeat").unwrap()),
        state: None,
        disposition: HeartbeatWakeDisposition::Accepted,
        gates: Vec::new(),
        accepted: true,
        error: None,
        trace: TraceContext::new("trace-heartbeat-dispatch"),
        audit_id: Some("audit-heartbeat".into()),
        metadata,
    }
}

