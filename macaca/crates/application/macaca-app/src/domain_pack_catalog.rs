//! Composition-root owned domain pack catalog helpers.
//!
//! Domain packs are optional OS extensions declared by applications through
//! `service_contract.use_packs`.  Base `macaca-app` crates remain pack-neutral;
//! shell composition roots assemble an installed catalog from package crates
//! (for example `macaca-domain-pack-finance`) and inject the shared catalog
//! into `AppRuntime`, Application Service projections, and WASM policy sync.
//!
//! Design patterns:
//! - **Registry**: `InMemoryDomainPackCatalog` stores pack metadata.
//! - **Composition Root**: `compose_installed_domain_pack_catalog` merges
//!   package-provided definitions at host startup only.

use std::sync::Arc;

use tracing::info;

use crate::service_capability::{DomainPackCatalog, DomainPackDefinition, InMemoryDomainPackCatalog};

/// Thread-safe catalog handle shared across runtime, service provider, and UI.
pub type SharedDomainPackCatalog = Arc<dyn DomainPackCatalog>;

/// Build an empty catalog for unit tests and hosts without optional packs.
///
/// Empty catalogs still allow `required_services` / `optional_services` from
/// manifest declarations to resolve; only pack expansion stays unresolved.
pub fn empty_domain_pack_catalog() -> SharedDomainPackCatalog {
    Arc::new(InMemoryDomainPackCatalog::with_builtin_defaults())
}

/// Merge package-provided definitions into one deterministic in-memory catalog.
///
/// Later definitions with the same `pack_id` replace earlier entries so
/// composition roots can override test fixtures without branching on pack names.
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
