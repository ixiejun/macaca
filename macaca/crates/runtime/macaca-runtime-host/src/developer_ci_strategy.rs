//! Strategy boundary for replaceable CI provider adapters.
use macaca_proto::domain_pack_contract::developer_ci::{
    CiProviderCapability, DEVELOPER_CI_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};
use std::collections::BTreeSet;
pub trait DeveloperCiProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> CiProviderCapability;
}
#[derive(Debug, Clone)]
pub struct ConfiguredDeveloperCiStrategy {
    commands: BTreeSet<String>,
    capability: CiProviderCapability,
}
impl ConfiguredDeveloperCiStrategy {
    pub fn mock() -> Self {
        Self::with_commands(DEVELOPER_CI_COMMANDS.iter().copied())
    }
    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands = commands
            .into_iter()
            .map(Into::into)
            .collect::<BTreeSet<_>>();
        Self {
            capability: CiProviderCapability {
                provider_class: "mock".into(),
                supports_trigger: true,
                supports_cancel: true,
                supports_rerun: true,
                state: DomainPackProviderCapabilityState::Preview,
            },
            commands,
        }
    }
    pub fn unavailable() -> Self {
        let mut s = Self::with_commands(std::iter::empty::<String>());
        s.capability.provider_class = "unavailable".into();
        s.capability.state = DomainPackProviderCapabilityState::Unavailable;
        s
    }
}
impl DeveloperCiProviderStrategy for ConfiguredDeveloperCiStrategy {
    fn validate_command(&self, c: &str) -> ServiceResult<()> {
        self.commands
            .contains(c)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("ci_command_unsupported".into()))
    }
    fn capability(&self) -> CiProviderCapability {
        self.capability.clone()
    }
}
