//! Web composition-root wiring for optional domain-pack extensions.
//!
//! Domain packs are OS extensions declared by applications through manifest
//! `service_contract.use_packs`.  Base `macaca-web` remains pack-neutral; this
//! module is the single place where optional package crates (reached through
//! `macaca-sdk::domain_pack_bridge`) are merged into the shared catalog and
//! translated into `DomainPackProviderRegistration` products for
//! `bootstrap_domain_pack_services`.
//!
//! # Design patterns
//! - **Composition Root**: catalog and provider lists are assembled once at host
//!   startup and injected into `AppRuntime`, Application Service, and UI routes.
//! - **Abstract Factory**: package crates expose registration factories; web only
//!   forwards them without embedding domain semantics.
//! - **Facade**: finance pack symbols are imported through `macaca-sdk` so the shell
//!   `Cargo.toml` stays sdk-only for optional packages.
//! - **Feature toggle**: `domain-pack-finance` gates optional dependencies so
//!   minimal hosts can compile without finance packages.

use std::sync::Arc;

use macaca_sdk::{
    compose_installed_domain_pack_catalog, empty_domain_pack_catalog, SharedDomainPackCatalog,
};
use macaca_sdk::llm::LlmProvider;
use macaca_runtime_host::DomainPackProviderRegistration;
use tracing::info;

/// Build the catalog of domain packs installed by this web composition root.
///
/// The returned handle is shared across `AppRuntime`, Application Service
/// projections, WASM policy sync, and UI allowlist expansion.  When no optional
/// packs are enabled the catalog is empty and manifest `use_packs` entries
/// surface as `unresolved_domain_packs` in diagnostics instead of being silently
/// expanded.
pub(crate) fn build_installed_domain_pack_catalog() -> SharedDomainPackCatalog {
    #[cfg(feature = "domain-pack-finance")]
    {
        use macaca_sdk::finance_pack_catalog_definition;

        let catalog = compose_installed_domain_pack_catalog([finance_pack_catalog_definition()]);
        info!(
            feature = "domain-pack-finance",
            "Installed domain-pack catalog composed for web composition root"
        );
        return catalog;
    }

    #[cfg(not(feature = "domain-pack-finance"))]
    {
        info!("No optional domain packs enabled for web composition root");
        empty_domain_pack_catalog()
    }
}

/// Collect provider registrations for every domain pack installed by this host.
///
/// Registrations are forwarded to `macaca_runtime_host::bootstrap_domain_pack_services`
/// during service-runtime wiring.  Absent registrations keep service ids
/// unavailable with structured errors instead of OS-owned business fallbacks.
pub(crate) fn installed_domain_pack_provider_registrations(
    llm: Arc<dyn LlmProvider>,
) -> Vec<DomainPackProviderRegistration> {
    #[cfg(feature = "domain-pack-finance")]
    {
        use macaca_sdk::finance_domain_pack_registrations;

        let registrations = finance_domain_pack_registrations(llm);
        info!(
            feature = "domain-pack-finance",
            provider_count = registrations.len(),
            "Collected domain-pack provider registrations for web composition root"
        );
        return registrations;
    }

    #[cfg(not(feature = "domain-pack-finance"))]
    {
        let _ = llm;
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use macaca_sdk::{expand_service_capabilities, AppServiceContractConfig, DomainPackCatalog};

    use super::build_installed_domain_pack_catalog;

    #[test]
    fn installed_catalog_resolves_finance_pack_when_feature_enabled() {
        let catalog = build_installed_domain_pack_catalog();
        let declaration = AppServiceContractConfig {
            use_packs: vec!["pack.finance.v1".into()],
            ..Default::default()
        };
        let expanded = expand_service_capabilities(Some(&declaration), catalog.as_ref());

        #[cfg(feature = "domain-pack-finance")]
        {
            assert!(
                expanded.unresolved_packs.is_empty(),
                "finance pack should resolve when domain-pack-finance feature is enabled"
            );
            assert!(expanded.services.contains("service.market_data"));
        }

        #[cfg(not(feature = "domain-pack-finance"))]
        {
            assert_eq!(
                expanded.unresolved_packs,
                vec!["pack.finance.v1".to_string()]
            );
        }
    }

    #[test]
    fn installed_catalog_stays_empty_without_optional_features() {
        let catalog = build_installed_domain_pack_catalog();
        let missing = catalog.resolve("pack.finance.v1");

        #[cfg(feature = "domain-pack-finance")]
        assert!(missing.is_some());

        #[cfg(not(feature = "domain-pack-finance"))]
        assert!(missing.is_none());
    }
}
