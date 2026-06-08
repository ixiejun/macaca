//! Policy and workbench metadata sub-projectors (Specification + Composite).
//!
//! Each builder extracts bounded policy presence flags and declared names from
//! Manifest v1 abilities without copying prompt bodies or secret-like values.

use std::collections::BTreeSet;

use macaca_proto::{
    ApplicationAbilityDescriptor, ApplicationAbilityMetadataView,
    ApplicationContextPolicyMetadataView, ApplicationManifestV1,
    ApplicationMcpOverlayMetadataView, ApplicationSkillPolicyMetadataView,
    ApplicationToolPolicyMetadataView, ApplicationWorkbenchMetadataView,
};

use crate::model::{AgentSource, AppManifest};

use super::support::{ability_agent_name, agent_name};

pub(super) fn ability_view(ability: &ApplicationAbilityDescriptor) -> ApplicationAbilityMetadataView {
    ApplicationAbilityMetadataView {
        id: ability.id.clone(),
        name: ability_agent_name(ability).unwrap_or_else(|| ability.id.clone()),
        kind: format!("{:?}", ability.kind).to_lowercase(),
        implementation: format!("{:?}", ability.implementation).to_lowercase(),
        is_entry: ability
            .metadata
            .get("entry")
            .is_some_and(|value| value == "true"),
        activation_modes: ability
            .activation
            .iter()
            .map(|activation| activation.mode.clone())
            .collect(),
        capability_names: ability
            .capabilities
            .iter()
            .map(|capability| capability.id.as_str().to_string())
            .collect(),
        required_services: ability
            .services
            .iter()
            .filter(|service| !service.optional)
            .map(|service| service.service.as_str().to_string())
            .collect(),
        permission_names: ability
            .permissions
            .iter()
            .filter(|permission| !permission.optional)
            .map(|permission| permission.name.clone())
            .collect(),
    }
}

pub(super) fn tool_policy_view(manifest_v1: &ApplicationManifestV1) -> ApplicationToolPolicyMetadataView {
    let mut names = BTreeSet::new();
    for ability in &manifest_v1.abilities {
        for capability in &ability.capabilities {
            if let Some(name) = capability.id.as_str().strip_prefix("tool.") {
                names.insert(name.to_string());
            }
        }
    }
    let execution_tool_count = names
        .iter()
        .filter(|name| name.ends_with("_execute"))
        .count();
    ApplicationToolPolicyMetadataView {
        declared_tool_names: names.into_iter().collect(),
        execution_tool_count,
    }
}

pub(super) fn context_policy_view(
    legacy: &AppManifest,
    manifest_v1: &ApplicationManifestV1,
) -> ApplicationContextPolicyMetadataView {
    ApplicationContextPolicyMetadataView {
        context_config_present: legacy.context.is_some(),
        context_engine_declared: manifest_v1.abilities.iter().any(|ability| {
            ability
                .metadata
                .get("context.engine.present")
                .is_some_and(|value| value == "true")
        }),
    }
}

pub(super) fn skill_policy_view(legacy: &AppManifest) -> ApplicationSkillPolicyMetadataView {
    let mut names: Vec<_> = legacy
        .agents
        .iter()
        .filter_map(|source| match source {
            AgentSource::Inline(inline) if inline.skills.is_some() => Some(inline.name.clone()),
            _ => None,
        })
        .collect();
    names.sort();
    names.dedup();
    ApplicationSkillPolicyMetadataView {
        agents_with_skill_policy: names,
    }
}

pub(super) fn mcp_overlay_view(legacy: &AppManifest) -> ApplicationMcpOverlayMetadataView {
    let agents = skill_policy_view(legacy).agents_with_skill_policy;
    ApplicationMcpOverlayMetadataView {
        overlay_declared: !agents.is_empty(),
        agents_with_overlay: agents,
    }
}

pub(super) fn workbench_metadata_view(
    manifest_v1: &ApplicationManifestV1,
) -> ApplicationWorkbenchMetadataView {
    let Some(workbench) = &manifest_v1.workbench else {
        return ApplicationWorkbenchMetadataView::default();
    };
    ApplicationWorkbenchMetadataView {
        declared_capabilities: workbench
            .capabilities
            .iter()
            .map(|capability| capability.family.clone())
            .collect(),
        permission_profiles: workbench.permission_profiles.clone(),
        tool_families: workbench.tool_families.clone(),
        service_dependencies: workbench
            .service_dependencies
            .iter()
            .map(|dependency| dependency.service.as_str().to_string())
            .collect(),
        optional_provider_requirements: workbench
            .optional_provider_requirements
            .iter()
            .map(|provider| provider.provider_kind.clone())
            .collect(),
        plugin_dependencies: workbench
            .plugin_dependencies
            .iter()
            .map(|dependency| dependency.plugin_id.clone())
            .collect(),
        mcp_dependencies: workbench
            .mcp_dependencies
            .iter()
            .map(|dependency| dependency.server_id.clone())
            .collect(),
        skill_bundles: workbench
            .skill_bundles
            .iter()
            .map(|bundle| bundle.bundle_id.clone())
            .collect(),
        event_subscriptions: workbench
            .event_subscriptions
            .iter()
            .map(|subscription| subscription.topic.clone())
            .collect(),
        ui_surfaces: workbench
            .ui_surfaces
            .iter()
            .map(|surface| surface.surface_id.clone())
            .collect(),
    }
}
