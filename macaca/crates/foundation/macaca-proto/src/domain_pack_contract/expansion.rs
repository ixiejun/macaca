use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use super::catalog::DomainPackCatalog;
use super::model::{AppServiceContractConfig, DomainPackDefinition};
use super::spec::DomainPackDefinitionSpec;

/// Application-facing memento for one declared pack service.
///
/// The projection is intentionally descriptor/effective-capability data only. It lets
/// applications, shells, SDKs, tests, and audit tools inspect callable commands, explicit
/// unavailable reasons, provider capability flags, and replay references without constructing
/// providers, reading credentials, or exposing raw provider payloads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackEffectiveCapabilityProjection {
    pub pack_id: String,
    pub service_id: String,
    pub required: bool,
    pub callable_commands: BTreeSet<String>,
    pub denied_commands: BTreeMap<String, String>,
    pub unavailable_commands: BTreeMap<String, String>,
    pub unavailable_features: BTreeMap<String, String>,
    pub provider_capability_flags: BTreeMap<String, BTreeMap<String, String>>,
    pub replay_refs: BTreeSet<String>,
}

/// Result of deterministic capability expansion from manifest declarations plus catalog data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveServiceCapabilities {
    pub services: BTreeSet<String>,
    pub service_sources: BTreeMap<String, String>,
    pub service_command_schemas: BTreeMap<String, BTreeSet<String>>,
    pub service_result_schemas: BTreeMap<String, BTreeSet<String>>,
    pub resolved_packs: Vec<String>,
    pub required_packs: Vec<String>,
    pub optional_packs: Vec<String>,
    pub unresolved_required_packs: Vec<String>,
    pub unresolved_optional_packs: Vec<String>,
    pub incompatible_packs: Vec<String>,
    pub unresolved_packs: Vec<String>,
    pub unavailable_pack_reasons: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub granted_pack_permission_scopes: BTreeMap<String, BTreeSet<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_projections: Vec<DomainPackEffectiveCapabilityProjection>,
    pub capabilities_hash: String,
}

/// Expand one manifest-level declaration into effective capabilities.
///
/// Required and optional pack declarations are resolved through the same catalog Strategy while
/// preserving their admission semantics.  Explicit service ids are merged afterward so existing
/// manifest contracts keep working during the pack-platform rollout.
pub fn expand_service_capabilities(
    declaration: Option<&AppServiceContractConfig>,
    catalog: &dyn DomainPackCatalog,
) -> EffectiveServiceCapabilities {
    let mut services = BTreeSet::new();
    let mut service_sources = BTreeMap::new();
    let mut service_command_schemas = BTreeMap::new();
    let mut service_result_schemas = BTreeMap::new();
    let mut resolved_packs = Vec::new();
    let mut required_packs = Vec::new();
    let mut optional_packs = Vec::new();
    let mut unresolved_required_packs = Vec::new();
    let mut unresolved_optional_packs = Vec::new();
    let mut incompatible_packs = Vec::new();
    let mut unresolved_packs = Vec::new();
    let mut unavailable_pack_reasons = BTreeMap::new();
    let mut granted_pack_permission_scopes = BTreeMap::new();
    let mut capability_projections = Vec::new();

    if let Some(declaration) = declaration {
        let mut required_inputs = declaration.use_packs.clone();
        required_inputs.extend(declaration.required_packs.clone());
        required_packs.extend(required_inputs.iter().cloned());
        optional_packs.extend(declaration.optional_packs.iter().cloned());

        resolve_pack_set(
            &required_inputs,
            &declaration.pack_permission_scopes,
            catalog,
            &mut services,
            &mut service_sources,
            &mut service_command_schemas,
            &mut service_result_schemas,
            &mut resolved_packs,
            &mut unresolved_required_packs,
            &mut incompatible_packs,
            &mut unavailable_pack_reasons,
            &mut granted_pack_permission_scopes,
            &mut capability_projections,
            true,
        );
        resolve_pack_set(
            &declaration.optional_packs,
            &declaration.pack_permission_scopes,
            catalog,
            &mut services,
            &mut service_sources,
            &mut service_command_schemas,
            &mut service_result_schemas,
            &mut resolved_packs,
            &mut unresolved_optional_packs,
            &mut incompatible_packs,
            &mut unavailable_pack_reasons,
            &mut granted_pack_permission_scopes,
            &mut capability_projections,
            false,
        );

        services.extend(declaration.required_services.iter().cloned());
        services.extend(declaration.optional_services.iter().cloned());
    }

    unresolved_packs.extend(unresolved_required_packs.iter().cloned());
    unresolved_packs.extend(unresolved_optional_packs.iter().cloned());
    let result = EffectiveServiceCapabilities {
        capabilities_hash: hash_services(&services),
        services,
        service_sources,
        service_command_schemas,
        service_result_schemas,
        resolved_packs,
        required_packs,
        optional_packs,
        unresolved_required_packs,
        unresolved_optional_packs,
        incompatible_packs,
        unresolved_packs,
        unavailable_pack_reasons,
        granted_pack_permission_scopes,
        capability_projections,
    };
    tracing::info!(
        resolved_pack_count = result.resolved_packs.len(),
        unresolved_required_pack_count = result.unresolved_required_packs.len(),
        unresolved_optional_pack_count = result.unresolved_optional_packs.len(),
        incompatible_pack_count = result.incompatible_packs.len(),
        unavailable_reason_count = result.unavailable_pack_reasons.len(),
        service_count = result.services.len(),
        capabilities_hash = %result.capabilities_hash,
        "pack_resolved"
    );
    result
}

