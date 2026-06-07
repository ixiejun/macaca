//! Generic service capability declaration and expansion for applications.
//!
//! This module keeps capability resolution data-driven and provider-neutral.
//! The OS core does not need app-specific branches; applications declare packs
//! and service requirements, and the resolver computes an effective service set.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// Optional service declaration block in `app.yaml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppServiceContractConfig {
    /// Domain pack references. Example: `pack.finance.v1`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub use_packs: Vec<String>,
    /// Mandatory service identifiers required by the application.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_services: Vec<String>,
    /// Optional service identifiers that can improve output quality.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional_services: Vec<String>,
    /// Per-service policy override declarations (bounded metadata only).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub service_policy_overrides: BTreeMap<String, AppServicePolicyOverride>,
}

/// Bounded policy override declared by applications.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppServicePolicyOverride {
    /// Optional request timeout in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Optional max retry count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
}

/// Data-only catalog entry for one domain pack.
#[derive(Debug, Clone, Default)]
pub struct DomainPackDefinition {
    /// Stable pack identifier, for example `pack.finance.v1`.
    pub pack_id: String,
    /// Services expanded from this pack.
    pub services: BTreeSet<String>,
}

/// Domain pack lookup abstraction.
pub trait DomainPackCatalog: Send + Sync {
    /// Resolve one pack id to a pack definition.
    fn resolve(&self, pack_id: &str) -> Option<DomainPackDefinition>;
}

/// In-memory catalog with deterministic ordering and test-friendly behavior.
#[derive(Debug, Clone, Default)]
pub struct InMemoryDomainPackCatalog {
    packs: BTreeMap<String, DomainPackDefinition>,
}

impl InMemoryDomainPackCatalog {
    /// Create an empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or replace one data-only domain pack definition.
    pub fn register(&mut self, definition: DomainPackDefinition) {
        self.packs.insert(definition.pack_id.clone(), definition);
    }

    /// Build an empty catalog for generic runtime startup paths.
    ///
    /// Domain-pack metadata is owned by optional package crates (for example
    /// `macaca-domain-pack-finance`).  Composition roots register installed
    /// packs through [`Self::register`] rather than hardcoding pack ids here.
    pub fn with_builtin_defaults() -> Self {
        Self::new()
    }
}

impl DomainPackCatalog for InMemoryDomainPackCatalog {
    fn resolve(&self, pack_id: &str) -> Option<DomainPackDefinition> {
        self.packs.get(pack_id).cloned()
    }
}

/// Result of deterministic capability expansion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveServiceCapabilities {
    /// Ordered set of service identifiers allowed by declarations.
    pub services: BTreeSet<String>,
    /// Packs that were successfully resolved.
    pub resolved_packs: Vec<String>,
    /// Pack ids that were declared but not found.
    pub unresolved_packs: Vec<String>,
    /// Stable hash for audit logs and replay correlation.
    pub capabilities_hash: String,
}

/// Expand one manifest-level declaration into effective capabilities.
pub fn expand_service_capabilities(
    declaration: Option<&AppServiceContractConfig>,
    catalog: &dyn DomainPackCatalog,
) -> EffectiveServiceCapabilities {
    let mut services = BTreeSet::new();
    let mut resolved_packs = Vec::new();
    let mut unresolved_packs = Vec::new();
    if let Some(declaration) = declaration {
        for pack_id in &declaration.use_packs {
            if let Some(pack) = catalog.resolve(pack_id) {
                resolved_packs.push(pack_id.clone());
                services.extend(pack.services);
            } else {
                unresolved_packs.push(pack_id.clone());
            }
        }
        services.extend(declaration.required_services.iter().cloned());
        services.extend(declaration.optional_services.iter().cloned());
    }
    EffectiveServiceCapabilities {
        capabilities_hash: hash_services(&services),
        services,
        resolved_packs,
        unresolved_packs,
    }
}

fn hash_services(services: &BTreeSet<String>) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for service in services {
        service.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_merges_packs_and_declared_services() {
        let mut catalog = InMemoryDomainPackCatalog::new();
        catalog.register(DomainPackDefinition {
            pack_id: "pack.example.v1".into(),
            services: BTreeSet::from(["service.market_data".into()]),
        });
        let declaration = AppServiceContractConfig {
            use_packs: vec!["pack.example.v1".into()],
            required_services: vec!["service.custom.required".into()],
            optional_services: vec!["service.custom.optional".into()],
            service_policy_overrides: BTreeMap::new(),
        };
        let expanded = expand_service_capabilities(Some(&declaration), &catalog);
        assert!(expanded.services.contains("service.market_data"));
        assert!(expanded.services.contains("service.custom.required"));
        assert!(expanded.services.contains("service.custom.optional"));
        assert!(expanded.unresolved_packs.is_empty());
        assert!(!expanded.capabilities_hash.is_empty());
    }
}
