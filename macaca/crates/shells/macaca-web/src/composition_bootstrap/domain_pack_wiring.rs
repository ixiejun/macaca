//! Web composition-root wiring for domain-pack extension contracts.
//!
//! Domain packs are OS extensions declared by applications through manifest
//! `service_contract.use_packs`. Base `macaca-web` remains pack-neutral.
//! Concrete package installation belongs to an external host composition root.
//!
//! # Design patterns
//! - **Composition Root**: catalog and provider lists are assembled once at host
//!   startup and injected into `AppRuntime`, Application Service, and UI routes.
//! - **Abstract Factory**: external host crates inject package registrations
//!   without embedding domain semantics in the Web shell.

use std::sync::Arc;

use macaca_host_composition::application_bootstrap::DomainPackProviderRegistration;
use macaca_host_composition::llm::LlmProvider;
use macaca_sdk::{empty_domain_pack_catalog, SharedDomainPackCatalog};
use tracing::info;

/// Build the catalog of domain packs installed by this web composition root.
///
/// The returned handle is shared across `AppRuntime`, Application Service
/// projections, WASM policy sync, and UI allowlist expansion.  When no optional
/// packs are enabled the catalog is empty and manifest `use_packs` entries
/// surface as `unresolved_domain_packs` in diagnostics instead of being silently
/// expanded.
pub(crate) fn build_installed_domain_pack_catalog() -> SharedDomainPackCatalog {
    info!("No concrete domain packs installed by the base Web shell");
    empty_domain_pack_catalog()
}

/// Collect provider registrations for every domain pack installed by this host.
///
/// Registrations are forwarded to `macaca_host_composition::application_bootstrap::bootstrap_domain_pack_services`
/// during service-runtime wiring.  Absent registrations keep service ids
/// unavailable with structured errors instead of OS-owned business fallbacks.
pub(crate) fn installed_domain_pack_provider_registrations(
    llm: Arc<dyn LlmProvider>,
) -> Vec<DomainPackProviderRegistration> {
    let _ = llm;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use macaca_sdk::{expand_service_capabilities, AppServiceContractConfig, DomainPackCatalog};

    use super::build_installed_domain_pack_catalog;

    #[test]
    fn base_web_catalog_leaves_uninstalled_packs_unresolved() {
        let catalog = build_installed_domain_pack_catalog();
        let declaration = AppServiceContractConfig {
            use_packs: vec!["pack.finance.v1".into()],
            ..Default::default()
        };
        let expanded = expand_service_capabilities(Some(&declaration), catalog.as_ref());

        assert_eq!(
            expanded.unresolved_packs,
            vec!["pack.finance.v1".to_string()]
        );
    }

    #[test]
    fn base_web_catalog_stays_empty() {
        let catalog = build_installed_domain_pack_catalog();
        let missing = catalog.resolve("pack.finance.v1");

        assert!(missing.is_none());
    }
}
