//! Optional finance/crypto domain-pack services for Macaca Agent OS.
//!
//! This crate is an extension package, not part of the base operating system.
//! Composition roots (web shell, custom hosts, integration tests) register the
//! providers through `finance_domain_pack_registrations` and optionally merge
//! `finance_pack_catalog_definition` into their domain-pack catalog.
//!
//! # Design patterns
//! - **Bridge**: adapters translate exchange/RSS/LLM payloads into stable service JSON.
//! - **Strategy**: `FinanceDataSystemServiceProvider` selects output by `service_kind`.
//! - **Abstract Factory**: `finance_domain_pack_registrations` builds host registrations.
//! - **Null Object** (absence): when not registered, services are unavailable — not faked.

pub mod bootstrap;
pub mod catalog;
pub mod contract;
pub mod data_provider;
pub mod live_data;
pub mod llm_analysis_provider;

pub use bootstrap::finance_domain_pack_registrations;
pub use catalog::finance_pack_catalog_definition;
pub use contract::{
    extract_symbol, finance_descriptor, FINANCE_ANALYZE_COMMAND, FINANCE_FINANCIALS_SERVICE_ID,
    FINANCE_LLM_ANALYSIS_SERVICE_ID, FINANCE_LOOKUP_COMMAND, FINANCE_MARKET_DATA_SERVICE_ID,
    FINANCE_NEWS_DIGEST_SERVICE_ID, FINANCE_PACK_ID,
};
pub use data_provider::FinanceDataSystemServiceProvider;
pub use llm_analysis_provider::FinanceLlmAnalysisSystemServiceProvider;

#[cfg(test)]
mod tests {
    use macaca_proto::{
        expand_service_capabilities, AppServiceContractConfig, InMemoryDomainPackCatalog,
    };
    use serde_json::json;

    use super::catalog::finance_pack_catalog_definition;
    use super::contract::extract_symbol;

    #[test]
    fn catalog_expands_finance_pack_services() {
        let mut catalog = InMemoryDomainPackCatalog::new();
        catalog.register(finance_pack_catalog_definition());
        let declaration = AppServiceContractConfig {
            use_packs: vec!["pack.finance.v1".into()],
            ..Default::default()
        };
        let expanded = expand_service_capabilities(Some(&declaration), &catalog);
        assert!(expanded.services.contains("service.market_data"));
        assert!(expanded.unresolved_packs.is_empty());
    }

    #[test]
    fn extract_symbol_rejects_prompt_shaped_input() {
        let payload = json!({
            "asset_class": "crypto",
            "symbol": "Analyze BTC/USDT now with available skills",
        });
        assert!(extract_symbol(&payload).is_err());
    }
}
