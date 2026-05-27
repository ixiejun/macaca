use std::collections::BTreeMap;

use crate::{
    ServiceHealth, ToolArtifactRef, ToolAuditRef, ToolEnvironmentArtifactRoot,
    ToolEnvironmentCleanupResult, ToolEnvironmentHealthResult, ToolEnvironmentNetworkPolicy,
    ToolEnvironmentProcessHandle, ToolFamilyRef, ToolGatewayAuditEvent, ToolGatewayHealthResult,
    ToolGatewayMeteringEvent, ToolManagedGatewayDescriptor, ToolManagedGatewayRouteKind,
    ToolRuntimeEnvironmentDescriptor, ToolRuntimeEnvironmentKind, ToolRuntimeEnvironmentScope,
    ToolRuntimeEnvironmentState, TraceContext, TOOL_ENVIRONMENT_CLEANUP_COMMAND,
    TOOL_ENVIRONMENT_HEALTH_COMMAND, TOOL_GATEWAY_AUDIT_COMMAND, TOOL_GATEWAY_HEALTH_COMMAND,
    TOOL_GATEWAY_METER_COMMAND,
};

#[test]
fn runtime_environment_descriptor_serializes_without_secret_values() {
    let mut descriptor = ToolRuntimeEnvironmentDescriptor::unavailable(
        "env.session",
        "provider.runtime",
        ToolRuntimeEnvironmentKind::LocalSandbox,
        "sandbox provider is not configured",
    );
    descriptor.scope = ToolRuntimeEnvironmentScope::Session;
    descriptor.state = ToolRuntimeEnvironmentState::Ready;
    descriptor.health = ServiceHealth::Healthy;
    descriptor.network_policy = ToolEnvironmentNetworkPolicy {
        egress_allowed: true,
        allowed_host_refs: vec!["policy.hosts.public_docs".into()],
        denied_host_refs: Vec::new(),
        metadata: BTreeMap::new(),
    };
    descriptor.artifact_roots.push(ToolEnvironmentArtifactRoot {
        root_ref: ToolArtifactRef("artifact.root.session".into()),
        scope: ToolRuntimeEnvironmentScope::Session,
        metadata: BTreeMap::new(),
    });
    descriptor
        .process_handles
        .push(ToolEnvironmentProcessHandle {
            handle_ref: "process.handle.1".into(),
            state: ToolRuntimeEnvironmentState::Busy,
            pid_hint: Some(42),
            metadata: BTreeMap::new(),
        });

    let encoded = serde_json::to_value(&descriptor).unwrap();

    assert_eq!(encoded["kind"], "local_sandbox");
    assert!(encoded.get("secret_value").is_none());
    assert!(encoded.get("headers").is_none());
    assert!(encoded.get("raw_payload").is_none());

    let decoded: ToolRuntimeEnvironmentDescriptor = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.environment_id, "env.session");
    assert_eq!(decoded.process_handles[0].handle_ref, "process.handle.1");
}

#[test]
fn environment_health_and_cleanup_results_are_auditable() {
    let trace = TraceContext::new("trace-environment-health");
    let descriptor = ToolRuntimeEnvironmentDescriptor::unavailable(
        "env.sandbox",
        "provider.unavailable",
        ToolRuntimeEnvironmentKind::LocalSandbox,
        "sandbox provider is absent",
    );
    let health = ToolEnvironmentHealthResult {
        trace: trace.clone(),
        descriptors: vec![descriptor],
        captured_at: chrono::Utc::now(),
        audit_refs: vec![ToolAuditRef::new("tool.environment.health")],
        metadata: BTreeMap::new(),
    };
    let cleanup = ToolEnvironmentCleanupResult {
        trace,
        environment_id: "env.sandbox".into(),
        status: "released".into(),
        released_process_handles: vec!["process.handle.1".into()],
        released_artifact_roots: vec![ToolArtifactRef("artifact.root.session".into())],
        audit_refs: vec![ToolAuditRef::new("tool.environment.cleanup")],
        captured_at: chrono::Utc::now(),
        metadata: BTreeMap::new(),
    };

    assert_eq!(TOOL_ENVIRONMENT_HEALTH_COMMAND, "tool.environment.health");
    assert_eq!(TOOL_ENVIRONMENT_CLEANUP_COMMAND, "tool.environment.cleanup");
    assert_eq!(health.descriptors.len(), 1);
    assert_eq!(cleanup.released_process_handles[0], "process.handle.1");
}

#[test]
fn managed_gateway_contracts_round_trip_with_metering_and_audit() {
    let trace = TraceContext::new("trace-gateway");
    let gateway = ToolManagedGatewayDescriptor::unavailable(
        "gateway.web",
        "provider.gateway",
        ToolManagedGatewayRouteKind::Web,
        ToolFamilyRef::new("web").unwrap(),
        "managed gateway provider is absent",
    );
    let health = ToolGatewayHealthResult {
        trace: trace.clone(),
        gateways: vec![gateway],
        captured_at: chrono::Utc::now(),
        audit_refs: vec![ToolAuditRef::new("tool.gateway.health")],
        metadata: BTreeMap::new(),
    };
    let meter = ToolGatewayMeteringEvent {
        trace: trace.clone(),
        gateway_id: "gateway.web".into(),
        provider_id: "provider.gateway".into(),
        tool_id: "tool.web.extract".into(),
        metering_ref: "meter.gateway.1".into(),
        units: 1,
        metadata: BTreeMap::new(),
    };
    let audit = ToolGatewayAuditEvent {
        trace,
        gateway_id: "gateway.web".into(),
        provider_id: "provider.gateway".into(),
        tool_id: "tool.web.extract".into(),
        status: "unavailable".into(),
        latency_millis: 0,
        input_hash: "sha256:input".into(),
        output_hash: "sha256:output".into(),
        artifact_refs: Vec::new(),
        metering_ref: Some("meter.gateway.1".into()),
        metadata: BTreeMap::new(),
    };

    let encoded = serde_json::to_string(&audit).unwrap();

    assert_eq!(TOOL_GATEWAY_HEALTH_COMMAND, "tool.gateway.health");
    assert_eq!(TOOL_GATEWAY_METER_COMMAND, "tool.gateway.meter");
    assert_eq!(TOOL_GATEWAY_AUDIT_COMMAND, "tool.gateway.audit");
    assert_eq!(health.gateways[0].family.as_str(), "web");
    assert_eq!(meter.units, 1);
    assert!(!encoded.contains("secret"));
    assert!(!encoded.contains("raw_payload"));
}
