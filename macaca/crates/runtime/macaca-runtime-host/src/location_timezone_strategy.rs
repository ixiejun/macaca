//! Provider-neutral Strategy adapters for timezone implementations.

use std::collections::{BTreeMap, BTreeSet};

use macaca_proto::domain_pack_contract::location_timezone::{
    TimezoneProviderCapability, LOCATION_TIMEZONE_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};

pub trait LocationTimezoneProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> TimezoneProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredLocationTimezoneStrategy {
    commands: BTreeSet<String>,
    capability: TimezoneProviderCapability,
}

impl ConfiguredLocationTimezoneStrategy {
    pub fn mock() -> Self {
        Self::with_commands(LOCATION_TIMEZONE_COMMANDS.iter().copied())
    }

    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        Self {
            capability: TimezoneProviderCapability {
                provider_class: "mock".into(),
                identifier_systems: BTreeSet::from(["iana".into(), "windows".into()]),
                supported_resolvers: BTreeSet::from([
                    "reject".into(),
                    "earlier".into(),
                    "later".into(),
                    "compatible".into(),
                    "explicit_offset".into(),
                ]),
                dataset_versions: BTreeMap::from([
                    ("tzdb".into(), "synthetic-2026a".into()),
                    ("cldr".into(), "synthetic-47".into()),
                ]),
                state: DomainPackProviderCapabilityState::Preview,
            },
            commands,
        }
    }

    pub fn unavailable() -> Self {
        let mut strategy = Self::with_commands(std::iter::empty::<String>());
        strategy.capability.provider_class = "unavailable".into();
        strategy.capability.identifier_systems.clear();
        strategy.capability.supported_resolvers.clear();
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl LocationTimezoneProviderStrategy for ConfiguredLocationTimezoneStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("timezone_command_unsupported".into()))
    }

    fn capability(&self) -> TimezoneProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_supports_resolver_and_command_gaps() {
        let strategy = ConfiguredLocationTimezoneStrategy::with_commands(["timezone.get_offset"]);
        assert!(strategy.validate_command("timezone.get_offset").is_ok());
        assert!(strategy
            .validate_command("timezone.list_transitions")
            .is_err());
        assert_eq!(
            ConfiguredLocationTimezoneStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
