//! Strategy boundary for replaceable repository provider adapters.
use macaca_proto::domain_pack_contract::developer_repository::DEVELOPER_REPOSITORY_COMMANDS;
use macaca_proto::{ServiceError, ServiceResult};
use std::collections::BTreeSet;
pub trait DeveloperRepositoryProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn provider_class(&self) -> &'static str;
}
#[derive(Debug, Clone)]
pub struct ConfiguredDeveloperRepositoryStrategy {
    commands: BTreeSet<String>,
    provider_class: &'static str,
}
impl ConfiguredDeveloperRepositoryStrategy {
    pub fn mock() -> Self {
        Self::with_commands(DEVELOPER_REPOSITORY_COMMANDS.iter().copied())
    }
    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            commands: commands.into_iter().map(Into::into).collect(),
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
impl DeveloperRepositoryProviderStrategy for ConfiguredDeveloperRepositoryStrategy {
    fn validate_command(&self, c: &str) -> ServiceResult<()> {
        self.commands.contains(c).then_some(()).ok_or_else(|| {
            ServiceError::UnsupportedCommand("repository_command_unsupported".into())
        })
    }
    fn provider_class(&self) -> &'static str {
        self.provider_class
    }
}
