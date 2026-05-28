//! Service-execution steps for the application-neutral proof fixture.
//!
//! These helpers execute the proof through provider-neutral service commands.
//! They are test-owned orchestration, so no application workflow semantics leak
//! into runtime-host production services.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::{approval, hook, interaction, process, review, sandbox};
use macaca_proto::{
    ApplicationId, CapabilityToolDescriptor, CapabilityToolInvocationResult,
    CapabilityToolInvocationScope, CapabilityToolOriginKind, CleanupPolicy,
    IndustrialToolDescriptor, KernelServiceId, ServiceBusSource, ServiceCallResult, ServiceCommand,
    ServiceCommandName, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    ServiceScope, ServiceType, ToolExecutorRouteKind, ToolFamilyRef, ToolInvokeCommand,
    TraceContext, TraceSchemaRef, WorkbenchCommandResult, WorkbenchCommandScope,
    TOOL_INVOKE_COMMAND, TOOL_SERVICE_ID,
};
use macaca_runtime_host::{
    bootstrap_tool_planning_service, ApprovalSystemServiceProvider, HookSystemServiceProvider,
    ProcessSystemServiceProvider, SandboxSystemServiceProvider, ServiceProviderFactoryContext,
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
    ToolPlanningService,
};
use serde_json::json;

use super::{call_service, push_audit, summary, ProcessExecOutput};

const APP_SESSION: &str = "proof-session";

pub async fn approve_patch(
    provider: &ApprovalSystemServiceProvider,
    audit_refs: &mut Vec<String>,
) -> String {
    let created = call_service::<approval::ApprovalServiceResponse>(
        provider,
        approval::REQUEST_CREATE_COMMAND,
        approval::ApprovalRequestCreate {
            action_summary: "apply workspace patch through service.git".into(),
            side_effect_class: approval::ApprovalSideEffectClass::GitPatch,
            approval_profile_ref: Some("approval.managed".into()),
            reviewer_hint: None,
            reason_code: "proof_patch_requires_approval".into(),
            target_refs: vec!["git://workspace/README.md".into()],
            trace_refs: vec!["trace-proof-git-apply-patch".into()],
            expires_at: None,
            metadata: BTreeMap::new(),
        },
        "trace-proof-approval-create",
    )
    .await;
    let approval_ref = match &created.output {
        Some(approval::ApprovalServiceResponse::Created(record)) => record.approval_ref.clone(),
        _ => panic!("expected approval record"),
    };
    push_audit(&created, audit_refs);
    let resolved = call_service::<approval::ApprovalServiceResponse>(
        provider,
        approval::REQUEST_RESOLVE_COMMAND,
        approval::ApprovalRequestResolve {
            approval_ref: approval_ref.clone(),
            decision: approval::ApprovalDecision::Approve,
            reviewer_ref: "operator://proof".into(),
            reason_code: "approved_for_generic_fixture".into(),
            decision_summary: Some("approved bounded proof mutation".into()),
            trace_refs: vec!["trace-proof-approval-resolve".into()],
        },
        "trace-proof-approval-resolve",
    )
    .await;
    assert!(matches!(
        resolved.output,
        Some(approval::ApprovalServiceResponse::Resolved(ref record))
            if record.status == approval::ApprovalRequestStatus::Approved
    ));
    push_audit(&resolved, audit_refs);
    approval_ref
}

pub async fn run_hook(
    provider: &HookSystemServiceProvider,
    scope: &WorkbenchCommandScope,
    pre: bool,
    audit_refs: &mut Vec<String>,
) {
    let hook_scope = hook::HookScope {
        application_id: Some(scope.application_id.to_string()),
        session_id: Some(scope.session_id.clone()),
        thread_ref: scope.thread_ref.clone(),
        turn_ref: scope.turn_ref.clone(),
        tool_family: Some("git".into()),
        command_name: Some("git.apply_patch".into()),
    };
    let result = if pre {
        call_service::<hook::HookServiceResponse>(
            provider,
            hook::PRE_TOOL_RUN_COMMAND,
            hook::HookPreToolRun {
                scope: hook_scope,
                managed_only: true,
                input_summary: Some(summary("git patch request")),
                metadata: BTreeMap::new(),
            },
            "trace-proof-hook-pre",
        )
        .await
    } else {
        call_service::<hook::HookServiceResponse>(
            provider,
            hook::POST_TOOL_RUN_COMMAND,
            hook::HookPostToolRun {
                scope: hook_scope,
                managed_only: true,
                output_summary: Some(summary("git patch completed")),
                metadata: BTreeMap::new(),
            },
            "trace-proof-hook-post",
        )
        .await
    };
    let audit_ref = match &result.output {
        Some(hook::HookServiceResponse::Run(run)) => run.audit_ref.clone(),
        _ => panic!("expected hook run"),
    };
    push_audit(&result, audit_refs);
    let audit = call_service::<hook::HookServiceResponse>(
        provider,
        hook::RESULT_AUDIT_COMMAND,
        hook::HookResultAuditRead { audit_ref },
        "trace-proof-hook-audit",
    )
    .await;
    assert!(matches!(
        audit.output,
        Some(hook::HookServiceResponse::Audit(_))
    ));
}

