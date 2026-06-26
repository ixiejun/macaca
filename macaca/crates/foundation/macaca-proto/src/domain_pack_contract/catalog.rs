use std::collections::BTreeMap;
use std::sync::Arc;

use tracing::info;

use super::model::DomainPackDefinition;

/// Domain pack lookup abstraction used during manifest capability expansion.
///
/// Catalog backends are intentionally read-only at this boundary.  A host composition root may
/// source definitions from built-in optional packages, plugins, or remote registries, but callers
/// only see deterministic provider-neutral metadata.
pub trait DomainPackCatalog: Send + Sync {
    /// Return every descriptor visible to this catalog in stable order.
    ///
    /// Discovery callers use this provider-neutral view instead of importing optional package
    /// crates or runtime-host internals.  Implementations must keep the returned descriptors
    /// sanitized and data-only.
    fn list(&self) -> Vec<DomainPackDefinition>;

    /// Resolve one descriptor by stable pack id.
    fn resolve(&self, pack_id: &str) -> Option<DomainPackDefinition>;
}

/// In-memory catalog with deterministic ordering and test-friendly behavior.
#[derive(Debug, Clone, Default)]
pub struct InMemoryDomainPackCatalog {
    packs: BTreeMap<String, DomainPackDefinition>,
}

impl InMemoryDomainPackCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or replace one data-only domain pack definition.
    pub fn register(&mut self, definition: DomainPackDefinition) {
        self.packs.insert(definition.pack_id.clone(), definition);
    }

    /// Build an empty catalog for generic runtime startup paths.
    ///
    /// Optional package crates own concrete catalog entries.  The base OS must remain domain
    /// neutral, so there are no hardcoded built-in pack ids here.
    pub fn with_builtin_defaults() -> Self {
        Self::new()
    }
}

impl DomainPackCatalog for InMemoryDomainPackCatalog {
    fn list(&self) -> Vec<DomainPackDefinition> {
        self.packs.values().cloned().collect()
    }

    fn resolve(&self, pack_id: &str) -> Option<DomainPackDefinition> {
        self.packs.get(pack_id).cloned()
    }
}

/// Thread-safe catalog handle shared across runtime, service provider, and UI layers.
pub type SharedDomainPackCatalog = Arc<dyn DomainPackCatalog>;

/// Build an empty catalog for unit tests and hosts without optional packs.
pub fn empty_domain_pack_catalog() -> SharedDomainPackCatalog {
    Arc::new(InMemoryDomainPackCatalog::with_builtin_defaults())
}

/// Merge package-provided definitions into one deterministic in-memory catalog.
///
/// Later definitions with the same `pack_id` replace earlier entries.  This allows tests and
/// composition roots to override entries declaratively without branching on pack names.
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
