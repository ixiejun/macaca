use std::sync::Arc;

use macaca_proto::{
    ApplicationId, CapabilityToolDescriptor, CapabilityToolInvocationScope,
    CapabilityToolOriginKind, IndustrialToolDescriptor, KernelServiceId, ServiceBusSource,
    ServiceCommand, ServiceCommandName, ServiceHealth, ToolAuditQueryCommand,
    ToolCatalogPlanCommand, ToolCommandResult, ToolFamilyRef, ToolHiddenReason, ToolInvokeCommand,
    ToolResultClass, TraceContext, TOOL_AUDIT_QUERY_COMMAND, TOOL_CATALOG_PLAN_COMMAND,
    TOOL_INVOKE_COMMAND,
};
use serde_json::json;

use crate::{
    bootstrap_tool_planning_service, ServiceRuntime, ServiceRuntimeConfig,
    StaticToolDescriptorContributor, ToolPlanningService,
};

#[tokio::test]
async fn tool_service_audit_replays_sanitized_plan_memento() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let planner = ToolPlanningService::builder()
        .with_contributor(StaticToolDescriptorContributor::new(
            "audit-contributor",
            ServiceHealth::Healthy,
            vec![descriptor("tool.audit.visible", "visible_tool")],
        ))
        .build();
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(planner),
        "trace-bootstrap-tool-audit-plan",
    )
    .await
    .unwrap();

    let mut command =
        ToolCatalogPlanCommand::new(TraceContext::new("trace-tool-audit-plan")).unwrap();
    command.allowed_tools = vec!["other_tool".into()];
    command.include_hidden = true;
    let plan: macaca_proto::ToolCatalogPlanResult =
        call_tool_service(runtime.clone(), TOOL_CATALOG_PLAN_COMMAND, command).await;
    assert_eq!(
        plan.hidden[0].hidden_reason,
        Some(ToolHiddenReason::PolicyDenied)
    );

    let audit: ToolCommandResult = call_tool_service(
        runtime,
        TOOL_AUDIT_QUERY_COMMAND,
        ToolAuditQueryCommand::new(TraceContext::new("trace-tool-audit-query")).unwrap(),
    )
    .await;

    assert_eq!(audit.status, "ok");
    assert_eq!(audit.result_class, ToolResultClass::StructuredJson);
    let inline = audit.inline_output.expect("audit inline output");
    assert_eq!(inline["plan"]["trace_id"], "trace-tool-audit-plan");
    assert_eq!(inline["plan"]["visible_count"], 0);
    assert_eq!(inline["plan"]["hidden_count"], 1);
    assert_eq!(inline["plan"]["reason_counts"]["policy_denied"], 1);
    assert_eq!(inline["events"][0]["event"], "tool.planning.started");
    let encoded = inline.to_string();
    assert!(!encoded.contains("raw_provider_payload"));
    assert!(!encoded.contains("credentials"));
}

#[tokio::test]
async fn tool_service_audit_replays_denied_invocation_without_raw_input() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    bootstrap_tool_planning_service(
        runtime.clone(),
        Arc::new(ToolPlanningService::builder().build()),
        "trace-bootstrap-tool-audit-invoke",
    )
    .await
    .unwrap();

    let mut command = ToolInvokeCommand {
        trace: TraceContext::new("trace-tool-audit-invoke"),
        scope: CapabilityToolInvocationScope::new(ApplicationId::new(), "session-a", "agent-a")
            .unwrap(),
        tool_id: "tool.audit.denied".into(),
        descriptor: Some(descriptor("tool.audit.denied", "denied_tool")),
        input: json!({"secret": "do-not-leak", "value": 42}),
        policy_ref: None,
        approval_ref: None,
        metadata: Default::default(),
    };
    command
        .metadata
        .insert("policy.decision".into(), "deny".into());
    let denied: ToolCommandResult =
        call_tool_service(runtime.clone(), TOOL_INVOKE_COMMAND, command).await;
    assert_eq!(denied.status, "denied");

    let audit: ToolCommandResult = call_tool_service(
        runtime,
        TOOL_AUDIT_QUERY_COMMAND,
        ToolAuditQueryCommand::new(TraceContext::new("trace-tool-audit-query-2")).unwrap(),
    )
    .await;

    let inline = audit.inline_output.expect("audit inline output");
    assert_eq!(
        inline["invocations"][0]["trace_id"],
        "trace-tool-audit-invoke"
    );
    assert_eq!(inline["invocations"][0]["status"], "denied");
    assert!(inline["invocations"][0]["input_hash"].as_str().is_some());
    assert!(inline["invocations"][0].get("raw_input").is_none());
    assert!(!inline.to_string().contains("do-not-leak"));
}

async fn call_tool_service<T, R>(runtime: Arc<ServiceRuntime>, command_name: &str, payload: T) -> R
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
            &KernelServiceId::new(macaca_proto::TOOL_SERVICE_ID),
            ServiceBusSource::new("tool-audit-test"),
            command,
        )
        .await
        .unwrap();
    serde_json::from_value(reply.output.unwrap()).unwrap()
}

fn descriptor(id: &str, name: &str) -> IndustrialToolDescriptor {
    let base = CapabilityToolDescriptor::new(
        "service.driver",
        "provider.audit",
        "capability.audit",
        name,
        "Audit test tool",
        json!({"type": "object"}),
        CapabilityToolOriginKind::Driver,
    )
    .unwrap();
    IndustrialToolDescriptor::new(
        id,
        name,
        "Audit Test Tool",
        ToolFamilyRef::new("audit").unwrap(),
        base,
    )
    .unwrap()
}
