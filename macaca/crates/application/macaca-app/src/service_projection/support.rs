//! Shared projection helpers (digest, diagnostics, agent naming).
//!
//! Pure functions with no side effects except structured `tracing` on UI paths.
//! Keeps cross-cutting utilities out of public projection entry points.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::Path;

use macaca_proto::{
    ApplicationAbilityDescriptor, ApplicationEntryMetadataView, ApplicationId,
    ApplicationManifestDigestView, ApplicationManifestV1, ApplicationServiceAgentView,
    PackageRuntimeKind,
};

use crate::consumption::app_entry_agent_name;
use crate::manifest_v1::LegacyAppManifestProjection;
use crate::model::{AgentSource, AppManifest};
use crate::service_capability::EffectiveServiceCapabilities;
use crate::ApplicationRuntimeKindSpec;

pub(super) fn digest_view(
    legacy: &AppManifest,
    projection: &LegacyAppManifestProjection,
) -> ApplicationManifestDigestView {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    legacy.id.hash(&mut hasher);
    legacy.name.hash(&mut hasher);
    legacy.version.hash(&mut hasher);
    legacy.agents.len().hash(&mut hasher);
    projection.manifest.abilities.len().hash(&mut hasher);
    for ability in &projection.manifest.abilities {
        ability.id.hash(&mut hasher);
    }
    ApplicationManifestDigestView {
        algorithm: "std-default-hasher-v1".into(),
        digest: format!("{:016x}", hasher.finish()),
        source_format: projection
            .manifest
            .metadata
            .get("source.format")
            .cloned()
            .unwrap_or_else(|| "unknown".into()),
        ability_count: projection.manifest.abilities.len(),
        agent_count: legacy.agents.len(),
    }
}

pub(super) fn runtime_diagnostics(
    legacy: &AppManifest,
    runtime_kind: &PackageRuntimeKind,
    capabilities: &crate::service_capability::EffectiveServiceCapabilities,
) -> Vec<String> {
    // Diagnostics are intentionally bounded plain strings. They should help
    // operators debug admission/runtime readiness without leaking payload data.
    let mut diagnostics = vec![format!(
        "effective_service_capabilities_hash={}",
        capabilities.capabilities_hash
    )];
    diagnostics.push(format!(
        "effective_service_count={}",
        capabilities.services.len()
    ));
    if !capabilities.unresolved_packs.is_empty() {
        diagnostics.push(format!(
            "unresolved_domain_packs={}",
            capabilities.unresolved_packs.join(",")
        ));
    }
    let spec = ApplicationRuntimeKindSpec;
    if spec.execution_available_for_runtime(Some(runtime_kind)) {
        diagnostics
    } else {
        diagnostics.push(format!("runtime unavailable for {:?}", legacy.layer));
        diagnostics
    }
}

pub(super) fn skills_dir(app_dir: Option<&Path>, legacy: &AppManifest) -> Option<String> {
    if let Some(resources) = &legacy.resources {
        if let Some(skills) = &resources.skills {
            return Some(
                app_dir
                    .map(|dir| dir.join(skills).display().to_string())
                    .unwrap_or_else(|| skills.clone()),
            );
        }
    }
    app_dir.map(|path| path.join("skills").display().to_string())
}

pub(super) fn agent_name(agent: &AgentSource) -> String {
    match agent {
        AgentSource::Inline(inline) => inline.name.clone(),
        AgentSource::FilePath(path) => path.clone(),
    }
}

pub(super) fn sanitize_heartbeat_agent_metadata(
    metadata: &std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, String> {
    metadata
        .iter()
        .filter_map(|(key, value)| {
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || value.is_empty() {
                return None;
            }
            Some((
                key.chars().take(64).collect(),
                value.chars().take(256).collect(),
            ))
        })
        .collect()
}

/// Resolve the manifest-declared agent name for one ability activation block.
pub(super) fn ability_agent_name(ability: &ApplicationAbilityDescriptor) -> Option<String> {
    ability
        .activation
        .iter()
        .find(|activation| activation.mode == "agent")
        .and_then(|activation| activation.entry.clone())
}

/// Merge legacy inline/file agents with Manifest v1 ability capability names.
pub(super) fn agent_views(
    manifest_v1: &ApplicationManifestV1,
    legacy: &AppManifest,
) -> Vec<ApplicationServiceAgentView> {
    let mut views: Vec<_> = legacy
        .agents
        .iter()
        .map(|agent| ApplicationServiceAgentView {
            name: agent_name(agent),
            capability_names: Vec::new(),
        })
        .collect();
    for ability in manifest_v1.abilities.iter() {
        if let Some(name) = ability_agent_name(ability) {
            let capability_names = ability
                .capabilities
                .iter()
                .map(|capability| capability.id.as_str().to_string())
                .collect();
            if let Some(existing) = views.iter_mut().find(|view| view.name == name) {
                existing.capability_names = capability_names;
            } else {
                views.push(ApplicationServiceAgentView {
                    name,
                    capability_names,
                });
            }
        }
    }
    views.sort_by(|left, right| left.name.cmp(&right.name));
    views.dedup_by(|left, right| left.name == right.name);
    views
}

/// Project runtime entry metadata without copying prompt or secret payloads.
pub(super) fn entry_view(
    manifest_v1: &ApplicationManifestV1,
    legacy: &AppManifest,
) -> ApplicationEntryMetadataView {
    ApplicationEntryMetadataView {
        agent_name: app_entry_agent_name(legacy)
            .map(str::to_string)
            .or_else(|| manifest_v1.runtime.entry.clone()),
        entry_kind: manifest_v1.runtime.metadata.get("entry.kind").cloned(),
        activation_mode: manifest_v1
            .abilities
            .iter()
            .flat_map(|ability| ability.activation.iter())
            .find(|activation| activation.entry == manifest_v1.runtime.entry)
            .map(|activation| activation.mode.clone()),
    }
}
