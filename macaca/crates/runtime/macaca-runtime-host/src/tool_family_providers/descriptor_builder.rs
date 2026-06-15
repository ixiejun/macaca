//! Builder for industrial tool-family descriptors (Builder + Specification helpers).
//!
//! Translates catalog rows into `IndustrialToolDescriptor` values consumed by planning and
//! invocation.  Sanitized metadata, availability expressions, and route commands are derived
//! here so downstream layers never need provider-specific branching.

use macaca_proto::{
    AvailabilityExpression, CapabilityToolDescriptor, CapabilityToolOriginKind,
    IndustrialToolDescriptor, MacacaResult, ToolExecutorRouteKind, ToolLifecycleScope, ToolsetRef,
};

use super::family_catalog::FamilySpec;

/// Materialize one industrial descriptor from a catalog row.
///
/// Emits `debug!` trace evidence per family so audits can correlate descriptor output with
/// catalog input without logging raw provider payloads.
pub(crate) fn spec_to_descriptor(spec: FamilySpec) -> MacacaResult<IndustrialToolDescriptor> {
    tracing::debug!(
        family = spec.family,
        owner_service = %spec.owner_service,
        provider_path = spec.provider_path,
        "building industrial tool family descriptor"
    );
    let mut base = CapabilityToolDescriptor::new(
        spec.owner_service.clone(),
        spec.provider_id.clone(),
        spec.capability_id.clone(),
        spec.tool_name.clone(),
        format!("Application-neutral {} family tool provider", spec.family),
        serde_json::json!({
            "type": "object",
            "additionalProperties": true,
        }),
        origin_kind_for_route(&spec.route_kind),
    )?;
    base.metadata
        .insert("tool.family".into(), spec.family.into());
    base.metadata.insert(
        "tool.stable_id".into(),
        format!("tool.family.{}.default", spec.family),
    );
    base.metadata.insert(
        "tool.toolsets".into(),
        toolsets_for_family(spec.family).join(","),
    );
    base.metadata
        .insert("provider_path".into(), spec.provider_path.into());
    base.metadata
        .insert("owner_service".into(), spec.owner_service.clone());

    let mut descriptor = IndustrialToolDescriptor::new(
        format!("tool.family.{}.default", spec.family),
        spec.tool_name.clone(),
        format!("{} family provider", spec.family),
        macaca_proto::ToolFamilyRef::new(spec.family)?,
        base,
    )?;
    descriptor.executor_route = descriptor
        .executor_route
        .with_route_kind(spec.route_kind.clone(), command_name_for(&spec));
    descriptor.toolsets = toolsets_for_family(spec.family)
        .into_iter()
        .map(ToolsetRef::new)
        .collect::<MacacaResult<Vec<_>>>()?;
    descriptor.lifecycle_scope = ToolLifecycleScope::Session;
    descriptor.side_effect_class = spec.side_effect_class.clone();
    descriptor.result_class = spec.result_class.clone();
    descriptor.artifact_policy = spec.artifact_policy.clone();
    descriptor.trust_level = spec.trust_level.clone();
    descriptor.availability = availability_for(&spec);
    descriptor
        .sanitized_metadata
        .insert("provider_path".into(), spec.provider_path.into());
    descriptor
        .sanitized_metadata
        .insert("owner_service".into(), spec.owner_service.clone());
    descriptor
        .sanitized_metadata
        .insert("availability_state".into(), spec.availability_state());
    descriptor
        .sanitized_metadata
        .insert("route_kind".into(), format!("{:?}", spec.route_kind));
    Ok(descriptor)
}

/// Map executor route kind to the bounded descriptor origin required by shared DTOs.
fn origin_kind_for_route(route_kind: &ToolExecutorRouteKind) -> CapabilityToolOriginKind {
    match route_kind {
        ToolExecutorRouteKind::Driver => CapabilityToolOriginKind::Driver,
        ToolExecutorRouteKind::Skill => CapabilityToolOriginKind::Skill,
        ToolExecutorRouteKind::Mcp => CapabilityToolOriginKind::Mcp,
        // Route kinds outside the Driver/Skill/MCP descriptor-origin surface use the MCP
        // origin as a neutral catalog bucket. The typed executor route is
        // authoritative for dispatch.
        ToolExecutorRouteKind::OwningServiceCommand
        | ToolExecutorRouteKind::RuntimeEnvironment
        | ToolExecutorRouteKind::ManagedGateway
        | ToolExecutorRouteKind::Plugin
        | ToolExecutorRouteKind::Unavailable => CapabilityToolOriginKind::Mcp,
    }
}

