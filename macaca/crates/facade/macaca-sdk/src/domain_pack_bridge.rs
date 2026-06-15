//! Facade for provider-neutral domain-pack contracts.
//!
//! # Role in the architecture
//!
//! The SDK exposes only catalog metadata, capability expansion, and registry
//! helpers from `macaca-proto`. Concrete package crates are installed by an
//! approved host composition root and never proxied through SDK features.
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
