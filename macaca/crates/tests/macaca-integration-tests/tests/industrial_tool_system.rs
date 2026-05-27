//! Application-neutral industrial tool-system integration proof.
//!
//! This proof intentionally avoids fake owner services and manually injected
//! availability signals. The runtime-host industrial composition helper builds
//! provider-backed family descriptors, typed executor routes, availability, and
//! toolsets exactly like production Web startup. The test then proves planning,
//! typed runtime invocation, artifact handling, provider health, and sanitized
//! audit replay through `service.tool`.

use std::sync::Arc;

use macaca_proto::{
    ApplicationId, CapabilityToolInvocationScope, KernelServiceId, ServiceBusSource,
    ServiceCommand, ServiceCommandName, ToolArtifactOpenCommand, ToolCatalogPlanCommand,
    ToolExecutorRouteKind, ToolGenericTraceCommand, ToolInvokeCommand, ToolProviderHealthResult,
    ToolResultClass, ToolResultGetCommand, ToolsetRef, TraceContext, TOOL_ARTIFACT_OPEN_COMMAND,
    TOOL_AUDIT_QUERY_COMMAND, TOOL_CATALOG_PLAN_COMMAND, TOOL_INVOKE_COMMAND,
    TOOL_PROVIDER_HEALTH_COMMAND, TOOL_RESULT_GET_COMMAND, TOOL_SERVICE_ID,
};
use macaca_runtime_host::{
    bootstrap_tool_planning_service, industrial_tool_planning_service, ServiceRuntime,
    ServiceRuntimeConfig,
};
use serde_json::json;

#[tokio::test]
async fn industrial_tool_system_plans_invokes_artifacts_and_audit_replay() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(industrial_tool_planning_service().unwrap()),
        "trace-proof-bootstrap",
    )
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
    assert!(plan.visible.iter().all(|entry| {
        !entry
            .descriptor
            .executor_route
            .service_id
            .starts_with("service.tool.family.")
    }));
    assert!(plan.visible.iter().any(|entry| {
        entry.descriptor.executor_route.route_kind == ToolExecutorRouteKind::RuntimeEnvironment
    }));
    assert!(plan.visible.iter().any(|entry| {
        entry.descriptor.executor_route.route_kind == ToolExecutorRouteKind::ManagedGateway
    }));

    let health = call_tool::<_, ToolProviderHealthResult>(
        runtime.clone(),
        TOOL_PROVIDER_HEALTH_COMMAND,
        ToolGenericTraceCommand::new(TraceContext::new("trace-industrial-health")).unwrap(),
    )
    .await;
    assert_eq!(health.provider_count, 1);

    let descriptor = plan
        .visible
        .iter()
        .find(|entry| entry.descriptor.family.as_str() == "document")
        .map(|entry| entry.descriptor.clone())
        .unwrap();
    assert_eq!(
        descriptor.executor_route.route_kind,
        ToolExecutorRouteKind::RuntimeEnvironment
    );
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
