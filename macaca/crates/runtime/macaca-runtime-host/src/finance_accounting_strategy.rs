//! Provider-neutral Strategy adapters for finance-accounting services.
//!
//! A strategy advertises only bounded capabilities and validates canonical
//! command names. Provider selection stays in the descriptor-owned registry;
//! this module never branches on vendor names or application workflows.

use std::collections::BTreeSet;

use macaca_proto::domain_pack_contract::finance_accounting::{
    AccountingProviderCapability, FINANCE_ACCOUNTING_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};

pub trait FinanceAccountingProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> AccountingProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredFinanceAccountingStrategy {
    commands: BTreeSet<String>,
    capability: AccountingProviderCapability,
}

impl ConfiguredFinanceAccountingStrategy {
    pub fn mock() -> Self {
        Self::with_commands(FINANCE_ACCOUNTING_COMMANDS.iter().copied())
    }

    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        let supported_reports = ["trial_balance", "balance_sheet", "profit_loss", "cash_flow"]
            .into_iter()
            .map(String::from)
            .collect();
        Self {
            capability: AccountingProviderCapability {
                provider_class: "mock".into(),
                supported_commands: commands.clone(),
                supported_reports,
                write_support: commands.iter().any(|command| {
                    command == "accounting.account_request"
                        || command == "accounting.post_journal"
                        || command == "accounting.reconciliation_request"
                }),
                state: DomainPackProviderCapabilityState::Preview,
            },
            commands,
        }
    }

    pub fn unavailable() -> Self {
        let mut strategy = Self::with_commands(std::iter::empty::<String>());
        strategy.capability.provider_class = "unavailable".into();
        strategy.capability.supported_reports.clear();
        strategy.capability.write_support = false;
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl FinanceAccountingProviderStrategy for ConfiguredFinanceAccountingStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| {
                ServiceError::UnsupportedCommand("accounting_command_unsupported".into())
            })
    }

    fn capability(&self) -> AccountingProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_supports_capability_gaps_and_unavailable_state() {
        let strategy = ConfiguredFinanceAccountingStrategy::with_commands([
            "accounting.get_chart_of_accounts",
        ]);
        assert!(strategy
            .validate_command("accounting.get_chart_of_accounts")
            .is_ok());
        assert!(strategy
            .validate_command("accounting.post_journal")
            .is_err());
        assert!(!strategy.capability().write_support);
        assert_eq!(
            ConfiguredFinanceAccountingStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
