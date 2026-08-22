//! Strategy boundary for replaceable design-tool provider adapters.
use macaca_proto::domain_pack_contract::developer_design_tools::DEVELOPER_DESIGN_TOOLS_COMMANDS;
use macaca_proto::{ServiceError, ServiceResult};
use std::collections::BTreeSet;

pub trait DeveloperDesignToolsProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn provider_class(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct ConfiguredDeveloperDesignToolsStrategy {
    commands: BTreeSet<String>,
    provider_class: &'static str,
}

impl ConfiguredDeveloperDesignToolsStrategy {
    pub fn mock() -> Self {
        Self::with_commands(DEVELOPER_DESIGN_TOOLS_COMMANDS.iter().copied())
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

impl DeveloperDesignToolsProviderStrategy for ConfiguredDeveloperDesignToolsStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| {
                ServiceError::UnsupportedCommand("design_tools_command_unsupported".into())
            })
    }
    fn provider_class(&self) -> &'static str {
        self.provider_class
    }
}
