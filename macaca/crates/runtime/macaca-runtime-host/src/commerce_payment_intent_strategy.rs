//! Strategy boundary for replaceable payment-intent providers.
use std::collections::BTreeSet;

use macaca_proto::domain_pack_contract::commerce_payment_intent::{
    PaymentIntentProviderCapability, COMMERCE_PAYMENT_INTENT_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};

pub trait CommercePaymentIntentProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> PaymentIntentProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredCommercePaymentIntentStrategy {
    commands: BTreeSet<String>,
    capability: PaymentIntentProviderCapability,
}

impl ConfiguredCommercePaymentIntentStrategy {
    pub fn mock() -> Self {
        Self::with_commands(COMMERCE_PAYMENT_INTENT_COMMANDS.iter().copied())
    }

    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        Self {
            capability: PaymentIntentProviderCapability {
                provider_class: "mock".into(),
                payment_method_types: ["card", "wallet", "bank_debit"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                capture_modes: ["automatic", "manual"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                feature_flags: commands
                    .iter()
                    .cloned()
                    .chain(
                        [
                            "action_required",
                            "partial_capture",
                            "idempotency",
                            "event_references",
                            "async_export",
                            "cancellation",
                        ]
                        .map(String::from),
                    )
                    .collect(),
                state: DomainPackProviderCapabilityState::Preview,
            },
            commands,
        }
    }

    pub fn unavailable() -> Self {
        let mut strategy = Self::with_commands(std::iter::empty::<String>());
        strategy.capability.provider_class = "unavailable".into();
        strategy.capability.payment_method_types.clear();
        strategy.capability.capture_modes.clear();
        strategy.capability.feature_flags.clear();
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl CommercePaymentIntentProviderStrategy for ConfiguredCommercePaymentIntentStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| {
                ServiceError::UnsupportedCommand("payment_intent_command_unsupported".into())
            })
    }

    fn capability(&self) -> PaymentIntentProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_reports_payment_capability_gaps() {
        let strategy =
            ConfiguredCommercePaymentIntentStrategy::with_commands(["payment_intent.get_status"]);
        assert!(strategy
            .validate_command("payment_intent.get_status")
            .is_ok());
        assert!(strategy.validate_command("payment_intent.capture").is_err());
        assert_eq!(
            ConfiguredCommercePaymentIntentStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
