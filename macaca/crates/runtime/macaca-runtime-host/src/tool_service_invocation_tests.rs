use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    ApplicationId, CapabilityToolDescriptor, CapabilityToolInvocationResult,
    CapabilityToolInvocationScope, CapabilityToolOriginKind, CleanupPolicy,
    IndustrialToolDescriptor, KernelServiceId, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceScope, ServiceType,
    ToolExecutorRouteKind, ToolFamilyRef, ToolInvokeCommand, ToolResultClass, ToolResultGetCommand,
    TraceContext, TraceSchemaRef, TOOL_INVOKE_COMMAND, TOOL_RESULT_GET_COMMAND,
};
use serde_json::json;

use crate::{
    bootstrap_local_approval_service, bootstrap_local_file_service,
    bootstrap_tool_planning_service, ServiceProviderFactoryContext, ServiceProviderInstance,
    ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory, ToolPlanningService,
};

#[tokio::test]
async fn tool_invoke_routes_descriptor_to_owning_service() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let owner = Arc::new(EchoToolOwner::new(
        "service.driver",
        CapabilityToolOriginKind::Driver,
    ));
    register_owner(runtime.clone(), owner.clone()).await;
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(ToolPlanningService::builder().build()),
        "trace-bootstrap-tool-invoke",
    )
    .await
    .unwrap();

    let result = invoke_tool(
        runtime.clone(),
        descriptor("service.driver", CapabilityToolOriginKind::Driver),
        json!({"value": 42}),
    )
    .await;

    assert_eq!(result.status, "ok");
    assert_eq!(result.result_class, ToolResultClass::StructuredJson);
    assert_eq!(result.inline_output.unwrap()["routed_to"], "service.driver");
    assert_eq!(owner.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn tool_invoke_policy_denial_does_not_dispatch_to_owner() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let owner = Arc::new(EchoToolOwner::new(
        "service.skill",
        CapabilityToolOriginKind::Skill,
    ));
    register_owner(runtime.clone(), owner.clone()).await;
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(ToolPlanningService::builder().build()),
        "trace-bootstrap-tool-policy",
    )
    .await
    .unwrap();

    let mut command = invoke_command(
        descriptor("service.skill", CapabilityToolOriginKind::Skill),
        json!({}),
    );
    command
        .metadata
        .insert("policy.decision".into(), "deny".into());
    let result = call_tool_service(runtime.clone(), TOOL_INVOKE_COMMAND, command).await;

    assert_eq!(result.status, "denied");
    assert_eq!(result.result_class, ToolResultClass::Failure);
    assert_eq!(owner.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn tool_invoke_oversized_result_becomes_retrievable_artifact_ref() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let owner = Arc::new(EchoToolOwner::new(
        "service.mcp",
        CapabilityToolOriginKind::Mcp,
    ));
    register_owner(runtime.clone(), owner).await;
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(ToolPlanningService::builder().build()),
        "trace-bootstrap-tool-artifact",
    )
    .await
    .unwrap();

    let mut command = invoke_command(
        descriptor("service.mcp", CapabilityToolOriginKind::Mcp),
        json!({"large": "x".repeat(96)}),
    );
    command
        .metadata
        .insert("result.inline_budget_bytes".into(), "16".into());
    let invoked = call_tool_service(runtime.clone(), TOOL_INVOKE_COMMAND, command).await;
    let artifact_ref = invoked.artifact_refs.first().cloned().unwrap();
    let invocation_ref = invoked.invocation_ref.clone().unwrap();

    assert_eq!(invoked.status, "ok");
    assert_eq!(invoked.result_class, ToolResultClass::BinaryArtifact);
    assert!(invoked.inline_output.unwrap()["artifact_ref"]
        .as_str()
        .is_some());

    let fetched = call_tool_service(
        runtime,
        TOOL_RESULT_GET_COMMAND,
        ToolResultGetCommand {
            trace: TraceContext::new("trace-tool-result-get"),
            invocation_ref,
            metadata: Default::default(),
        },
    )
    .await;

    assert_eq!(fetched.status, "ok");
    assert_eq!(fetched.artifact_refs, vec![artifact_ref]);
}

