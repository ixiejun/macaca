//! Contract tests for autonomy dispatch Strategy behavior.
//!
//! **Pattern:** Contract Test — validates scheduled agent execution dispatch
//! through service boundaries using recording backends and fake resolvers.
//! Tests stay at the `ServiceRuntime` syscall surface and do not execute tools
//! or inspect application-specific provider internals.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, AgentExecutionTargetCommand,
    ApplicationId, AutonomousExecutionSourceKind, AutonomyPayloadRef, AutonomyScope, CleanupPolicy,
    ResolveScheduledAgentTaskPayloadCommand, ScheduledAgentTaskId,
    ScheduledAgentTaskResolvedPayload, SchedulerTargetCommand, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceScope, ServiceType,
    TaskId, TraceContext, TraceSchemaRef, SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND,
    SCHEDULED_AGENT_TASK_SERVICE_ID, SCHEDULER_SERVICE_ID,
};
use macaca_skill::{
    SkillAliasKind, SkillAliasRecord, SkillAliasResolutionPolicy, SkillAliasUpsertCommand,
    SkillServiceScope, SKILL_ALIAS_UPSERT_COMMAND,
};

use crate::agent_execution_service_provider::{
    AgentExecutionBackend, AgentExecutionSystemServiceProvider,
};
use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    SkillSystemServiceProvider, StaticServiceProviderFactory,
};

use super::AutonomyDispatchStrategies;

/// Test double that records the command crossing into `service.agent_execution`.
///
/// The double keeps the runtime test at the service boundary.  It does not
/// execute tools, inspect application-specific names, or simulate provider
/// internals; it only proves the Strategy builds the audited command shape
/// that Agent Execution owns.
#[derive(Default)]
struct RecordingExecutionBackend {
    commands: Mutex<Vec<AgentExecutionCommand>>,
    emit_evidence: bool,
    emit_output_hash: bool,
}

#[async_trait]
impl AgentExecutionBackend for RecordingExecutionBackend {
    async fn execute(&self, command: AgentExecutionCommand) -> ServiceResult<AgentExecutionResult> {
        self.commands.lock().unwrap().push(command.clone());
        let mut result =
            AgentExecutionResult::completed(&command, serde_json::json!({"accepted": true}));
        if self.emit_evidence {
            result
                .metadata
                .insert("result_evidence_ref".into(), "event/agent-result/1".into());
        }
        if self.emit_output_hash {
            result
                .metadata
                .insert("result_output_hash".into(), "hash.agent-output.1".into());
        }
        Ok(result)
    }
}

/// ServiceRuntime-facing payload resolver used by dispatch Strategy tests.
///
/// The real Scheduled Agent Task service owns the payload Memento.  This fake
/// implements only the trusted resolve command so the test can assert that
/// Runtime Host does not read raw prompts from Scheduler target metadata.
struct FakeScheduledAgentTaskResolver {
    resolved: ScheduledAgentTaskResolvedPayload,
}

impl FakeScheduledAgentTaskResolver {
    fn descriptor() -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            macaca_proto::KernelServiceId::new(SCHEDULED_AGENT_TASK_SERVICE_ID),
            ServiceType::new("autonomy.scheduled_agent_task"),
            TraceSchemaRef::new("trace.schema.scheduled_agent_task.test"),
        );
        descriptor.health = ServiceHealth::Healthy;
        descriptor.supported_scopes = vec![ServiceScope::Application("*".into())];
        descriptor
    }
}

