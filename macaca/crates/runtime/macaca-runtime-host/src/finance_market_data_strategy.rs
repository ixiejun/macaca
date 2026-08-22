//! Strategy boundary for replaceable market-data providers.
use macaca_proto::domain_pack_contract::finance_market_data::FINANCE_MARKET_DATA_COMMANDS;
use macaca_proto::{ServiceError, ServiceResult};
use std::collections::BTreeSet;
pub trait FinanceMarketDataProviderStrategy: Send + Sync {
    fn validate_command(&self, c: &str) -> ServiceResult<()>;
    fn provider_class(&self) -> &'static str;
}
#[derive(Debug, Clone)]
pub struct ConfiguredFinanceMarketDataStrategy {
    commands: BTreeSet<String>,
    provider_class: &'static str,
}
impl ConfiguredFinanceMarketDataStrategy {
    pub fn mock() -> Self {
        Self::with_commands(FINANCE_MARKET_DATA_COMMANDS.iter().copied())
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
impl FinanceMarketDataProviderStrategy for ConfiguredFinanceMarketDataStrategy {
    fn validate_command(&self, c: &str) -> ServiceResult<()> {
        self.commands.contains(c).then_some(()).ok_or_else(|| {
            ServiceError::UnsupportedCommand("market_data_command_unsupported".into())
        })
    }
    fn provider_class(&self) -> &'static str {
        self.provider_class
    }
}