#[tokio::test]
async fn tool_invoke_runtime_environment_route_executes_without_mcp_owner() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(ToolPlanningService::builder().build()),
        "trace-bootstrap-tool-runtime-env",
    )
    .await
    .unwrap();

    let mut descriptor = family_descriptor("file", "service.tool.runtime_environment");
    descriptor.executor_route = descriptor.executor_route.with_route_kind(
        ToolExecutorRouteKind::RuntimeEnvironment,
        Some("tool.runtime_environment.invoke".into()),
    );
    let result = invoke_tool(runtime, descriptor, json!({"operation": "status"})).await;

    assert_eq!(result.status, "ok");
    assert_eq!(
        result.inline_output.unwrap()["route_kind"],
        "runtime_environment"
    );
}

#[tokio::test]
async fn tool_invoke_managed_gateway_route_executes_with_metering_ref() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(ToolPlanningService::builder().build()),
        "trace-bootstrap-tool-gateway",
    )
    .await
    .unwrap();

    let mut descriptor = family_descriptor("web", "service.tool.managed_gateway");
    descriptor.executor_route = descriptor.executor_route.with_route_kind(
        ToolExecutorRouteKind::ManagedGateway,
        Some("tool.managed_gateway.invoke".into()),
    );
    let result = invoke_tool(runtime, descriptor, json!({"operation": "lookup"})).await;

    assert_eq!(result.status, "ok");
    assert_eq!(
        result.inline_output.unwrap()["route_kind"],
        "managed_gateway"
    );
}

#[tokio::test]
async fn tool_invoke_file_family_routes_to_service_file_provider() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("input.txt"), "from service.file").unwrap();
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    bootstrap_local_file_service(runtime.clone(), "trace-bootstrap-file-tool")
        .await
        .unwrap();
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(ToolPlanningService::builder().build()),
        "trace-bootstrap-tool-file",
    )
    .await
    .unwrap();

    let mut descriptor = family_descriptor("file", macaca_proto::workbench::file::SERVICE_ID);
    descriptor.executor_route = descriptor.executor_route.with_route_kind(
        ToolExecutorRouteKind::OwningServiceCommand,
        Some(macaca_proto::workbench::file::TOOL_INVOKE_COMMAND.into()),
    );
    let result = invoke_tool(
        runtime,
        descriptor,
        json!({
            "operation": "read",
            "target": {
                "workspace_root": workspace.path().to_string_lossy(),
                "path": "input.txt",
                "follow_symlinks": false
            },
            "inline_budget_bytes": 1024
        }),
    )
    .await;

    assert_eq!(result.status, "ok");
    assert_eq!(
        result.inline_output.unwrap()["Read"]["content"],
        serde_json::Value::String("from service.file".into())
    );
}

#[tokio::test]
async fn tool_invoke_workbench_family_routes_typed_payload_to_owning_service() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    bootstrap_local_approval_service(runtime.clone(), "trace-bootstrap-approval-tool")
        .await
        .unwrap();
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(ToolPlanningService::builder().build()),
        "trace-bootstrap-tool-approval",
    )
    .await
    .unwrap();

    let mut descriptor =
        family_descriptor("approval", macaca_proto::workbench::approval::SERVICE_ID);
    descriptor.executor_route = descriptor.executor_route.with_route_kind(
        ToolExecutorRouteKind::OwningServiceCommand,
        Some(macaca_proto::workbench::approval::POLICY_EXPLAIN_COMMAND.into()),
    );
    let result = invoke_tool(
        runtime,
        descriptor,
        json!({
            "side_effect_class": "tool_invocation",
            "approval_profile_ref": null,
            "metadata": {}
        }),
    )
    .await;

    assert_eq!(result.status, "ok");
    assert_eq!(result.result_class, ToolResultClass::StructuredJson);
    assert_eq!(
        result.inline_output.unwrap()["output"]["PolicyExplanation"]["required"],
        true
    );
}

#[tokio::test]
async fn tool_invoke_entitlement_gate_stops_before_owner_dispatch() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let owner = Arc::new(EchoToolOwner::new(
        "service.driver",
        CapabilityToolOriginKind::Driver,
    ));
    register_owner(runtime.clone(), owner.clone()).await;
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(ToolPlanningService::builder().build()),
        "trace-bootstrap-tool-entitlement",
    )
    .await
    .unwrap();

    let mut command = invoke_command(
        descriptor("service.driver", CapabilityToolOriginKind::Driver),
        json!({}),
    );
    command
        .metadata
        .insert("entitlement.required".into(), "capability.paid".into());
    let result = call_tool_service(runtime, TOOL_INVOKE_COMMAND, command).await;

    assert_eq!(result.status, "entitlement_missing");
    assert_eq!(owner.calls.load(Ordering::SeqCst), 0);
}

