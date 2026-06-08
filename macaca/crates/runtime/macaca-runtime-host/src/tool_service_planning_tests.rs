use macaca_proto::{
    ApplicationId, AvailabilityExpression, CapabilityToolDescriptor, CapabilityToolOriginKind,
    IndustrialToolDescriptor, KernelServiceId, MacacaResult, ServiceBusSource, ServiceCommand,
    ServiceCommandName, ServiceHealth, ToolCatalogPlanCommand, ToolCatalogPlanResult,
    ToolFamilyRef, ToolGenericTraceCommand, ToolHiddenReason, ToolProviderHealthResult, ToolsetRef,
    TraceContext, TOOL_CATALOG_PLAN_COMMAND, TOOL_PROVIDER_HEALTH_COMMAND,
    TOOL_TOOLSET_RESOLVE_COMMAND,
};
use std::sync::Arc;

use crate::{
    bootstrap_tool_planning_service, AvailabilitySignalSet, CapabilityToolDescriptorContributor,
    ServiceRuntime, ServiceRuntimeConfig, StaticToolDescriptorContributor, ToolPlanningService,
    ToolPlanningToolsetResolver,
};

fn descriptor(
    id: &str,
    name: &str,
    family: &str,
    toolsets: &[&str],
    availability: Vec<AvailabilityExpression>,
) -> MacacaResult<IndustrialToolDescriptor> {
    let base = CapabilityToolDescriptor::new(
        "service.fixture",
        "provider.fixture",
        format!("capability.{family}.{name}"),
        name,
        format!("{family} capability"),
        serde_json::json!({"type": "object"}),
        CapabilityToolOriginKind::Mcp,
    )?;
    let mut descriptor = IndustrialToolDescriptor::new(
        id,
        name,
        format!("{family} {name}"),
        ToolFamilyRef::new(family)?,
        base,
    )?;
    descriptor.toolsets = toolsets
        .iter()
        .map(|toolset| ToolsetRef::new(*toolset))
        .collect::<MacacaResult<Vec<_>>>()?;
    descriptor.availability = availability;
    Ok(descriptor)
}

#[tokio::test]
async fn planning_separates_visible_hidden_policy_and_conflicts() {
    let contributor = StaticToolDescriptorContributor::new(
        "fixture-contributor",
        ServiceHealth::Healthy,
        vec![
            descriptor("tool.web.lookup", "lookup", "web", &["research"], vec![]).unwrap(),
            descriptor(
                "tool.browser.open",
                "open",
                "browser",
                &["research"],
                vec![AvailabilityExpression::Binary {
                    binary: "browser-bin".into(),
                }],
            )
            .unwrap(),
            descriptor(
                "tool.memory.recall",
                "recall",
                "memory",
                &["research"],
                vec![AvailabilityExpression::Auth {
                    provider_ref: "memory-auth".into(),
                }],
            )
            .unwrap(),
            descriptor("tool.chat.send", "send", "communication", &[], vec![]).unwrap(),
            descriptor(
                "tool.web.lookup-alt",
                "lookup",
                "web",
                &["research"],
                vec![],
            )
            .unwrap(),
        ],
    );
    let service = ToolPlanningService::builder()
        .with_contributor(contributor)
        .with_availability(AvailabilitySignalSet::default().with_binary("browser-bin"))
        .with_toolsets(
            ToolPlanningToolsetResolver::default()
                .with_toolset("research", ["web", "browser", "memory", "document"]),
        )
        .build();
    let mut command = ToolCatalogPlanCommand::new(TraceContext::new("trace-plan")).unwrap();
    command.application_id = Some(ApplicationId(uuid::Uuid::new_v4()));
    command.agent_name = Some("fixture-planning-agent".into());
    command.requested_toolsets = vec![ToolsetRef::new("research").unwrap()];
    command.requested_families = vec![ToolFamilyRef::new("communication").unwrap()];
    command.denied_families = vec![ToolFamilyRef::new("communication").unwrap()];
    command.include_hidden = true;

    let result = service.plan(command).await.unwrap();

    assert_eq!(result.visible.len(), 3);
    assert_eq!(result.hidden.len(), 2);
    assert_eq!(result.conflicts.len(), 1);
    assert_eq!(
        result.hidden[0].hidden_reason,
        Some(ToolHiddenReason::MissingAuth)
    );
    assert_eq!(
        result.hidden[1].hidden_reason,
        Some(ToolHiddenReason::PolicyDenied)
    );
    assert!(result
        .metadata
        .get("tool.reason.missing_auth")
        .is_some_and(|count| count == "1"));
}

