//! Provider-neutral Strategy adapters for identity-account services.
//!
//! The runtime only asks a strategy whether a canonical command is supported
//! and what bounded capabilities it advertises. Concrete directory vendors,
//! plugins, and remote adapters remain outside the OS command path.

use std::collections::{BTreeMap, BTreeSet};

use macaca_proto::domain_pack_contract::identity_account::{
    AccountProviderCapability, IDENTITY_ACCOUNT_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};

pub trait IdentityAccountProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> AccountProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredIdentityAccountStrategy {
    commands: BTreeSet<String>,
    capability: AccountProviderCapability,
}

impl ConfiguredIdentityAccountStrategy {
    pub fn mock() -> Self {
        Self::with_commands(IDENTITY_ACCOUNT_COMMANDS.iter().copied())
    }

    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        Self {
            capability: AccountProviderCapability {
                provider_class: "mock".into(),
                feature_flags: commands.clone(),
                supported_lifecycle_states: BTreeSet::from([
                    "active".into(),
                    "suspended".into(),
                    "disabled".into(),
                ]),
                limits: BTreeMap::from([
                    ("max_page_size".into(), 100),
                    ("max_snapshot_items".into(), 100),
                    ("max_export_bytes".into(), 65_536),
                ]),
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

impl IdentityAccountProviderStrategy for ConfiguredIdentityAccountStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("account_command_unsupported".into()))
    }

    fn capability(&self) -> AccountProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_supports_replacement_and_capability_gaps() {
        let strategy = ConfiguredIdentityAccountStrategy::with_commands(["account.read_account"]);
        assert!(strategy.validate_command("account.read_account").is_ok());
        assert!(strategy.validate_command("account.update_account").is_err());
        assert_eq!(strategy.capability().feature_flags.len(), 1);
        assert_eq!(
            ConfiguredIdentityAccountStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
