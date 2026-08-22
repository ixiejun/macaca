//! Provider-neutral Strategy adapters for identity-profile operations.
//!
//! The Strategy receives only command names and capability configuration. It
//! never receives profile values, credentials, avatar bytes, or application
//! preference payloads, keeping provider replacement behind the service port.

use std::collections::BTreeSet;

use macaca_proto::domain_pack_contract::identity_profile::{
    ProfileProviderCapability, IDENTITY_PROFILE_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};

pub trait IdentityProfileProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> ProfileProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredIdentityProfileStrategy {
    commands: BTreeSet<String>,
    capability: ProfileProviderCapability,
}

impl ConfiguredIdentityProfileStrategy {
    /// Build a synthetic profile strategy with all protocol commands enabled.
    pub fn mock() -> Self {
        Self::with_commands(IDENTITY_PROFILE_COMMANDS.iter().copied())
    }

    /// Build a mock with an explicit command gap for capability-admission tests.
    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        Self {
            capability: ProfileProviderCapability {
                provider_class: "mock".into(),
                feature_flags: commands.clone(),
                supported_value_types: BTreeSet::from(["reference".into(), "hash".into()]),
                limits: [
                    ("max_page_size".into(), 100),
                    ("max_snapshot_items".into(), 100),
                ]
                .into_iter()
                .collect(),
                state: DomainPackProviderCapabilityState::Preview,
            },
            commands,
        }
    }

    /// Build an explicit unavailable Null Object strategy.
    pub fn unavailable() -> Self {
        let mut strategy = Self::with_commands(std::iter::empty::<String>());
        strategy.capability.provider_class = "unavailable".into();
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl IdentityProfileProviderStrategy for ConfiguredIdentityProfileStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand(command.into()))
    }

    fn capability(&self) -> ProfileProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_strategy_supports_replacement_and_capability_gaps() {
        let strategy = ConfiguredIdentityProfileStrategy::with_commands(["profile.read_profile"]);
        assert!(strategy.validate_command("profile.read_profile").is_ok());
        assert!(strategy.validate_command("profile.update_profile").is_err());
        assert_eq!(strategy.capability().feature_flags.len(), 1);
        assert_eq!(
            ConfiguredIdentityProfileStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
