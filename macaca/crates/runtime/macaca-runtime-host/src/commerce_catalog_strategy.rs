//! Provider-neutral Strategy boundary for replaceable catalog adapters.
use macaca_proto::domain_pack_contract::commerce_catalog::{
    CatalogProviderCapability, COMMERCE_CATALOG_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};
use std::collections::BTreeSet;

pub trait CommerceCatalogProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> CatalogProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredCommerceCatalogStrategy {
    commands: BTreeSet<String>,
    capability: CatalogProviderCapability,
}

impl ConfiguredCommerceCatalogStrategy {
    pub fn mock() -> Self {
        Self::with_commands(COMMERCE_CATALOG_COMMANDS.iter().copied())
    }
    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        Self {
            capability: CatalogProviderCapability {
                provider_class: "mock".into(),
                product_models: [
                    "product".into(),
                    "variant".into(),
                    "price".into(),
                    "availability".into(),
                    "taxonomy".into(),
                ]
                .into_iter()
                .collect(),
                feature_flags: commands
                    .iter()
                    .cloned()
                    .chain([
                        "localization".into(),
                        "facets".into(),
                        "version_tokens".into(),
                        "async_export".into(),
                        "cancellation".into(),
                    ])
                    .collect(),
                limits: [
                    ("max_page_size".into(), 100),
                    ("max_export_bytes".into(), 65_536),
                    ("max_reference_count".into(), 256),
                ]
                .into_iter()
                .collect(),
                state: DomainPackProviderCapabilityState::Preview,
            },
            commands,
        }
    }
    pub fn unavailable() -> Self {
        let mut strategy = Self::with_commands(std::iter::empty::<String>());
        strategy.capability.provider_class = "unavailable".into();
        strategy.capability.product_models.clear();
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl CommerceCatalogProviderStrategy for ConfiguredCommerceCatalogStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("catalog_command_unsupported".into()))
    }
    fn capability(&self) -> CatalogProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strategy_reports_command_gaps_and_unavailable_state() {
        let strategy = ConfiguredCommerceCatalogStrategy::with_commands(["catalog.get_product"]);
        assert!(strategy.validate_command("catalog.get_product").is_ok());
        assert!(strategy.validate_command("catalog.export_catalog").is_err());
        assert_eq!(
            ConfiguredCommerceCatalogStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
