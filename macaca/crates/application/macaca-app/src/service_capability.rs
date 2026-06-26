//! Application-layer re-export of the canonical domain-pack contract in `macaca-proto`.
//!
//! Historical consumers import these symbols from `macaca_app::service_capability`.
//! The implementation lives in `macaca_proto::domain_pack_contract` so optional domain-pack
//! package crates can share the same types without depending on `macaca-app`.

pub use macaca_proto::{
    expand_service_capabilities, AppPackPolicyOverride, AppServiceContractConfig,
    AppServiceContractSpec, AppServicePolicyOverride, DomainPackCatalog, DomainPackCompatibility,
    DomainPackDataGovernance, DomainPackDefinition, DomainPackDefinitionSpec,
    DomainPackDiagnostics, DomainPackHierarchySpec, DomainPackIdentitySpec, DomainPackMetadata,
    DomainPackPolicyTemplate, DomainPackSdkMetadata, DomainPackStability,
    EffectiveServiceCapabilities, InMemoryDomainPackCatalog,
};