fn resolve_pack_set(
    pack_ids: &[String],
    declared_scopes: &BTreeMap<String, BTreeSet<String>>,
    catalog: &dyn DomainPackCatalog,
    services: &mut BTreeSet<String>,
    service_sources: &mut BTreeMap<String, String>,
    service_command_schemas: &mut BTreeMap<String, BTreeSet<String>>,
    service_result_schemas: &mut BTreeMap<String, BTreeSet<String>>,
    resolved_packs: &mut Vec<String>,
    unresolved: &mut Vec<String>,
    incompatible_packs: &mut Vec<String>,
    unavailable_pack_reasons: &mut BTreeMap<String, String>,
    granted_pack_permission_scopes: &mut BTreeMap<String, BTreeSet<String>>,
    capability_projections: &mut Vec<DomainPackEffectiveCapabilityProjection>,
    required: bool,
) {
    for pack_id in pack_ids {
        match catalog.resolve(pack_id) {
            Some(pack) => {
                let requested_scopes = declared_scopes.get(pack_id).cloned().unwrap_or_default();
                if let Err(error) = DomainPackDefinitionSpec.validate(&pack) {
                    tracing::warn!(
                        pack_id = %pack_id,
                        error = %error,
                        "pack_resolution_failed"
                    );
                    incompatible_packs.push(pack_id.clone());
                    capability_projections.push(absent_projection(
                        pack_id,
                        required,
                        "pack_incompatible",
                    ));
                    continue;
                }
                if !pack.is_callable() {
                    let reason = if pack.metadata.diagnostics.unavailable_reason.is_empty() {
                        "pack descriptor is not callable in the active catalog"
                    } else {
                        pack.metadata.diagnostics.unavailable_reason.as_str()
                    };
                    tracing::warn!(
                        pack_id = %pack_id,
                        availability = ?pack.metadata.availability,
                        stability = ?pack.metadata.stability,
                        reason = reason,
                        "pack_unavailable"
                    );
                    unavailable_pack_reasons.insert(pack_id.clone(), reason.to_string());
                    unresolved.push(pack_id.clone());
                    capability_projections.extend(pack_projections(
                        &pack,
                        required,
                        false,
                        Some(reason),
                    ));
                    continue;
                }
                if !requested_scopes.is_subset(&pack.metadata.permission_scopes) {
                    const REASON: &str = "permission_scope_not_declared";
                    tracing::warn!(
                        pack_id = %pack_id,
                        requested_scope_count = requested_scopes.len(),
                        "pack_permission_admission_denied"
                    );
                    unavailable_pack_reasons.insert(pack_id.clone(), REASON.into());
                    unresolved.push(pack_id.clone());
                    capability_projections.extend(pack_projections(
                        &pack,
                        required,
                        false,
                        Some(REASON),
                    ));
                    continue;
                }
                tracing::info!(
                    pack_id = %pack_id,
                    service_count = pack.services.len(),
                    "pack_resolved"
                );
                resolved_packs.push(pack_id.clone());
                if !requested_scopes.is_empty() {
                    granted_pack_permission_scopes.insert(pack_id.clone(), requested_scopes);
                }
                capability_projections.extend(pack_projections(&pack, required, true, None));
                for service in pack.services {
                    service_sources
                        .entry(service.clone())
                        .or_insert_with(|| pack_id.clone());
                    if let Some(commands) = pack.metadata.service_command_schemas.get(&service) {
                        service_command_schemas
                            .entry(service.clone())
                            .or_default()
                            .extend(commands.iter().cloned());
                    }
                    if let Some(results) = pack.metadata.service_result_schemas.get(&service) {
                        service_result_schemas
                            .entry(service.clone())
                            .or_default()
                            .extend(results.iter().cloned());
                    }
                    services.insert(service);
                }
            }
            None => {
                tracing::warn!(
                    pack_id = %pack_id,
                    "pack_unavailable"
                );
                unavailable_pack_reasons
                    .insert(pack_id.clone(), "pack descriptor is not installed".into());
                unresolved.push(pack_id.clone());
                capability_projections.push(absent_projection(
                    pack_id,
                    required,
                    "pack_descriptor_not_installed",
                ));
            }
        }
    }
}