pub async fn prepare_sandbox(
    provider: &SandboxSystemServiceProvider,
    root: &std::path::Path,
    scope: &WorkbenchCommandScope,
    audit_refs: &mut Vec<String>,
) -> sandbox::SandboxEnvironmentRecord {
    let result = call_service::<sandbox::SandboxServiceResponse>(
        provider,
        sandbox::ENVIRONMENT_PREPARE_COMMAND,
        sandbox::SandboxEnvironmentPrepareRequest {
            requested_profile_ref: sandbox::WORKSPACE_WRITE_PROFILE.into(),
            runtime_kind: sandbox::SandboxRuntimeKind::LocalSandbox,
            scope: sandbox::SandboxEnvironmentScope::Session,
            workspace_root: Some(root.to_string_lossy().to_string()),
            cwd: Some(".".into()),
            application_id: Some(scope.application_id.to_string()),
            session_id: Some(scope.session_id.clone()),
            thread_ref: scope.thread_ref.clone(),
            resource_request_refs: vec!["process".into()],
        },
        "trace-proof-sandbox-prepare",
    )
    .await;
    push_audit(&result, audit_refs);
    match result.output {
        Some(sandbox::SandboxServiceResponse::EnvironmentPrepared { record }) => record,
        _ => panic!("expected sandbox environment"),
    }
}

pub async fn run_process_test(
    provider: &ProcessSystemServiceProvider,
    root: &std::path::Path,
    scope: &WorkbenchCommandScope,
    sandbox: &sandbox::SandboxEnvironmentRecord,
    audit_refs: &mut Vec<String>,
) -> ProcessExecOutput {
    let result = call_service::<process::ProcessServiceResponse>(
        provider,
        process::EXEC_COMMAND,
        process::ProcessCommandRequest {
            executable: "sh".into(),
            args: vec![
                "-c".into(),
                "grep -q 'proof_marker: after' README.md".into(),
            ],
            env: Vec::new(),
            sandbox: process::ProcessSandboxRequest {
                workspace_root: root.to_string_lossy().to_string(),
                cwd: Some(".".into()),
                sandbox_profile_ref: sandbox.environment_id.clone(),
                permission_profile_ref: sandbox::WORKSPACE_WRITE_PROFILE.into(),
                approval_ref: None,
                resource_lease_refs: sandbox
                    .resource_leases
                    .iter()
                    .map(|lease| lease.lease_ref.clone())
                    .collect(),
            },
            application_id: Some(scope.application_id.to_string()),
            session_id: Some(scope.session_id.clone()),
            thread_ref: scope.thread_ref.clone(),
            inline_output_budget_bytes: 1024,
            timeout_ms: Some(5000),
            allocate_pty: false,
        },
        "trace-proof-process-test",
    )
    .await;
    push_audit(&result, audit_refs);
    match result.output {
        Some(process::ProcessServiceResponse::Exec { record, .. }) => ProcessExecOutput { record },
        _ => panic!("expected process exec"),
    }
}

pub async fn start_review(
    provider: &macaca_runtime_host::ReviewSystemServiceProvider,
    audit_refs: &[String],
) -> String {
    let started = call_service::<review::ReviewServiceResponse>(
        provider,
        review::START_COMMAND,
        review::ReviewStartRequest {
            subject_ref: "change://application-neutral-proof".into(),
            changed_paths: vec!["README.md".into()],
            evidence_refs: audit_refs.to_vec(),
            rationale: Some(summary("risk: generic proof changed a workspace file")),
        },
        "trace-proof-review-start",
    )
    .await;
    match started.output {
        Some(review::ReviewServiceResponse::Started(run)) => run.review_ref,
        _ => panic!("expected review start"),
    }
}