#[async_trait]
impl SystemService for FakeScheduledAgentTaskResolver {
    fn descriptor(&self) -> ServiceDescriptor {
        Self::descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        if command.name.as_str() != SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let _: ResolveScheduledAgentTaskPayloadCommand = serde_json::from_value(command.payload)
            .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
        Ok(ServiceCallResult {
            output: serde_json::to_value(Some(self.resolved.clone()))
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "resolved".into(),
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

async fn register_static_service(
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

async fn register_skill_alias(
    runtime: &ServiceRuntime,
    source_skill_id: &str,
    target_skill_id: &str,
) {
    let provider = Arc::new(SkillSystemServiceProvider::new());
    let trace = TraceContext::new("trace-scheduled-agent-skill-alias-upsert");
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
                    kind: SkillAliasKind::SupersededBy,
                    resolution_policy: SkillAliasResolutionPolicy::Redirect,
                    valid_from: chrono::Utc::now(),
                    valid_until: None,
                    rationale: "test alias for dispatch boundary".into(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    evidence_ids: vec!["evidence://skill-alias/test".into()],
                },
            })
            .unwrap(),
            trace,
        ))
        .await
        .unwrap();
    register_static_service(runtime, provider.descriptor(), provider).await;
}

fn payload_ref() -> AutonomyPayloadRef {
    let mut payload_ref = AutonomyPayloadRef::new(
        "scheduled-agent-task://payload/test",
        "Analyze the market and record result.",
    )
    .unwrap();
    payload_ref.content_digest = Some("digest.prompt.123".into());
    payload_ref
}

fn resolved_payload(payload_ref: AutonomyPayloadRef) -> ScheduledAgentTaskResolvedPayload {
    ScheduledAgentTaskResolvedPayload {
        task_id: ScheduledAgentTaskId::new("scheduled-agent-task-test").unwrap(),
        application_id: ApplicationId::from_name("scheduled-agent-dispatch-test"),
        session_id: "session-scheduled-agent".into(),
        task_ref: Some(TaskId::new()),
        target_agent: "task-runner".into(),
        execution_intent: AgentExecutionIntent::TaskWorker,
        user_prompt: "Analyze the market and record result.".into(),
        delegated_context: serde_json::json!({"bounded": true}),
        policy: Default::default(),
        payload_ref,
        payload_digest: Some("digest.prompt.123".into()),
        trace: TraceContext::new("trace-resolved-payload"),
        audit_id: Some("audit.scheduled_agent_task.created.1".into()),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn agent_execution_target_resolves_payload_and_invokes_agent_execution_service() {
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig::default());
    let payload_ref = payload_ref();
    let backend = Arc::new(RecordingExecutionBackend {
        emit_evidence: true,
        ..RecordingExecutionBackend::default()
    });

    register_static_service(
        &runtime,
        FakeScheduledAgentTaskResolver::descriptor(),
        Arc::new(FakeScheduledAgentTaskResolver {
            resolved: resolved_payload(payload_ref.clone()),
        }),
    )
    .await;
    register_static_service(
        &runtime,
        AgentExecutionSystemServiceProvider::new(backend.clone()).descriptor(),
        Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
    )
    .await;

    let target = AgentExecutionTargetCommand {
        application_id: ApplicationId::from_name("scheduled-agent-dispatch-test"),
        session_id: "session-scheduled-agent".into(),
        task_id: Some(TaskId::new()),
        target_agent: Some("task-runner".into()),
        execution_intent: AgentExecutionIntent::TaskWorker,
        payload_ref,
        metadata: BTreeMap::new(),
    };
    let dispatcher = AutonomyDispatchStrategies::new(&runtime, 1_000);
    let outcome = dispatcher
        .dispatch(
            TraceContext::new("trace-scheduled-agent-dispatch"),
            AutonomyScope::application(ApplicationId::from_name("scheduled-agent-dispatch-test")),
            SchedulerTargetCommand::AgentExecution(target),
        )
        .await
        .unwrap();

    assert!(outcome.succeeded);
    let commands = backend.commands.lock().unwrap();
    assert_eq!(commands.len(), 1);
    let command = &commands[0];
    assert_eq!(command.execution_intent, AgentExecutionIntent::TaskWorker);
    assert_eq!(command.user_prompt, "Analyze the market and record result.");
    assert_eq!(
        command.metadata["scheduler_run_source"],
        SCHEDULER_SERVICE_ID
    );
    assert_eq!(command.metadata["payload_digest"], "digest.prompt.123");
    assert_eq!(
        command.metadata["scheduled_agent_task_audit_id"],
        "audit.scheduled_agent_task.created.1"
    );
    let envelope = command
        .execution_envelope
        .as_ref()
        .expect("scheduled dispatch must attach an execution envelope");
    assert_eq!(
        envelope.source_kind,
        AutonomousExecutionSourceKind::ScheduledAgentTask
    );
    assert_eq!(
        envelope.source_instruction,
        "Analyze the market and record result."
    );
}

#[tokio::test]
async fn agent_execution_target_resolves_skill_alias_before_agent_execution() {
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig::default());
    let payload_ref = payload_ref();
    let backend = Arc::new(RecordingExecutionBackend {
        emit_evidence: true,
        ..RecordingExecutionBackend::default()
    });
    let mut resolved = resolved_payload(payload_ref.clone());
    resolved.metadata.insert(
        "skill.alias.requested_id".into(),
        "skill://agent/superseded-debug".into(),
    );

    register_static_service(
        &runtime,
        FakeScheduledAgentTaskResolver::descriptor(),
        Arc::new(FakeScheduledAgentTaskResolver { resolved }),
    )
    .await;
    register_skill_alias(
        &runtime,
        "skill://agent/superseded-debug",
        "skill://agent/current-debug",
    )
    .await;
    register_static_service(
        &runtime,
        AgentExecutionSystemServiceProvider::new(backend.clone()).descriptor(),
        Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
    )
    .await;

    let target = AgentExecutionTargetCommand {
        application_id: ApplicationId::from_name("scheduled-agent-dispatch-test"),
        session_id: "session-scheduled-agent".into(),
        task_id: Some(TaskId::new()),
        target_agent: Some("task-runner".into()),
        execution_intent: AgentExecutionIntent::TaskWorker,
        payload_ref,
        metadata: BTreeMap::new(),
    };
    let outcome = AutonomyDispatchStrategies::new(&runtime, 1_000)
        .dispatch(
            TraceContext::new("trace-scheduled-agent-dispatch-skill-alias"),
            AutonomyScope::application(ApplicationId::from_name("scheduled-agent-dispatch-test")),
            SchedulerTargetCommand::AgentExecution(target),
        )
        .await
        .unwrap();

    assert!(outcome.succeeded);
    let commands = backend.commands.lock().unwrap();
    let metadata = &commands[0].metadata;
    assert_eq!(
        metadata["skill.alias.requested_id"],
        "skill://agent/superseded-debug"
    );
    assert_eq!(metadata["skill.alias.resolved"], "true");
    assert_eq!(metadata["skill.alias.status"], "redirected");
    assert_eq!(
        metadata["skill.alias.effective_id"],
        "skill://agent/current-debug"
    );
    assert_eq!(metadata["skill.alias.kind"], "superseded_by");
    assert_eq!(metadata["skill.alias.policy"], "redirect");
}

#[tokio::test]
async fn completed_agent_execution_without_result_evidence_is_retryable() {
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig::default());
    let payload_ref = payload_ref();
    let backend = Arc::new(RecordingExecutionBackend::default());

