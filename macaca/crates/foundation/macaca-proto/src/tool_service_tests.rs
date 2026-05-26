use std::collections::BTreeMap;

use crate::{
    tool_service_descriptor, AvailabilityExpression, CapabilityToolDescriptor,
    CapabilityToolOriginKind, IndustrialToolDescriptor, MacacaResult, ServiceHealth,
    ToolArtifactPolicy, ToolAuditRef, ToolCatalogPlanCommand, ToolCatalogPlanResult, ToolFamilyRef,
    ToolHiddenReason, ToolPlanEntry, ToolResultClass, ToolServiceHealthResult,
    ToolServiceSnapshotResult, ToolSideEffectClass, ToolTrustLevel, ToolsetRef, TraceContext,
    TOOL_CATALOG_PLAN_COMMAND, TOOL_SERVICE_ID,
};

fn descriptor() -> MacacaResult<IndustrialToolDescriptor> {
    let base = CapabilityToolDescriptor::new(
        "service.mcp",
        "provider.runtime",
        "capability.web.lookup",
        "web_lookup",
        "Lookup public information",
        serde_json::json!({"type": "object"}),
        CapabilityToolOriginKind::Mcp,
    )?;

    IndustrialToolDescriptor::new(
        "tool.web.lookup",
        "web_lookup",
        "Web lookup",
        ToolFamilyRef::new("web")?,
        base,
    )
}

#[test]
fn industrial_tool_descriptor_round_trips_without_secret_fields() {
    let mut descriptor = descriptor().unwrap();
    descriptor
        .toolsets
        .push(ToolsetRef::new("research").unwrap());
    descriptor
        .availability
        .push(AvailabilityExpression::Secret {
            key_ref: "search-api-key".into(),
        });
    descriptor.side_effect_class = ToolSideEffectClass::ReadOnly;
    descriptor.result_class = ToolResultClass::StructuredJson;
    descriptor.artifact_policy = ToolArtifactPolicy::PersistOversized;
    descriptor.trust_level = ToolTrustLevel::BuiltIn;
    descriptor
        .telemetry_labels
        .insert("family".into(), "web".into());
    descriptor
        .sanitized_metadata
        .insert("docs_ref".into(), "tool-docs/web".into());

    let encoded = serde_json::to_value(&descriptor).unwrap();

    assert_eq!(encoded["stable_tool_id"], "tool.web.lookup");
    assert_eq!(encoded["executor_route"]["service_id"], "service.mcp");
    assert!(encoded.get("raw_secret").is_none());
    assert!(encoded.get("credentials").is_none());
    assert!(encoded.get("headers").is_none());

    let decoded: IndustrialToolDescriptor = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.family.as_str(), "web");
    assert_eq!(decoded.toolsets[0].as_str(), "research");
}

#[test]
fn tool_plan_preserves_visible_and_hidden_diagnostics() {
    let descriptor = descriptor().unwrap();
    let visible = ToolPlanEntry::visible(descriptor.clone(), vec!["selected".into()]);
    let hidden = ToolPlanEntry::hidden(
        descriptor,
        ToolHiddenReason::MissingSecret,
        vec!["search-api-key".into()],
    );

    let result = ToolCatalogPlanResult {
        trace: TraceContext::new("trace-tool-plan"),
        visible: vec![visible],
        hidden: vec![hidden],
        conflicts: Vec::new(),
        audit_refs: vec![ToolAuditRef::new("audit.plan.1")],
        captured_at: chrono::Utc::now(),
        metadata: BTreeMap::new(),
    };

    let encoded = serde_json::to_string(&result).unwrap();
    assert!(encoded.contains("missing_secret"));
    assert!(encoded.contains("tool.web.lookup"));

    let decoded: ToolCatalogPlanResult = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.visible.len(), 1);
    assert_eq!(decoded.hidden.len(), 1);
    assert_eq!(
        decoded.hidden[0].hidden_reason,
        Some(ToolHiddenReason::MissingSecret)
    );
}

#[test]
fn service_tool_commands_and_descriptor_are_stable() {
    let command = ToolCatalogPlanCommand {
        trace: TraceContext::new("trace-tool-catalog"),
        application_id: None,
        agent_name: Some("coordinator".into()),
        requested_toolsets: vec![ToolsetRef::new("research").unwrap()],
        requested_families: vec![ToolFamilyRef::new("web").unwrap()],
        allowed_tools: Vec::new(),
        denied_families: Vec::new(),
        include_hidden: true,
        metadata: BTreeMap::new(),
    };

    let command_json = serde_json::to_value(&command).unwrap();
    assert_eq!(TOOL_SERVICE_ID, "service.tool");
    assert_eq!(TOOL_CATALOG_PLAN_COMMAND, "tool.catalog.plan");
    assert_eq!(command_json["requested_families"][0], "web");
    assert_eq!(tool_service_descriptor().id.as_str(), "service.tool");

    let snapshot = ToolServiceSnapshotResult::unavailable(
        TraceContext::new("trace-tool-snapshot"),
        "tool service is not installed",
    );
    let health = ToolServiceHealthResult {
        trace: TraceContext::new("trace-tool-health"),
        health: ServiceHealth::Unavailable {
            reason: "tool service is not installed".into(),
        },
        provider_count: 0,
        captured_at: chrono::Utc::now(),
        metadata: BTreeMap::new(),
    };

    assert!(matches!(snapshot.health, ServiceHealth::Unavailable { .. }));
    assert_eq!(health.provider_count, 0);
}
