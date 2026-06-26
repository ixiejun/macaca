//! Provider-neutral domain-pack contract shared across application, package, and shell layers.
//!
//! Domain packs are declarative capability bundles.  Applications request packs through
//! manifest service contracts; optional package crates publish metadata and concrete service
//! providers separately.  This module owns only the data contracts, deterministic expansion,
//! validation specifications, and trace-safe adapter helpers.
//!
//! # Design patterns
//! - **Value Object**: pack metadata structs are immutable DTOs.
//! - **Registry**: the in-memory catalog provides deterministic lookup.
//! - **Strategy**: the catalog trait allows alternate stores without changing expansion.
//! - **Specification**: validators make taxonomy rules executable and auditable.
//! - **Composition Root**: catalog composition happens once at host bootstrap.

mod catalog;
mod expansion;
mod model;
mod reference_catalogs;
mod service_helpers;
mod spec;

#[cfg(test)]
mod tests;

pub use catalog::{
    compose_installed_domain_pack_catalog, empty_domain_pack_catalog, DomainPackCatalog,
    InMemoryDomainPackCatalog, SharedDomainPackCatalog,
};
pub use expansion::{expand_service_capabilities, EffectiveServiceCapabilities};
pub use model::{
    AppPackPolicyOverride, AppServiceContractConfig, AppServicePolicyOverride,
    DomainPackCompatibility, DomainPackDataGovernance, DomainPackDefinition, DomainPackDiagnostics,
    DomainPackMetadata, DomainPackPolicyTemplate, DomainPackProviderSnapshot,
    DomainPackSdkMetadata, DomainPackStability, DomainPackUnavailableDiagnostic,
};
pub use reference_catalogs::{
    developer_pack_definition, foundation_pack_definition, knowledge_pack_definition,
    reference_domain_pack_definitions,
};
pub use service_helpers::{
    domain_pack_command_trace, domain_pack_service_adapter_error, domain_pack_service_result,
};
pub use spec::{
    validate_domain_pack_family_id, validate_domain_pack_id, validate_domain_pack_parent,
    validate_domain_pack_version, AppServiceContractSpec, DomainPackDefinitionSpec,
    DomainPackHierarchySpec, DomainPackIdentitySpec,
};