fn pack_projections(
    pack: &DomainPackDefinition,
    required: bool,
    callable: bool,
    unavailable_reason: Option<&str>,
) -> Vec<DomainPackEffectiveCapabilityProjection> {
    let mut projections = pack
        .services
        .iter()
        .map(|service_id| {
            let command_schemas = pack
                .metadata
                .service_command_schemas
                .get(service_id)
                .cloned()
                .unwrap_or_default();
            let unavailable_commands = if callable {
                BTreeMap::new()
            } else {
                command_schemas
                    .iter()
                    .map(|command| {
                        (
                            command.clone(),
                            unavailable_reason
                                .unwrap_or("pack descriptor is not callable")
                                .to_string(),
                        )
                    })
                    .collect()
            };
            let unavailable_features = unavailable_reason
                .map(|reason| BTreeMap::from([("pack".into(), reason.to_string())]))
                .unwrap_or_default();

            DomainPackEffectiveCapabilityProjection {
                pack_id: pack.pack_id.clone(),
                service_id: service_id.clone(),
                required,
                callable_commands: if callable {
                    command_schemas
                } else {
                    BTreeSet::new()
                },
                denied_commands: BTreeMap::new(),
                unavailable_commands,
                unavailable_features,
                provider_capability_flags: provider_flags(pack, service_id),
                replay_refs: replay_refs(pack),
            }
        })
        .collect::<Vec<_>>();
    projections.sort_by(|left, right| {
        left.pack_id
            .cmp(&right.pack_id)
            .then_with(|| left.service_id.cmp(&right.service_id))
    });
    projections
}

fn absent_projection(
    pack_id: impl Into<String>,
    required: bool,
    reason: &str,
) -> DomainPackEffectiveCapabilityProjection {
    DomainPackEffectiveCapabilityProjection {
        pack_id: pack_id.into(),
        required,
        unavailable_features: BTreeMap::from([("pack".into(), reason.into())]),
        ..Default::default()
    }
}

fn provider_flags(
    pack: &DomainPackDefinition,
    service_id: &str,
) -> BTreeMap<String, BTreeMap<String, String>> {
    pack.metadata
        .provider_descriptors
        .iter()
        .filter(|(_, descriptor)| descriptor.service_id == service_id)
        .map(|(provider_class, descriptor)| {
            let mut flags = descriptor.metadata.clone();
            flags.insert(
                "availability".into(),
                format!("{:?}", descriptor.availability),
            );
            flags.insert("capability_hash".into(), descriptor.capability_hash.clone());
            flags.insert(
                "compatibility_hash".into(),
                descriptor.compatibility_hash.clone(),
            );
            flags.insert(
                "diagnostics_schema".into(),
                descriptor.diagnostics_schema.clone(),
            );
            flags.insert("service_id".into(), descriptor.service_id.clone());
            (provider_class.clone(), flags)
        })
        .collect()
}

fn replay_refs(pack: &DomainPackDefinition) -> BTreeSet<String> {
    [
        pack.metadata.diagnostics.replay_schema.clone(),
        pack.stable_descriptor_hash(),
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .collect()
}

fn hash_services(services: &BTreeSet<String>) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for service in services {
        service.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}
