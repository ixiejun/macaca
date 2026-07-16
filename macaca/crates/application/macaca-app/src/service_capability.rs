//! Application-layer re-export of the canonical domain-pack contract in `macaca-proto`.
//!
//! Historical consumers import these symbols from `macaca_app::service_capability`.
//! The implementation lives in `macaca_proto::domain_pack_contract` so optional domain-pack
//! package crates can share the same types without depending on `macaca-app`.

pub use macaca_proto::{
    developer_pack_definition, expand_service_capabilities, foundation_pack_definition,
    industrial_reference_domain_pack_definitions, knowledge_pack_definition,
    reference_domain_pack_definitions, snapshot_domain_pack_catalog, AppPackPolicyOverride,
    AppServiceContractConfig, AppServiceContractSpec, AppServicePolicyOverride,
    DomainPackAvailability, DomainPackCallableSpec, DomainPackCatalog, DomainPackCatalogSnapshot,
    DomainPackCompatibility, DomainPackDataGovernance, DomainPackDefinition,
    DomainPackDefinitionSpec, DomainPackDiagnostics, DomainPackHierarchySpec,
    DomainPackIdentitySpec, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderSnapshot, DomainPackSdkMetadata, DomainPackStability,
    DomainPackUnavailableDiagnostic, EffectiveServiceCapabilities, InMemoryDomainPackCatalog,
};