#[tokio::test]
async fn exact_allowed_tools_preserves_compatibility_filtering() {
    let contributor = StaticToolDescriptorContributor::new(
        "fixture-contributor",
        ServiceHealth::Healthy,
        vec![
            descriptor("tool.file.read", "read_file", "file", &[], vec![]).unwrap(),
            descriptor("tool.file.write", "write_file", "file", &[], vec![]).unwrap(),
        ],
    );
    let service = ToolPlanningService::builder()
        .with_contributor(contributor)
        .build();
    let mut command = ToolCatalogPlanCommand::new(TraceContext::new("trace-allowed")).unwrap();
    command.allowed_tools = vec!["read_file".into()];
    command.include_hidden = true;

    let result = service.plan(command).await.unwrap();

    assert_eq!(result.visible.len(), 1);
    assert_eq!(result.visible[0].descriptor.visible_name, "read_file");
    assert_eq!(result.hidden.len(), 1);
    assert_eq!(
        result.hidden[0].hidden_reason,
        Some(ToolHiddenReason::PolicyDenied)
    );
}

#[tokio::test]
async fn capability_descriptor_contributor_adapts_existing_service_catalogs() {
    let mut base = CapabilityToolDescriptor::new(
        "service.skill",
        "provider.fixture",
        "capability.skill.search",
        "skill_search",
        "Search skill catalog",
        serde_json::json!({"type": "object"}),
        CapabilityToolOriginKind::Skill,
    )
    .unwrap();
    base.metadata
        .insert("tool.family".into(), "knowledge".into());
    base.metadata
        .insert("tool.toolsets".into(), "research,analysis".into());

    let service = ToolPlanningService::builder()
        .with_contributor(CapabilityToolDescriptorContributor::new(
            "skill-contributor",
            ServiceHealth::Healthy,
            vec![base],
        ))
        .with_toolsets(
            ToolPlanningToolsetResolver::default().with_toolset("research", ["knowledge"]),
        )
        .build();
    let mut command = ToolCatalogPlanCommand::new(TraceContext::new("trace-adapter")).unwrap();
    command.requested_toolsets = vec![ToolsetRef::new("research").unwrap()];

    let result = service.plan(command).await.unwrap();

    assert_eq!(result.visible.len(), 1);
    assert_eq!(result.visible[0].descriptor.family.as_str(), "knowledge");
    assert_eq!(result.visible[0].descriptor.toolsets.len(), 2);
    assert_eq!(
        result.visible[0].descriptor.executor_route.service_id,
        "service.skill"
    );
}

#[tokio::test]
async fn availability_evaluator_reports_stable_hidden_reasons() {
    let descriptors = vec![
        descriptor(
            "tool.config",
            "needs_config",
            "web",
            &[],
            vec![AvailabilityExpression::Config { key: "cfg".into() }],
        )
        .unwrap(),
        descriptor(
            "tool.secret",
            "needs_secret",
            "web",
            &[],
            vec![AvailabilityExpression::Secret {
                key_ref: "secret-ref".into(),
            }],
        )
        .unwrap(),
        descriptor(
            "tool.binary",
            "needs_binary",
            "web",
            &[],
            vec![AvailabilityExpression::Binary {
                binary: "missing-bin".into(),
            }],
        )
        .unwrap(),
        descriptor(
            "tool.service",
            "needs_service",
            "web",
            &[],
            vec![AvailabilityExpression::ServiceHealth {
                service_id: "service.missing".into(),
            }],
        )
        .unwrap(),
        descriptor(
            "tool.platform",
            "needs_platform",
            "web",
            &[],
            vec![AvailabilityExpression::Platform { os: "plan9".into() }],
        )
        .unwrap(),
        descriptor(
            "tool.policy",
            "needs_policy",
            "web",
            &[],
            vec![AvailabilityExpression::AgentPolicy {
                policy_ref: "policy.allowed".into(),
            }],
        )
        .unwrap(),
        descriptor(
            "tool.entitlement",
            "needs_entitlement",
            "web",
            &[],
            vec![AvailabilityExpression::Entitlement {
                entitlement: "entitlement.required".into(),
            }],
        )
        .unwrap(),
    ];
    let mut signals = AvailabilitySignalSet::default();
    signals.platform_os = Some("macos".into());
    let service = ToolPlanningService::builder()
        .with_contributor(StaticToolDescriptorContributor::new(
            "fixture-contributor",
            ServiceHealth::Healthy,
            descriptors,
        ))
        .with_availability(signals)
        .build();
    let mut command = ToolCatalogPlanCommand::new(TraceContext::new("trace-reasons")).unwrap();
    command.include_hidden = true;

    let result = service.plan(command).await.unwrap();
    let reasons: Vec<_> = result
        .hidden
        .iter()
        .filter_map(|entry| entry.hidden_reason.clone())
        .collect();

    assert!(reasons.contains(&ToolHiddenReason::MissingConfig));
    assert!(reasons.contains(&ToolHiddenReason::MissingSecret));
    assert!(reasons.contains(&ToolHiddenReason::MissingBinary));
    assert!(reasons.contains(&ToolHiddenReason::ServiceUnavailable));
    assert!(reasons.contains(&ToolHiddenReason::UnsupportedPlatform));
    assert!(reasons.contains(&ToolHiddenReason::PolicyDenied));
    assert!(reasons.contains(&ToolHiddenReason::EntitlementMissing));
}