/// Resolve the service.command name used when routing OwningServiceCommand families.
fn command_name_for(spec: &FamilySpec) -> Option<String> {
    match spec.route_kind {
        ToolExecutorRouteKind::RuntimeEnvironment => Some("tool.runtime_environment.invoke".into()),
        ToolExecutorRouteKind::ManagedGateway => Some("tool.managed_gateway.invoke".into()),
        ToolExecutorRouteKind::Plugin => Some("tool.plugin.invoke".into()),
        ToolExecutorRouteKind::OwningServiceCommand => match spec.family {
            "file" => Some(macaca_proto::workbench::file::TOOL_INVOKE_COMMAND.into()),
            "code_intelligence" => {
                Some(macaca_proto::workbench::code_intelligence::SEARCH_COMMAND.into())
            }
            "git" => Some(macaca_proto::workbench::git::STATUS_COMMAND.into()),
            "review" => Some(macaca_proto::workbench::review::START_COMMAND.into()),
            "sandbox" => Some(macaca_proto::workbench::sandbox::TOOL_INVOKE_COMMAND.into()),
            "shell" | "code_execution" => {
                Some(macaca_proto::workbench::process::TOOL_INVOKE_COMMAND.into())
            }
            "approval" => Some(macaca_proto::workbench::approval::POLICY_EXPLAIN_COMMAND.into()),
            "hook" => Some(macaca_proto::workbench::hook::CATALOG_LIST_COMMAND.into()),
            "config" => Some(macaca_proto::workbench::config::FEATURE_LIST_COMMAND.into()),
            "plugin_marketplace" => {
                Some(macaca_proto::workbench::plugin_marketplace::PLUGIN_LIST_COMMAND.into())
            }
            "diagnostics" => {
                Some(macaca_proto::workbench::diagnostics::HEALTH_SUMMARY_COMMAND.into())
            }
            "realtime" => Some(macaca_proto::workbench::realtime::HEALTH_COMMAND.into()),
            "remote_environment" => {
                Some(macaca_proto::workbench::remote_environment::HEALTH_COMMAND.into())
            }
            _ => Some(format!("{}.tool.invoke", spec.family)),
        },
        _ => None,
    }
}

/// Build availability expressions that gate planning without hiding diagnostic catalog rows.
fn availability_for(spec: &FamilySpec) -> Vec<AvailabilityExpression> {
    // Unsupported-platform expressions are a Null Object diagnostic: the family remains
    // visible in hidden catalog output while service.tool will not route invocations until
    // an owning provider registers a healthy route.
    if spec.unsupported_platform {
        vec![AvailabilityExpression::Platform {
            os: "provider_registered_platform".into(),
        }]
    } else {
        vec![AvailabilityExpression::ServiceHealth {
            service_id: spec.owner_service.clone().into(),
        }]
    }
}

/// Resolve toolset membership for a family across industrial planning bundles.
fn toolsets_for_family(family: &str) -> Vec<&'static str> {
    let mut toolsets = vec!["industrial.full_stack"];
    match family {
        "web" | "file" | "sandbox" | "shell" | "approval" | "hook" | "config"
        | "plugin_marketplace" | "memory" | "document" | "scheduler" | "git" | "review"
        | "diagnostics" => {
            toolsets.push("industrial.proof");
        }
        _ => {}
    }
    match family {
        "web" | "browser" | "memory" | "knowledge" | "document" | "code_intelligence" => {
            toolsets.push("industrial.research");
        }
        _ => {}
    }
    match family {
        "task" | "scheduler" | "skill" | "mcp" | "computer_use" | "review" | "approval"
        | "hook" | "config" | "plugin_marketplace" | "diagnostics" | "realtime"
        | "remote_environment" => {
            toolsets.push("industrial.automation");
        }
        _ => {}
    }
    match family {
        "file" | "sandbox" | "shell" | "approval" | "hook" | "config" | "plugin_marketplace"
        | "mcp" | "skill" | "code_intelligence" | "git" | "review" | "diagnostics" | "realtime"
        | "remote_environment" => {
            toolsets.push("industrial.workbench");
        }
        _ => {}
    }
    toolsets
}
