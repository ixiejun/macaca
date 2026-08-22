//! Strategy boundary for replaceable code-intelligence provider adapters.
use std::collections::BTreeSet;

use macaca_proto::domain_pack_contract::developer_code::DEVELOPER_CODE_COMMANDS;
use macaca_proto::{ServiceError, ServiceResult};

pub trait DeveloperCodeProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn provider_class(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct ConfiguredDeveloperCodeStrategy {
    commands: BTreeSet<String>,
    provider_class: &'static str,
}

impl ConfiguredDeveloperCodeStrategy {
    pub fn mock() -> Self {
        Self::with_commands(DEVELOPER_CODE_COMMANDS.iter().copied())
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

impl DeveloperCodeProviderStrategy for ConfiguredDeveloperCodeStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("code_command_unsupported".into()))
    }

    fn provider_class(&self) -> &'static str {
        self.provider_class
    }
}
