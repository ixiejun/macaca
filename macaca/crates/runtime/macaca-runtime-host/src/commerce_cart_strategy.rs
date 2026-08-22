//! Provider-neutral Strategy for replaceable cart adapters.

use std::collections::BTreeSet;

use macaca_proto::domain_pack_contract::commerce_cart::{
    CartProviderCapability, COMMERCE_CART_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};

pub trait CommerceCartProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> CartProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredCommerceCartStrategy {
    commands: BTreeSet<String>,
    capability: CartProviderCapability,
}

impl ConfiguredCommerceCartStrategy {
    pub fn mock() -> Self {
        Self::with_commands(COMMERCE_CART_COMMANDS.iter().copied())
    }

    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        Self {
            capability: CartProviderCapability {
                provider_class: "mock".into(),
                feature_flags: commands
                    .iter()
                    .cloned()
                    .chain(
                        [
                            "version_tokens",
                            "stale_data",
                            "async_export",
                            "cancellation",
                        ]
                        .map(String::from),
                    )
                    .collect(),
                limits: [
                    ("max_page_size".into(), 100),
                    ("max_lines".into(), 100),
                    ("max_export_bytes".into(), 65_536),
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
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl CommerceCartProviderStrategy for ConfiguredCommerceCartStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("cart_command_unsupported".into()))
    }
    fn capability(&self) -> CartProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strategy_supports_capability_gaps() {
        let strategy = ConfiguredCommerceCartStrategy::with_commands(["cart.read_cart"]);
        assert!(strategy.validate_command("cart.read_cart").is_ok());
        assert!(strategy.validate_command("cart.checkout").is_err());
        assert_eq!(
            ConfiguredCommerceCartStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
