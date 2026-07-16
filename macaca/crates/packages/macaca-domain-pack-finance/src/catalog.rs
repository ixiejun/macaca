//! Catalog metadata for the finance domain pack.
//!
//! Catalog entries are data-only: they describe which service ids a pack expands
//! to during manifest capability resolution.  Provider implementations are
//! registered separately through [`crate::bootstrap::finance_domain_pack_registrations`].

use std::collections::{BTreeMap, BTreeSet};

use macaca_proto::{
    DomainPackDataGovernance, DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata,
    DomainPackSdkMetadata, DomainPackStability,
};

use crate::contract::{
    FINANCE_FINANCIALS_SERVICE_ID, FINANCE_LLM_ANALYSIS_SERVICE_ID, FINANCE_MARKET_DATA_SERVICE_ID,
    FINANCE_NEWS_DIGEST_SERVICE_ID, FINANCE_PACK_ID,
};

/// Return the catalog definition for `pack.finance.v1`.
///
/// Composition roots merge this into their `DomainPackCatalog` when the finance
/// optional package is installed.  Base OS catalogs remain empty by default.
pub fn finance_pack_catalog_definition() -> DomainPackDefinition {
    DomainPackDefinition::with_metadata(
        FINANCE_PACK_ID,
        DomainPackMetadata {
            family_id: "finance".into(),
            version: "v1".into(),
            stability: DomainPackStability::Preview,
            service_command_schemas: BTreeMap::from([
                (
                    FINANCE_MARKET_DATA_SERVICE_ID.into(),
                    BTreeSet::from(["finance.market_data.quote.v1".into()]),
                ),
                (
                    FINANCE_FINANCIALS_SERVICE_ID.into(),
                    BTreeSet::from(["finance.financials.statement.v1".into()]),
                ),
                (
                    FINANCE_NEWS_DIGEST_SERVICE_ID.into(),
                    BTreeSet::from(["finance.news.digest.v1".into()]),
                ),
                (
                    FINANCE_LLM_ANALYSIS_SERVICE_ID.into(),
                    BTreeSet::from(["finance.analysis.generate.v1".into()]),
                ),
            ]),
            service_result_schemas: BTreeMap::from([
                (
                    FINANCE_MARKET_DATA_SERVICE_ID.into(),
                    BTreeSet::from(["finance.market_data.quote.result.v1".into()]),
                ),
                (
                    FINANCE_FINANCIALS_SERVICE_ID.into(),
                    BTreeSet::from(["finance.financials.statement.result.v1".into()]),
                ),
                (
                    FINANCE_NEWS_DIGEST_SERVICE_ID.into(),
                    BTreeSet::from(["finance.news.digest.result.v1".into()]),
                ),
                (
                    FINANCE_LLM_ANALYSIS_SERVICE_ID.into(),
                    BTreeSet::from(["finance.analysis.generate.result.v1".into()]),
                ),
            ]),
            source_attribution: BTreeSet::from(["macaca-domain-pack-finance".into()]),
            migration_notes: vec![
                "Finance package services are optional-module providers registered through the composition root.".into(),
            ],
            data_governance: DomainPackDataGovernance {
                classification: "market_reference".into(),
                retention_policy: "bounded_audit_metadata_only".into(),
                redaction_policy: "no_raw_provider_payloads".into(),
            },
            sdk: DomainPackSdkMetadata {
                client_namespace: "finance".into(),
                docs_url: "https://macaca.local/docs/packs/finance".into(),
                examples: vec!["Declare `pack.finance.v1` and call typed finance services.".into()],
            },
            diagnostics: DomainPackDiagnostics {
                health_probe: "service.health".into(),
                unavailable_reason: "finance domain pack provider is not installed".into(),
                replay_schema: "trace.domain_pack.finance.v1".into(),
            },
            ..Default::default()
        },
        BTreeSet::from([
            FINANCE_MARKET_DATA_SERVICE_ID.into(),
            FINANCE_FINANCIALS_SERVICE_ID.into(),
            FINANCE_NEWS_DIGEST_SERVICE_ID.into(),
            FINANCE_LLM_ANALYSIS_SERVICE_ID.into(),
        ]),
    )
}
