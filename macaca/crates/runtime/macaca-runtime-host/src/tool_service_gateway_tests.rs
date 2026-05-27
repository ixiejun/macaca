use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_proto::{
    ServiceHealth, ToolFamilyRef, ToolGatewayAuditEvent, ToolGatewayMeteringEvent,
    ToolManagedGatewayDescriptor, ToolManagedGatewayRouteKind, TraceContext,
};

use crate::{
    StaticToolManagedGatewayProvider, ToolManagedGatewayService,
    UnavailableToolManagedGatewayProvider,
};

#[tokio::test]
async fn tool_service_gateway_reports_unavailable_provider() {
    let provider = Arc::new(UnavailableToolManagedGatewayProvider::new(
        "provider.gateway.unavailable",
        "gateway.media",
        ToolManagedGatewayRouteKind::Media,
        ToolFamilyRef::new("media").unwrap(),
        "managed gateway provider is not configured",
    ));
    let service = ToolManagedGatewayService::new(vec![provider]);

    let result = service
        .health(TraceContext::new("trace-gateway-unavailable"))
        .await
        .unwrap();

    assert_eq!(result.gateways.len(), 1);
    assert!(matches!(
        result.gateways[0].health,
        ServiceHealth::Unavailable { .. }
    ));
    assert_eq!(
        serde_json::to_value(&result.audit_refs[0]).unwrap(),
        "tool.gateway.health"
    );
}

#[tokio::test]
async fn tool_service_gateway_records_metering_and_audit_hooks() {
    let descriptor = ToolManagedGatewayDescriptor {
        gateway_id: "gateway.web".into(),
        provider_id: "provider.gateway".into(),
        route_kind: ToolManagedGatewayRouteKind::Web,
        family: ToolFamilyRef::new("web").unwrap(),
        health: ServiceHealth::Healthy,
        metering_required: true,
        audit_required: true,
        metadata: BTreeMap::new(),
    };
    let provider = Arc::new(StaticToolManagedGatewayProvider::new(
        "provider.gateway",
        vec![descriptor],
    ));
    let service = ToolManagedGatewayService::new(vec![provider.clone()]);

    let metering_ref = service
        .record_metering(ToolGatewayMeteringEvent {
            trace: TraceContext::new("trace-gateway-meter"),
            gateway_id: "gateway.web".into(),
            provider_id: "provider.gateway".into(),
            tool_id: "tool.web.extract".into(),
            metering_ref: "meter.gateway.1".into(),
            units: 3,
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();
    let audit_ref = service
        .record_audit(ToolGatewayAuditEvent {
            trace: TraceContext::new("trace-gateway-audit"),
            gateway_id: "gateway.web".into(),
            provider_id: "provider.gateway".into(),
            tool_id: "tool.web.extract".into(),
            status: "ok".into(),
            latency_millis: 12,
            input_hash: "sha256:input".into(),
            output_hash: "sha256:output".into(),
            artifact_refs: Vec::new(),
            metering_ref: Some("meter.gateway.1".into()),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();

    assert_eq!(
        serde_json::to_value(&metering_ref).unwrap(),
        "tool.gateway.meter.recorded"
    );
    assert_eq!(
        serde_json::to_value(&audit_ref).unwrap(),
        "tool.gateway.audit.recorded"
    );
    assert_eq!(provider.recorded_metering_count(), 1);
    assert_eq!(provider.recorded_audit_count(), 1);
}

#[tokio::test]
async fn tool_service_gateway_missing_provider_is_structured_unavailable() {
    let service = ToolManagedGatewayService::new(Vec::new());

    let audit_ref = service
        .record_audit(ToolGatewayAuditEvent {
            trace: TraceContext::new("trace-gateway-missing"),
            gateway_id: "gateway.web".into(),
            provider_id: "provider.missing".into(),
            tool_id: "tool.web.extract".into(),
            status: "unavailable".into(),
            latency_millis: 0,
            input_hash: "sha256:input".into(),
            output_hash: "sha256:output".into(),
            artifact_refs: Vec::new(),
            metering_ref: None,
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();

    assert_eq!(
        serde_json::to_value(&audit_ref).unwrap(),
        "tool.gateway.audit.not_found"
    );
}
