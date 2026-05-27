//! Contract tests for the generic industrial tool-family catalog.
//!
//! These tests keep the catalog provider-neutral: every required family must be
//! represented by sanitized inventory data, unavailable families must surface as
//! diagnostics rather than callable tools, and availability must be driven by
//! generic service-health signals instead of provider-name branches.

use std::collections::BTreeSet;

use macaca_proto::{
    CapabilityToolOriginKind, ToolCatalogPlanCommand, ToolExecutorRouteKind, ToolFamilyRef,
    ToolHiddenReason, ToolResultClass, ToolsetRef, TraceContext,
};

use crate::{
    industrial_tool_family_provider_contributor, industrial_tool_family_provider_inventory,
    industrial_tool_family_toolsets, AvailabilitySignalSet, ToolDescriptorContributor,
    ToolPlanningService, REQUIRED_INDUSTRIAL_TOOL_FAMILIES,
};

#[tokio::test]
async fn industrial_family_inventory_covers_required_families_without_raw_metadata() {
    let inventory = industrial_tool_family_provider_inventory().unwrap();
    let actual: BTreeSet<_> = inventory
        .iter()
        .map(|entry| entry.family.as_str())
        .collect();
    let expected: BTreeSet<_> = REQUIRED_INDUSTRIAL_TOOL_FAMILIES.iter().copied().collect();

    assert_eq!(actual, expected);
    for entry in inventory {
        assert!(!entry.owner_service.as_str().trim().is_empty());
        assert!(
            !entry.owner_service.starts_with("service.tool.family."),
            "family {} still uses synthetic service.tool.family owner",
            entry.family
        );
        assert!(!entry.provider_id.trim().is_empty());
        assert!(!entry.capability_id.trim().is_empty());
        assert_ne!(entry.provider_path, "application_specific");
        assert!(entry
            .sanitized_metadata
            .keys()
            .all(|key| !key.contains("secret") && !key.contains("credential")));
        assert!(entry
            .sanitized_metadata
            .values()
            .all(|value| !value.contains("RAW_PROVIDER_PAYLOAD")));
    }
}

#[tokio::test]
async fn family_descriptors_use_typed_routes_instead_of_all_mcp_origins() {
    let descriptors = industrial_tool_family_provider_contributor()
        .unwrap()
        .descriptors()
        .await
        .unwrap();
    let route_kinds: Vec<_> = descriptors
        .iter()
        .map(|descriptor| descriptor.executor_route.route_kind.clone())
        .collect();
    let origin_kinds: Vec<_> = descriptors
        .iter()
        .map(|descriptor| descriptor.base_descriptor.origin_kind.clone())
        .collect();

    assert!(route_kinds.contains(&ToolExecutorRouteKind::RuntimeEnvironment));
    assert!(route_kinds.contains(&ToolExecutorRouteKind::ManagedGateway));
    assert!(route_kinds.contains(&ToolExecutorRouteKind::OwningServiceCommand));
    assert!(route_kinds.contains(&ToolExecutorRouteKind::Skill));
    assert!(route_kinds.contains(&ToolExecutorRouteKind::Mcp));
    assert!(route_kinds.contains(&ToolExecutorRouteKind::Plugin));
    assert!(origin_kinds.contains(&CapabilityToolOriginKind::Skill));
    assert!(origin_kinds.contains(&CapabilityToolOriginKind::Mcp));
    assert!(descriptors.iter().all(|descriptor| !descriptor
        .executor_route
        .service_id
        .starts_with("service.tool.family.")));
}

#[tokio::test]
async fn planner_surfaces_missing_optional_families_as_unavailable_diagnostics() {
    let service = ToolPlanningService::builder()
        .with_contributor(industrial_tool_family_provider_contributor().unwrap())
        .with_toolsets(industrial_tool_family_toolsets().unwrap())
        .build();
    let mut command = ToolCatalogPlanCommand::new(TraceContext::new("trace-family-plan")).unwrap();
    command.requested_toolsets = vec![ToolsetRef::new("industrial.full_stack").unwrap()];
    command.include_hidden = true;

    let result = service.plan(command).await.unwrap();
    let hidden_families: BTreeSet<_> = result
        .hidden
        .iter()
        .map(|entry| entry.descriptor.family.as_str().to_string())
        .collect();

    assert_eq!(
        hidden_families.len(),
        REQUIRED_INDUSTRIAL_TOOL_FAMILIES.len()
    );
    assert_eq!(result.visible.len(), 0);
    assert!(result.hidden.iter().all(|entry| {
        entry.hidden_reason == Some(ToolHiddenReason::ServiceUnavailable)
            || entry.hidden_reason == Some(ToolHiddenReason::UnsupportedPlatform)
    }));
    assert!(result
        .metadata
        .get("tool.reason.service_unavailable")
        .is_some());
}

#[tokio::test]
async fn planner_marks_signal_backed_families_visible_without_provider_name_branches() {
    let service = ToolPlanningService::builder()
        .with_contributor(industrial_tool_family_provider_contributor().unwrap())
        .with_availability(
            AvailabilitySignalSet::default().with_service("service.tool.managed_gateway"),
        )
        .with_toolsets(industrial_tool_family_toolsets().unwrap())
        .build();
    let mut command =
        ToolCatalogPlanCommand::new(TraceContext::new("trace-family-visible")).unwrap();
    command.requested_families = vec![ToolFamilyRef::new("web").unwrap()];
    command.include_hidden = true;

    let result = service.plan(command).await.unwrap();

    assert_eq!(result.visible.len(), 1);
    assert_eq!(result.visible[0].descriptor.family.as_str(), "web");
    assert_eq!(
        result.visible[0]
            .descriptor
            .sanitized_metadata
            .get("provider_path")
            .map(String::as_str),
        Some("gateway")
    );
    assert_eq!(
        result.visible[0].descriptor.result_class,
        ToolResultClass::StructuredJson
    );
    assert_eq!(result.hidden.len(), 0);
}
