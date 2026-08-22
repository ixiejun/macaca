//! Provider-neutral Strategy for replaceable place-search adapters.

use std::collections::{BTreeMap, BTreeSet};

use macaca_proto::domain_pack_contract::location_place_search::{
    PlaceSearchProviderCapability, LOCATION_PLACE_SEARCH_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};

pub trait PlaceSearchProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> PlaceSearchProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredPlaceSearchStrategy {
    commands: BTreeSet<String>,
    capability: PlaceSearchProviderCapability,
}

impl ConfiguredPlaceSearchStrategy {
    pub fn mock() -> Self {
        Self::with_commands(LOCATION_PLACE_SEARCH_COMMANDS.iter().copied())
    }

    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        Self {
            capability: PlaceSearchProviderCapability {
                provider_class: "mock".into(),
                supported_fields: BTreeSet::from([
                    "summary".into(),
                    "category".into(),
                    "opening_hours_reference".into(),
                    "media_reference".into(),
                ]),
                supported_categories: BTreeSet::from(["synthetic".into(), "food".into()]),
                cost_classes: BTreeMap::from([("details".into(), "metered".into())]),
                limits: BTreeMap::from([
                    ("max_page_size".into(), 20),
                    ("max_session_count".into(), 32),
                    ("max_snapshot_items".into(), 100),
                ]),
                state: DomainPackProviderCapabilityState::Preview,
            },
            commands,
        }
    }

    pub fn unavailable() -> Self {
        let mut strategy = Self::with_commands(std::iter::empty::<String>());
        strategy.capability.provider_class = "unavailable".into();
        strategy.capability.supported_fields.clear();
        strategy.capability.supported_categories.clear();
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl PlaceSearchProviderStrategy for ConfiguredPlaceSearchStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| {
                ServiceError::UnsupportedCommand("place_search_command_unsupported".into())
            })
    }

    fn capability(&self) -> PlaceSearchProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_supports_field_and_command_gaps() {
        let strategy = ConfiguredPlaceSearchStrategy::with_commands(["place_search.search"]);
        assert!(strategy.validate_command("place_search.search").is_ok());
        assert!(strategy
            .validate_command("place_search.get_details")
            .is_err());
        assert_eq!(
            ConfiguredPlaceSearchStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
