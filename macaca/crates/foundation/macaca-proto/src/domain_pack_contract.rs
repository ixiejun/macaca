//! Provider-neutral domain-pack contract shared across application, package, and shell layers.
//!
//! Optional domain packs extend the OS with additional `service.*` identifiers declared
//! in application manifests (`service_contract.use_packs`).  This module owns the
//! **data-only** contract: pack metadata, catalog lookup, and deterministic capability
//! expansion.  Provider implementations live in optional package crates; composition
//! roots register catalog entries and service providers at host startup.
//!
//! # Design patterns
//! - **Value Object**: [`DomainPackDefinition`] is immutable pack metadata (pack id + services).
//! - **Registry**: [`InMemoryDomainPackCatalog`] stores pack definitions with deterministic ordering.
//! - **Strategy**: [`DomainPackCatalog`] trait allows alternate catalog backends without changing expansion.
//! - **Composition Root**: [`compose_installed_domain_pack_catalog`] merges package definitions once at bootstrap.
//!
//! # Layering rationale
//!
//! Placed in `macaca-proto` (not `macaca-app`) so optional package crates such as
//! `macaca-domain-pack-finance` can publish catalog metadata without depending on the
//! application runtime crate.  That breaks the historical `app → sdk → pack → app` cycle
//! and lets presentation shells reach packs through `macaca-sdk` bridges only.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use crate::{
    CleanupPolicy, MacacaError, ServiceCallResult, ServiceCommand, ServiceError, ServiceResult,
    TraceContext,
};

/// Optional service declaration block in `app.yaml` `service_contract` section.
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

/// Domain pack lookup abstraction used during manifest capability expansion.
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
    /// Domain-pack metadata is owned by optional package crates.  Composition roots
    /// register installed packs through [`Self::register`] rather than hardcoding pack ids here.
    pub fn with_builtin_defaults() -> Self {
        Self::new()
    }
}

impl DomainPackCatalog for InMemoryDomainPackCatalog {
    fn resolve(&self, pack_id: &str) -> Option<DomainPackDefinition> {
        self.packs.get(pack_id).cloned()
    }
}

/// Result of deterministic capability expansion from manifest declarations + catalog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveServiceCapabilities {
    /// Ordered set of service identifiers allowed by declarations.
    pub services: BTreeSet<String>,
    /// Packs that were successfully resolved.
    pub resolved_packs: Vec<String>,
    /// Pack ids that were declared but not found in the installed catalog.
    pub unresolved_packs: Vec<String>,
    /// Stable hash for audit logs and replay correlation.
    pub capabilities_hash: String,
}

/// Expand one manifest-level declaration into effective capabilities.
///
/// Walks declared `use_packs` through the catalog, merges expanded service ids with
/// explicit `required_services` / `optional_services`, and records unresolved pack ids
/// for diagnostics (never silently ignored).
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

/// Thread-safe catalog handle shared across runtime, service provider, and UI layers.
pub type SharedDomainPackCatalog = Arc<dyn DomainPackCatalog>;

/// Build an empty catalog for unit tests and hosts without optional packs.
///
/// Empty catalogs still allow `required_services` / `optional_services` from manifest
/// declarations to resolve; only pack expansion stays unresolved.
pub fn empty_domain_pack_catalog() -> SharedDomainPackCatalog {
    Arc::new(InMemoryDomainPackCatalog::with_builtin_defaults())
}

/// Merge package-provided definitions into one deterministic in-memory catalog.
///
/// Later definitions with the same `pack_id` replace earlier entries so composition roots
/// can override test fixtures without branching on pack names.
pub fn compose_installed_domain_pack_catalog(
    definitions: impl IntoIterator<Item = DomainPackDefinition>,
) -> SharedDomainPackCatalog {
    let mut catalog = InMemoryDomainPackCatalog::new();
    let mut registered = 0_usize;
    for definition in definitions {
        registered += 1;
        catalog.register(definition);
    }
    info!(
        registered_pack_count = registered,
        "Composed installed domain-pack catalog for composition root"
    );
    Arc::new(catalog)
}

/// Common result builder used by descriptor-owned domain-pack service adapters.
///
/// Package providers may add bounded metadata inside the output payload, but OS-level
/// observability must stay provider-neutral (`provider_class` only).
pub fn domain_pack_service_result(
    output: Value,
    trace: TraceContext,
    provider_class: &'static str,
) -> ServiceCallResult {
    let mut metadata = BTreeMap::new();
    metadata.insert("provider_class".into(), provider_class.into());
    ServiceCallResult {
        output,
        trace,
        status: "ok".into(),
        metadata,
        cleanup_hint: Some(CleanupPolicy::None),
    }
}

/// Extract a trace context or fail before domain-pack provider logic runs.
///
/// Every domain-pack service call must be trace-addressable before side effects or
/// external adapter invocation so audit replay can correlate package providers.
pub fn domain_pack_command_trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
    command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)
}

/// Map bounded OS errors into structured service-unavailable responses for package adapters.
pub fn domain_pack_service_adapter_error(error: MacacaError) -> ServiceError {
    ServiceError::ServiceUnavailable(error.to_string())
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
