//! Strategy boundary for replaceable terminal/process providers.
use macaca_proto::domain_pack_contract::developer_terminal::DEVELOPER_TERMINAL_COMMANDS;
use macaca_proto::{ServiceError, ServiceResult};
use std::collections::BTreeSet;
pub trait DeveloperTerminalProviderStrategy: Send + Sync {
    fn validate_command(&self, c: &str) -> ServiceResult<()>;
    fn provider_class(&self) -> &'static str;
}
#[derive(Debug, Clone)]
pub struct ConfiguredDeveloperTerminalStrategy {
    commands: BTreeSet<String>,
    provider_class: &'static str,
}
impl ConfiguredDeveloperTerminalStrategy {
    pub fn mock() -> Self {
        Self::with_commands(DEVELOPER_TERMINAL_COMMANDS.iter().copied())
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
impl DeveloperTerminalProviderStrategy for ConfiguredDeveloperTerminalStrategy {
    fn validate_command(&self, c: &str) -> ServiceResult<()> {
        self.commands
            .contains(c)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("terminal_command_unsupported".into()))
    }
    fn provider_class(&self) -> &'static str {
        self.provider_class
    }
}
