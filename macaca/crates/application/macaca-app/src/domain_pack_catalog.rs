//! Application-layer re-export of composition-root domain-pack catalog helpers.
//!
//! Catalog assembly helpers are canonical in `macaca_proto::domain_pack_contract` so
//! presentation shells and optional package crates can share the same registry semantics
//! without creating `app ↔ pack` cyclic dependencies.

pub use macaca_proto::{
    compose_installed_domain_pack_catalog, empty_domain_pack_catalog, SharedDomainPackCatalog,
};
