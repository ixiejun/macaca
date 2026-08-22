//! Replaceable provider Strategy for the commerce order service.
use std::collections::BTreeSet;

use macaca_proto::domain_pack_contract::commerce_order::{
    OrderProviderCapability, COMMERCE_ORDER_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};

pub trait CommerceOrderProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> OrderProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredCommerceOrderStrategy {
    commands: BTreeSet<String>,
    capability: OrderProviderCapability,
}

impl ConfiguredCommerceOrderStrategy {
    pub fn mock() -> Self {
        Self::with_commands(COMMERCE_ORDER_COMMANDS.iter().copied())
    }

    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        Self {
            capability: OrderProviderCapability {
                provider_class: "mock".into(),
                feature_flags: commands
                    .iter()
                    .cloned()
                    .chain(
                        [
                            "source_conversion",
                            "version_tokens",
                            "status_freshness",
                            "fulfillment_intent",
                            "return_references",
                            "async_export",
                            "cancellation",
                        ]
                        .map(String::from),
                    )
                    .collect(),
                supported_states: [
                    "planned",
                    "created",
                    "paid",
                    "fulfilled",
                    "cancelled",
                    "returned",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                limits: [
                    ("max_page_size".into(), 100),
                    ("max_reference_count".into(), 256),
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
        strategy.capability.feature_flags.clear();
        strategy.capability.supported_states.clear();
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl CommerceOrderProviderStrategy for ConfiguredCommerceOrderStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("order_command_unsupported".into()))
    }

    fn capability(&self) -> OrderProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_exposes_capability_gaps() {
        let strategy = ConfiguredCommerceOrderStrategy::with_commands(["order.read_order"]);
        assert!(strategy.validate_command("order.read_order").is_ok());
        assert!(strategy.validate_command("order.create_order").is_err());
        assert_eq!(
            ConfiguredCommerceOrderStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