    register_static_service(
        &runtime,
        FakeScheduledAgentTaskResolver::descriptor(),
        Arc::new(FakeScheduledAgentTaskResolver {
            resolved: resolved_payload(payload_ref.clone()),
        }),
    )
    .await;
    register_static_service(
        &runtime,
        AgentExecutionSystemServiceProvider::new(backend.clone()).descriptor(),
        Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
    )
    .await;

    let target = AgentExecutionTargetCommand {
        application_id: ApplicationId::from_name("scheduled-agent-dispatch-test"),
        session_id: "session-scheduled-agent".into(),
        task_id: Some(TaskId::new()),
        target_agent: Some("task-runner".into()),
        execution_intent: AgentExecutionIntent::TaskWorker,
        payload_ref,
        metadata: BTreeMap::new(),
    };
    let dispatcher = AutonomyDispatchStrategies::new(&runtime, 1_000);
    let outcome = dispatcher
        .dispatch(
            TraceContext::new("trace-scheduled-agent-dispatch-missing-evidence"),
            AutonomyScope::application(ApplicationId::from_name("scheduled-agent-dispatch-test")),
            SchedulerTargetCommand::AgentExecution(target),
        )
        .await
        .unwrap();

    assert!(!outcome.succeeded);
    assert!(outcome.retryable);
    assert_eq!(
        outcome.reason_code,
        "agent_execution_result_evidence_missing"
    );
}

#[tokio::test]
async fn completed_agent_execution_with_result_hash_satisfies_agent_result_policy() {
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig::default());
    let payload_ref = payload_ref();
    let backend = Arc::new(RecordingExecutionBackend {
        emit_output_hash: true,
        ..RecordingExecutionBackend::default()
    });

    register_static_service(
        &runtime,
        FakeScheduledAgentTaskResolver::descriptor(),
        Arc::new(FakeScheduledAgentTaskResolver {
            resolved: resolved_payload(payload_ref.clone()),
        }),
    )
    .await;
    register_static_service(
        &runtime,
        AgentExecutionSystemServiceProvider::new(backend.clone()).descriptor(),
        Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
    )
    .await;

    let target = AgentExecutionTargetCommand {
        application_id: ApplicationId::from_name("scheduled-agent-dispatch-test"),
        session_id: "session-scheduled-agent".into(),
        task_id: Some(TaskId::new()),
        target_agent: Some("task-runner".into()),
        execution_intent: AgentExecutionIntent::TaskWorker,
        payload_ref,
        metadata: BTreeMap::new(),
    };
    let dispatcher = AutonomyDispatchStrategies::new(&runtime, 1_000);
    let outcome = dispatcher
        .dispatch(
            TraceContext::new("trace-scheduled-agent-dispatch-output-hash"),
            AutonomyScope::application(ApplicationId::from_name("scheduled-agent-dispatch-test")),
            SchedulerTargetCommand::AgentExecution(target),
        )
        .await
        .unwrap();

    assert!(outcome.succeeded);
    assert!(!outcome.retryable);
    assert_eq!(outcome.reason_code, "dispatch_succeeded");
}
