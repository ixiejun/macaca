//! Provider-neutral Strategy boundary for transcription adapters.

use std::collections::BTreeSet;

use macaca_proto::media_transcription::TranscriptionProviderCapability;
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};

pub trait MediaTranscriptionStrategy: Send + Sync {
    fn validate(&self, operation: &str) -> ServiceResult<()>;
    fn capability(&self) -> TranscriptionProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredMediaTranscriptionStrategy {
    supported_operations: BTreeSet<String>,
    capability: TranscriptionProviderCapability,
}

impl ConfiguredMediaTranscriptionStrategy {
    pub fn mock() -> Self {
        Self {
            supported_operations: macaca_proto::media_transcription::MEDIA_TRANSCRIPTION_COMMANDS
                .iter()
                .map(|command| (*command).to_owned())
                .collect(),
            capability: TranscriptionProviderCapability {
                provider_class: "mock".into(),
                languages: BTreeSet::from(["und".into()]),
                model_classes: BTreeSet::from(["provider-neutral".into()]),
                features: BTreeSet::from(["metadata_only".into()]),
                state: DomainPackProviderCapabilityState::Preview,
            },
        }
    }

    pub fn unavailable() -> Self {
        let mut strategy = Self::mock();
        strategy.capability.provider_class = "unavailable".into();
        strategy.capability.features.clear();
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl MediaTranscriptionStrategy for ConfiguredMediaTranscriptionStrategy {
    fn validate(&self, operation: &str) -> ServiceResult<()> {
        self.supported_operations
            .contains(operation)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand(operation.into()))
    }

    fn capability(&self) -> TranscriptionProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_is_provider_name_agnostic_and_supports_capability_gaps() {
        let strategy = ConfiguredMediaTranscriptionStrategy::mock();
        assert!(strategy.validate("transcription.plan_batch").is_ok());
        assert!(strategy.validate("transcription.unknown").is_err());
        assert_eq!(strategy.capability().provider_class, "mock");
        assert_eq!(
            ConfiguredMediaTranscriptionStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
