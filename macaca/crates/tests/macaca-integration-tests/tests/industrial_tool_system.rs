//! Application-neutral industrial tool-system integration proof.
//!
//! The test wires `service.tool` with the generic family provider contributor,
//! registers a minimal owner service for one family, then proves the full
//! planning, invocation, artifact, and audit-replay chain without embedding any
//! application workflow or raw provider payload in OS-level code.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    ApplicationId, CapabilityToolInvocationResult, CapabilityToolInvocationScope,
    CapabilityToolOriginKind, CleanupPolicy, KernelServiceId, McpToolInvokeCommand,
    ServiceBusSource, ServiceCallResult, ServiceCommand, ServiceCommandName, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, ServiceScope, ServiceType, ToolArtifactOpenCommand,
    ToolCatalogPlanCommand, ToolGenericTraceCommand, ToolInvokeCommand, ToolResultClass,
    ToolResultGetCommand, ToolsetRef, TraceContext, TraceSchemaRef, MCP_TOOL_INVOKE_COMMAND,
    TOOL_ARTIFACT_OPEN_COMMAND, TOOL_AUDIT_QUERY_COMMAND, TOOL_CATALOG_PLAN_COMMAND,
    TOOL_INVOKE_COMMAND, TOOL_RESULT_GET_COMMAND, TOOL_SERVICE_ID,
};
use macaca_runtime_host::{
    bootstrap_tool_planning_service, industrial_tool_family_provider_contributor,
    industrial_tool_family_toolsets, AvailabilitySignalSet, ServiceProviderFactoryContext,
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
    ToolPlanningService,
};
use serde_json::json;

#[tokio::test]
async fn industrial_tool_system_plans_invokes_artifacts_and_audit_replay() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    register_owner(runtime.clone()).await;
    let planner = ToolPlanningService::builder()
        .with_contributor(industrial_tool_family_provider_contributor().unwrap())
        .with_availability(
            AvailabilitySignalSet::default()
                .with_service("service.tool.family.web")
                .with_service("service.tool.family.file")
                .with_service("service.tool.family.shell")
                .with_service("service.tool.family.memory")
                .with_service("service.tool.family.document")
                .with_service("service.tool.family.scheduler"),
        )
        .with_toolsets(industrial_tool_family_toolsets().unwrap())
        .build();
    bootstrap_tool_planning_service(runtime.clone(), Arc::new(planner), "trace-proof-bootstrap")
        .await
        .unwrap();

    let mut plan_command =
        ToolCatalogPlanCommand::new(TraceContext::new("trace-industrial-plan")).unwrap();
    plan_command.requested_toolsets = vec![ToolsetRef::new("industrial.proof").unwrap()];
    plan_command.include_hidden = true;
    let plan = call_tool::<_, macaca_proto::ToolCatalogPlanResult>(
        runtime.clone(),
        TOOL_CATALOG_PLAN_COMMAND,
        plan_command,
    )
    .await;
    assert_eq!(plan.visible.len(), 6);
    assert_eq!(plan.hidden.len(), 0);

    let descriptor = plan
        .visible
        .iter()
        .find(|entry| entry.descriptor.family.as_str() == "document")
        .map(|entry| entry.descriptor.clone())
        .unwrap();
    let mut invoke = ToolInvokeCommand {
        trace: TraceContext::new("trace-industrial-invoke"),
        scope: CapabilityToolInvocationScope::new(ApplicationId::new(), "session-proof", "agent")
            .unwrap(),
        tool_id: descriptor.stable_tool_id.clone(),
        descriptor: Some(descriptor),
        input: json!({"payload": "RAW_PROVIDER_PAYLOAD_SHOULD_NOT_LEAK", "repeat": "x".repeat(96)}),
        policy_ref: None,
        approval_ref: None,
        metadata: Default::default(),
    };
    invoke
        .metadata
        .insert("result.inline_budget_bytes".into(), "32".into());
    let invoked = call_tool::<_, macaca_proto::ToolCommandResult>(
        runtime.clone(),
        TOOL_INVOKE_COMMAND,
        invoke,
    )
    .await;

    assert_eq!(invoked.status, "ok");
    assert_eq!(invoked.result_class, ToolResultClass::BinaryArtifact);
    let artifact_ref = invoked.artifact_refs.first().cloned().unwrap();
    let invocation_ref = invoked.invocation_ref.clone().unwrap();

    let fetched = call_tool::<_, macaca_proto::ToolCommandResult>(
        runtime.clone(),
        TOOL_RESULT_GET_COMMAND,
        ToolResultGetCommand {
            trace: TraceContext::new("trace-industrial-result"),
            invocation_ref,
            metadata: Default::default(),
        },
    )
    .await;
    assert_eq!(fetched.artifact_refs, vec![artifact_ref.clone()]);

    let artifact = call_tool::<_, macaca_proto::ToolCommandResult>(
        runtime.clone(),
        TOOL_ARTIFACT_OPEN_COMMAND,
        ToolArtifactOpenCommand {
            trace: TraceContext::new("trace-industrial-artifact"),
            artifact_ref,
            metadata: Default::default(),
        },
    )
    .await;
    assert_eq!(artifact.status, "ok");

    let audit = call_tool::<_, macaca_proto::ToolCommandResult>(
        runtime,
        TOOL_AUDIT_QUERY_COMMAND,
        ToolGenericTraceCommand {
            trace: TraceContext::new("trace-industrial-audit"),
            metadata: Default::default(),
        },
    )
    .await;
    let audit_payload = serde_json::to_string(&audit.inline_output).unwrap();
    assert!(audit_payload.contains("hash:"));
    assert!(!audit_payload.contains("RAW_PROVIDER_PAYLOAD_SHOULD_NOT_LEAK"));
}

async fn call_tool<T, R>(runtime: Arc<ServiceRuntime>, command_name: &str, payload: T) -> R
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let trace = TraceContext::new(format!("trace-{command_name}"));
    let command = ServiceCommand::with_trace(
        ServiceCommandName::new(command_name),
        serde_json::to_value(payload).unwrap(),
        trace,
    );
    let reply = runtime
        .call(
            &KernelServiceId::new(TOOL_SERVICE_ID),
            ServiceBusSource::new("industrial-tool-system-test"),
            command,
        )
        .await
        .unwrap();
    serde_json::from_value(reply.output.unwrap()).unwrap()
}

async fn register_owner(runtime: Arc<ServiceRuntime>) {
    let owner = Arc::new(IndustrialProofOwner);
    let descriptor = owner.descriptor();
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, owner)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .unwrap();
}

struct IndustrialProofOwner;

#[async_trait]
impl SystemService for IndustrialProofOwner {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new("service.tool.family.document"),
            ServiceType::new("industrial.tool.proof"),
            TraceSchemaRef::new("trace.industrial.tool.proof"),
        );
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        if command.name.as_str() != MCP_TOOL_INVOKE_COMMAND {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let typed: McpToolInvokeCommand = serde_json::from_value(command.payload)
            .map_err(|err| ServiceError::InvalidArgument(err.to_string()))?;
        let result = CapabilityToolInvocationResult::ok(
            "service.tool.family.document",
            CapabilityToolOriginKind::Mcp,
            typed.invocation.tool_name,
            json!({
                "summary": "sanitized industrial proof output",
                "large": "x".repeat(128),
            }),
            trace.clone(),
        );
        Ok(ServiceCallResult {
            output: serde_json::to_value(result).unwrap(),
            trace,
            status: "ok".into(),
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