async fn invoke_tool(
    runtime: Arc<ServiceRuntime>,
    descriptor: IndustrialToolDescriptor,
    input: serde_json::Value,
) -> macaca_proto::ToolInvokeResult {
    call_tool_service(
        runtime,
        TOOL_INVOKE_COMMAND,
        invoke_command(descriptor, input),
    )
    .await
}

fn invoke_command(
    descriptor: IndustrialToolDescriptor,
    input: serde_json::Value,
) -> ToolInvokeCommand {
    ToolInvokeCommand {
        trace: TraceContext::new("trace-tool-invoke"),
        scope: CapabilityToolInvocationScope::new(ApplicationId::new(), "session-a", "agent-a")
            .unwrap(),
        tool_id: descriptor.stable_tool_id.clone(),
        descriptor: Some(descriptor),
        input,
        policy_ref: None,
        approval_ref: None,
        metadata: Default::default(),
    }
}

async fn call_tool_service<T: serde::Serialize>(
    runtime: Arc<ServiceRuntime>,
    command_name: &str,
    payload: T,
) -> macaca_proto::ToolCommandResult {
    let trace = TraceContext::new(format!("trace-{command_name}"));
    let command = ServiceCommand::with_trace(
        macaca_proto::ServiceCommandName::new(command_name),
        serde_json::to_value(payload).unwrap(),
        trace,
    );
    let reply = runtime
        .call(
            &KernelServiceId::new(macaca_proto::TOOL_SERVICE_ID),
            macaca_proto::ServiceBusSource::new("tool-invocation-test"),
            command,
        )
        .await
        .unwrap();
    serde_json::from_value(reply.output.unwrap()).unwrap()
}

async fn register_owner(runtime: Arc<ServiceRuntime>, owner: Arc<EchoToolOwner>) {
    let descriptor = owner.descriptor();
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, owner)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .unwrap();
}

fn descriptor(service_id: &str, origin_kind: CapabilityToolOriginKind) -> IndustrialToolDescriptor {
    let base = CapabilityToolDescriptor::new(
        service_id,
        "provider.echo",
        "capability.echo",
        "echo_tool",
        "Echo through owning service",
        json!({"type": "object"}),
        origin_kind,
    )
    .unwrap();
    IndustrialToolDescriptor::new(
        format!("tool.{service_id}.echo"),
        "echo_tool",
        "Echo Tool",
        ToolFamilyRef::new("test").unwrap(),
        base,
    )
    .unwrap()
}

fn family_descriptor(family: &str, service_id: &str) -> IndustrialToolDescriptor {
    let base = CapabilityToolDescriptor::new(
        service_id,
        format!("provider.tool.family.{family}"),
        format!("capability.tool.family.{family}"),
        format!("{family}_tool"),
        format!("Generic {family} family tool"),
        json!({"type": "object"}),
        CapabilityToolOriginKind::Mcp,
    )
    .unwrap();
    IndustrialToolDescriptor::new(
        format!("tool.family.{family}.default"),
        format!("{family}_tool"),
        format!("{family} family provider"),
        ToolFamilyRef::new(family).unwrap(),
        base,
    )
    .unwrap()
}

struct EchoToolOwner {
    service_id: &'static str,
    origin_kind: CapabilityToolOriginKind,
    calls: AtomicUsize,
}

impl EchoToolOwner {
    fn new(service_id: &'static str, origin_kind: CapabilityToolOriginKind) -> Self {
        Self {
            service_id,
            origin_kind,
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl SystemService for EchoToolOwner {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(self.service_id),
            ServiceType::new("test.tool.owner"),
            TraceSchemaRef::new("trace.test.tool.owner"),
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
            self.service_id,
            self.origin_kind.clone(),
            "echo_tool",
            json!({"routed_to": self.service_id, "input": command.payload}),
            trace.clone(),
        );
        Ok(ServiceCallResult {
            output: serde_json::to_value(output).unwrap(),
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
