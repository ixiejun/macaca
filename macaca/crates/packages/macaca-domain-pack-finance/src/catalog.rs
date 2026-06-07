//! Catalog metadata for the finance domain pack.
//!
//! Catalog entries are data-only: they describe which service ids a pack expands
//! to during manifest capability resolution.  Provider implementations are
//! registered separately through [`crate::bootstrap::finance_domain_pack_registrations`].

use std::collections::BTreeSet;

use macaca_app::DomainPackDefinition;

use crate::contract::{
    FINANCE_FINANCIALS_SERVICE_ID, FINANCE_LLM_ANALYSIS_SERVICE_ID, FINANCE_MARKET_DATA_SERVICE_ID,
    FINANCE_NEWS_DIGEST_SERVICE_ID, FINANCE_PACK_ID,
};

/// Return the catalog definition for `pack.finance.v1`.
///
/// Composition roots merge this into their `DomainPackCatalog` when the finance
/// optional package is installed.  Base OS catalogs remain empty by default.
pub fn finance_pack_catalog_definition() -> DomainPackDefinition {
    DomainPackDefinition {
        pack_id: FINANCE_PACK_ID.into(),
        services: BTreeSet::from([
            FINANCE_MARKET_DATA_SERVICE_ID.into(),
            FINANCE_FINANCIALS_SERVICE_ID.into(),
            FINANCE_NEWS_DIGEST_SERVICE_ID.into(),
            FINANCE_LLM_ANALYSIS_SERVICE_ID.into(),
        ]),
    }
}