pub async fn review_result(
    provider: &macaca_runtime_host::ReviewSystemServiceProvider,
    review_ref: &str,
    audit_refs: &mut Vec<String>,
) -> review::ReviewResult {
    let result = call_service::<review::ReviewServiceResponse>(
        provider,
        review::RESULT_COMMAND,
        review::ReviewRefRequest {
            review_ref: review_ref.into(),
        },
        "trace-proof-review-result",
    )
    .await;
    push_audit(&result, audit_refs);
    match result.output {
        Some(review::ReviewServiceResponse::Result(result)) => result,
        _ => panic!("expected review result"),
    }
}

pub async fn interaction_snapshot(
    provider: &macaca_runtime_host::InteractionSystemServiceProvider,
    scope: &WorkbenchCommandScope,
) -> WorkbenchCommandResult<interaction::InteractionResponse> {
    super::call_interaction(
        provider,
        interaction::SNAPSHOT_COMMAND,
        interaction::InteractionSnapshotRequest { limit: 100 },
        scope.clone(),
        "trace-proof-interaction-snapshot",
    )
    .await
}

pub async fn invoke_skill_tool() -> macaca_proto::ToolInvokeResult {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let owner = Arc::new(EchoToolOwner::new());
    let descriptor = owner.descriptor();
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, owner)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .unwrap();
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(ToolPlanningService::builder().build()),
        "trace-proof-tool-bootstrap",
    )
    .await
    .unwrap();
    let descriptor = skill_descriptor();
    let command = ToolInvokeCommand {
        trace: TraceContext::new("trace-proof-tool-skill"),
        scope: CapabilityToolInvocationScope::new(ApplicationId::new(), APP_SESSION, "agent-proof")
            .unwrap(),
        tool_id: descriptor.stable_tool_id.clone(),
        descriptor: Some(descriptor),
        input: json!({"operation": "proof"}),
        policy_ref: None,
        approval_ref: None,
        metadata: BTreeMap::new(),
    };
    let reply = runtime
        .call(
            &KernelServiceId::new(TOOL_SERVICE_ID),
            ServiceBusSource::new("application-neutral-proof"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(TOOL_INVOKE_COMMAND),
                serde_json::to_value(command).unwrap(),
                TraceContext::new("trace-proof-tool-service-call"),
            ),
        )
        .await
        .unwrap();
    serde_json::from_value(reply.output.unwrap()).unwrap()
}

fn skill_descriptor() -> IndustrialToolDescriptor {
    let base = CapabilityToolDescriptor::new(
        macaca_skill::SKILL_SERVICE_ID,
        "provider.generic-proof-skill",
        "capability.generic-proof-skill",
        "generic_proof_skill",
        "Application-neutral proof skill route",
        json!({"type": "object", "additionalProperties": true}),
        CapabilityToolOriginKind::Skill,
    )
    .unwrap();
    let mut descriptor = IndustrialToolDescriptor::new(
        "tool.generic-proof.skill",
        "generic_proof_skill",
        "Generic Proof Skill",
        ToolFamilyRef::new("skill").unwrap(),
        base,
    )
    .unwrap();
    descriptor.executor_route = descriptor
        .executor_route
        .with_route_kind(ToolExecutorRouteKind::Skill, Some("skill.invoke".into()));
    descriptor
}

struct EchoToolOwner {
    calls: AtomicUsize,
}

impl EchoToolOwner {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl SystemService for EchoToolOwner {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(macaca_skill::SKILL_SERVICE_ID),
            ServiceType::new("proof.tool.owner"),
            TraceSchemaRef::new("trace.proof.tool.owner"),
        );
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        let output = CapabilityToolInvocationResult::ok(
            macaca_skill::SKILL_SERVICE_ID,
            CapabilityToolOriginKind::Skill,
            "generic_proof_skill",
            json!({"routed_to": macaca_skill::SKILL_SERVICE_ID}),
            trace.clone(),
        );
        Ok(ServiceCallResult {
            output: serde_json::to_value(output).unwrap(),
            trace,
            status: "ok".into(),
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
