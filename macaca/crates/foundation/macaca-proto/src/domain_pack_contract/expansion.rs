use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use super::catalog::DomainPackCatalog;
use super::model::AppServiceContractConfig;
use super::spec::DomainPackDefinitionSpec;

/// Result of deterministic capability expansion from manifest declarations plus catalog data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveServiceCapabilities {
    pub services: BTreeSet<String>,
    pub service_sources: BTreeMap<String, String>,
    pub resolved_packs: Vec<String>,
    pub required_packs: Vec<String>,
    pub optional_packs: Vec<String>,
    pub unresolved_required_packs: Vec<String>,
    pub unresolved_optional_packs: Vec<String>,
    pub incompatible_packs: Vec<String>,
    pub unresolved_packs: Vec<String>,
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
    let mut resolved_packs = Vec::new();
    let mut required_packs = Vec::new();
    let mut optional_packs = Vec::new();
    let mut unresolved_required_packs = Vec::new();
    let mut unresolved_optional_packs = Vec::new();
    let mut incompatible_packs = Vec::new();
    let mut unresolved_packs = Vec::new();

    if let Some(declaration) = declaration {
        let mut required_inputs = declaration.use_packs.clone();
        required_inputs.extend(declaration.required_packs.clone());
        required_packs.extend(required_inputs.iter().cloned());
        optional_packs.extend(declaration.optional_packs.iter().cloned());

        resolve_pack_set(
            &required_inputs,
            catalog,
            &mut services,
            &mut service_sources,
            &mut resolved_packs,
            &mut unresolved_required_packs,
            &mut incompatible_packs,
        );
        resolve_pack_set(
            &declaration.optional_packs,
            catalog,
            &mut services,
            &mut service_sources,
            &mut resolved_packs,
            &mut unresolved_optional_packs,
            &mut incompatible_packs,
        );

        services.extend(declaration.required_services.iter().cloned());
        services.extend(declaration.optional_services.iter().cloned());
    }

    unresolved_packs.extend(unresolved_required_packs.iter().cloned());
    unresolved_packs.extend(unresolved_optional_packs.iter().cloned());
    EffectiveServiceCapabilities {
        capabilities_hash: hash_services(&services),
        services,
        service_sources,
        resolved_packs,
        required_packs,
        optional_packs,
        unresolved_required_packs,
        unresolved_optional_packs,
        incompatible_packs,
        unresolved_packs,
    }
}

fn resolve_pack_set(
    pack_ids: &[String],
    catalog: &dyn DomainPackCatalog,
    services: &mut BTreeSet<String>,
    service_sources: &mut BTreeMap<String, String>,
    resolved_packs: &mut Vec<String>,
    unresolved: &mut Vec<String>,
    incompatible_packs: &mut Vec<String>,
) {
    for pack_id in pack_ids {
        match catalog.resolve(pack_id) {
            Some(pack) => {
                if let Err(error) = DomainPackDefinitionSpec.validate(&pack) {
                    tracing::warn!(
                        pack_id = %pack_id,
                        error = %error,
                        "Ignoring incompatible domain-pack definition during expansion"
                    );
                    incompatible_packs.push(pack_id.clone());
                    continue;
                }
                resolved_packs.push(pack_id.clone());
                for service in pack.services {
                    service_sources
                        .entry(service.clone())
                        .or_insert_with(|| pack_id.clone());
                    services.insert(service);
                }
            }
            None => unresolved.push(pack_id.clone()),
        }
    }
}

fn hash_services(services: &BTreeSet<String>) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for service in services {
        service.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}
