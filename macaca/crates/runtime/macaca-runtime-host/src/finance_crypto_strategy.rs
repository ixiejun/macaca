//! Strategy boundary for replaceable crypto market-data providers.
use macaca_proto::domain_pack_contract::finance_crypto::FINANCE_CRYPTO_COMMANDS;
use macaca_proto::{ServiceError, ServiceResult};
use std::collections::BTreeSet;
pub trait FinanceCryptoProviderStrategy: Send + Sync {
    fn validate_command(&self, c: &str) -> ServiceResult<()>;
    fn provider_class(&self) -> &'static str;
}
#[derive(Debug, Clone)]
pub struct ConfiguredFinanceCryptoStrategy {
    commands: BTreeSet<String>,
    provider_class: &'static str,
}
impl ConfiguredFinanceCryptoStrategy {
    pub fn mock() -> Self {
        Self::with_commands(FINANCE_CRYPTO_COMMANDS.iter().copied())
    }
    pub fn with_commands<I, S>(c: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            commands: c.into_iter().map(Into::into).collect(),
            provider_class: "mock",
        }
    }
    pub fn unavailable() -> Self {
        Self {
            commands: BTreeSet::new(),
            provider_class: "unavailable",
        }
    }
}
impl FinanceCryptoProviderStrategy for ConfiguredFinanceCryptoStrategy {
    fn validate_command(&self, c: &str) -> ServiceResult<()> {
        self.commands
            .contains(c)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("crypto_command_unsupported".into()))
    }
    fn provider_class(&self) -> &'static str {
        self.provider_class
    }
}
