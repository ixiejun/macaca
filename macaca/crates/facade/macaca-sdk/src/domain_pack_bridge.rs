//! Facade bridge for domain-pack contracts and optional package crates.
//!
//! # Role in the architecture
//!
//! Presentation shells must not declare direct workspace dependencies on optional
//! domain-pack package crates.  Those edges are tracked as shell dependency debt
//! until absorbed through this SDK bridge.
//!
//! This module implements the **Facade** pattern in two layers:
//! 1. **Contract re-exports** from `macaca-proto::domain_pack_contract` — catalog metadata,
//!    capability expansion, and composition-root registry helpers shared by all layers.
//! 2. **Optional package alias** — when the `domain-pack-finance` feature is enabled, re-exports
//!    `macaca-domain-pack-finance` so shells can register finance providers without a direct
//!    `Cargo.toml` edge on the package crate.
//!
//! # Cycle break
//!
//! Domain-pack contracts were moved from `macaca-app` into `macaca-proto` so package crates
//! no longer depend on the application runtime.  That removes the historical
//! `app → sdk → domain-pack-finance → app` cycle and makes this bridge acyclic:
//! `web → sdk → domain-pack-finance → proto` (with `app → sdk` remaining independent).
//!
//! # Trace and audit
//!
//! Catalog composition logs `registered_pack_count` at the proto layer.  Provider registration
//! logs remain in each package crate's bootstrap factory with `domain_pack` / `service_count`
//! provider-neutral dimensions.

/// Canonical domain-pack contract types and expansion helpers (proto-owned).
pub use macaca_proto::{
    compose_installed_domain_pack_catalog, empty_domain_pack_catalog, expand_service_capabilities,
    AppServiceContractConfig, AppServicePolicyOverride, DomainPackCatalog, DomainPackDefinition,
    EffectiveServiceCapabilities, InMemoryDomainPackCatalog, SharedDomainPackCatalog,
};

/// Optional finance domain-pack package crate alias.
///
/// Enabled by the `domain-pack-finance` feature on `macaca-sdk`.  Composition roots call
/// `finance_pack_catalog_definition` and `finance_domain_pack_registrations` from this alias
/// during host bootstrap.
#[cfg(feature = "domain-pack-finance")]
pub use macaca_domain_pack_finance as domain_pack_finance;

#[cfg(feature = "domain-pack-finance")]
pub use macaca_domain_pack_finance::{
    finance_domain_pack_registrations, finance_pack_catalog_definition,
};
