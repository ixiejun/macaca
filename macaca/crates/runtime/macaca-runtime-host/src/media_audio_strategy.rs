//! Provider-neutral Strategy boundary for replaceable audio adapters.

use std::collections::BTreeSet;

use macaca_proto::media_audio::{AudioProviderCapability, MEDIA_AUDIO_COMMANDS};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};

pub trait MediaAudioProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> AudioProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredMediaAudioStrategy {
    commands: BTreeSet<String>,
    capability: AudioProviderCapability,
}

impl ConfiguredMediaAudioStrategy {
    pub fn mock() -> Self {
        Self::with_commands(MEDIA_AUDIO_COMMANDS.iter().copied())
    }

    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        Self {
            capability: AudioProviderCapability {
                provider_class: "mock".into(),
                codecs: BTreeSet::from(["pcm".into()]),
                containers: BTreeSet::from(["wav".into()]),
                features: BTreeSet::from(["metadata_only".into(), "planning".into()]),
                max_duration_ms: 300_000,
                state: DomainPackProviderCapabilityState::Preview,
            },
            commands,
        }
    }

    pub fn unavailable() -> Self {
        let mut strategy = Self::with_commands(std::iter::empty::<String>());
        strategy.capability.provider_class = "unavailable".into();
        strategy.capability.features.clear();
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl MediaAudioProviderStrategy for ConfiguredMediaAudioStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("audio_command_unsupported".into()))
    }

    fn capability(&self) -> AudioProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_strategy_supports_replacement_and_capability_gaps() {
        let strategy = ConfiguredMediaAudioStrategy::with_commands(["audio.open_audio"]);
        assert!(strategy.validate_command("audio.open_audio").is_ok());
        assert!(strategy.validate_command("audio.export_request").is_err());
        assert_eq!(
            ConfiguredMediaAudioStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