#[tokio::test]
async fn tool_planning_service_registers_and_plans_through_service_runtime() {
    let service = Arc::new(
        ToolPlanningService::builder()
            .with_contributor(StaticToolDescriptorContributor::new(
                "fixture-contributor",
                ServiceHealth::Healthy,
                vec![descriptor("tool.file.read", "read_file", "file", &[], vec![]).unwrap()],
            ))
            .build(),
    );
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = bootstrap_tool_planning_service(
        Arc::clone(&runtime),
        service,
        "trace-tool-service-bootstrap",
    )
    .await
    .unwrap();

    assert_eq!(service_id, KernelServiceId::new("service.tool"));

    let trace = TraceContext::new("trace-tool-service-plan");
    let command = ToolCatalogPlanCommand::new(trace.clone()).unwrap();
    let reply = runtime
        .call(
            &service_id,
            ServiceBusSource::new("test.tool-planning"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(TOOL_CATALOG_PLAN_COMMAND),
                serde_json::to_value(command).unwrap(),
                trace,
            ),
        )
        .await
        .unwrap();
    let result: ToolCatalogPlanResult =
        serde_json::from_value(reply.output.expect("tool plan reply must carry output")).unwrap();

    assert_eq!(result.visible.len(), 1);
    assert_eq!(result.visible[0].descriptor.visible_name, "read_file");
}

#[tokio::test]
async fn tool_provider_health_reports_registered_contributors_before_first_plan() {
    let service = Arc::new(
        ToolPlanningService::builder()
            .with_contributor(StaticToolDescriptorContributor::new(
                "fixture-contributor",
                ServiceHealth::Healthy,
                vec![descriptor("tool.file.read", "read_file", "file", &[], vec![]).unwrap()],
            ))
            .build(),
    );
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = bootstrap_tool_planning_service(
        Arc::clone(&runtime),
        service,
        "trace-tool-health-bootstrap",
    )
    .await
    .unwrap();
    let trace = TraceContext::new("trace-tool-provider-health");
    let reply = runtime
        .call(
            &service_id,
            ServiceBusSource::new("test.tool-provider-health"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(TOOL_PROVIDER_HEALTH_COMMAND),
                serde_json::to_value(ToolGenericTraceCommand::new(trace.clone()).unwrap()).unwrap(),
                trace,
            ),
        )
        .await
        .unwrap();
    let result: ToolProviderHealthResult = serde_json::from_value(reply.output.unwrap()).unwrap();

    assert_eq!(result.provider_count, 1);
    assert_eq!(result.health, ServiceHealth::Healthy);
}

#[tokio::test]
async fn toolset_resolve_returns_real_plan_instead_of_empty_synthetic_plan() {
    let service = Arc::new(
        ToolPlanningService::builder()
            .with_contributor(StaticToolDescriptorContributor::new(
                "fixture-contributor",
                ServiceHealth::Healthy,
                vec![
                    descriptor("tool.web.lookup", "lookup", "web", &["research"], vec![]).unwrap(),
                ],
            ))
            .with_toolsets(ToolPlanningToolsetResolver::default().with_toolset("research", ["web"]))
            .build(),
    );
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let service_id = bootstrap_tool_planning_service(
        Arc::clone(&runtime),
        service,
        "trace-toolset-resolve-bootstrap",
    )
    .await
    .unwrap();
    let trace = TraceContext::new("trace-toolset-resolve");
    let mut command = ToolGenericTraceCommand::new(trace.clone()).unwrap();
    command
        .metadata
        .insert("toolsets".into(), "research".into());
    let reply = runtime
        .call(
            &service_id,
            ServiceBusSource::new("test.toolset-resolve"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(TOOL_TOOLSET_RESOLVE_COMMAND),
                serde_json::to_value(command).unwrap(),
                trace,
            ),
        )
        .await
        .unwrap();
    let result: ToolCatalogPlanResult = serde_json::from_value(reply.output.unwrap()).unwrap();

    assert_eq!(result.visible.len(), 1);
    assert_eq!(result.visible[0].descriptor.family.as_str(), "web");
}
