//! Finance domain-pack registration factory for composition roots.
//!
//! Uses the Abstract Factory pattern: this module materializes
//! [`DomainPackProviderRegistration`] products that the generic runtime-host
//! bootstrap accepts without understanding finance semantics.

use std::sync::Arc;

use macaca_llm::LlmProvider;
use macaca_kernel::DomainPackProviderRegistration;
use tracing::info;

use crate::contract::{
    finance_descriptor, FINANCE_FINANCIALS_SERVICE_ID, FINANCE_LLM_ANALYSIS_SERVICE_ID,
    FINANCE_MARKET_DATA_SERVICE_ID, FINANCE_NEWS_DIGEST_SERVICE_ID, FINANCE_PACK_ID,
};
use crate::data_provider::FinanceDataSystemServiceProvider;
use crate::llm_analysis_provider::FinanceLlmAnalysisSystemServiceProvider;

/// Build package-owned provider registrations for the finance domain pack.
///
/// Composition roots pass the returned iterator to
/// `macaca_runtime_host::bootstrap_domain_pack_services`.  When this list is
/// omitted, finance service ids remain absent and `service.call` surfaces
/// structured unavailable results instead of synthetic business output.
pub fn finance_domain_pack_registrations(
    llm: Arc<dyn LlmProvider>,
) -> Vec<DomainPackProviderRegistration> {
    info!(
        domain_pack = FINANCE_PACK_ID,
        service_count = 4_usize,
        "building finance domain-pack provider registrations"
    );
    vec![
        DomainPackProviderRegistration::new(
            finance_descriptor(FINANCE_MARKET_DATA_SERVICE_ID, "finance.market_data"),
            Arc::new(FinanceDataSystemServiceProvider::market_data()),
            "finance-market-data",
        ),
        DomainPackProviderRegistration::new(
            finance_descriptor(FINANCE_FINANCIALS_SERVICE_ID, "finance.financials"),
            Arc::new(FinanceDataSystemServiceProvider::financials()),
            "finance-financials",
        ),
        DomainPackProviderRegistration::new(
            finance_descriptor(FINANCE_NEWS_DIGEST_SERVICE_ID, "finance.news_digest"),
            Arc::new(FinanceDataSystemServiceProvider::news_digest()),
            "finance-news-digest",
        ),
        DomainPackProviderRegistration::new(
            finance_descriptor(FINANCE_LLM_ANALYSIS_SERVICE_ID, "finance.llm_analysis"),
            Arc::new(FinanceLlmAnalysisSystemServiceProvider::new(llm)),
            "finance-llm-analysis",
        ),
    ]
}
