//! Strategy boundary for replaceable issue-tracker provider adapters.
use macaca_proto::domain_pack_contract::developer_issue_tracker::DEVELOPER_ISSUE_TRACKER_COMMANDS;
use macaca_proto::{ServiceError, ServiceResult};
use std::collections::BTreeSet;

pub trait DeveloperIssueTrackerProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn provider_class(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct ConfiguredDeveloperIssueTrackerStrategy {
    commands: BTreeSet<String>,
    provider_class: &'static str,
}
impl ConfiguredDeveloperIssueTrackerStrategy {
    pub fn mock() -> Self {
        Self::with_commands(DEVELOPER_ISSUE_TRACKER_COMMANDS.iter().copied())
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
impl DeveloperIssueTrackerProviderStrategy for ConfiguredDeveloperIssueTrackerStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| {
                ServiceError::UnsupportedCommand("issue_tracker_command_unsupported".into())
            })
    }
    fn provider_class(&self) -> &'static str {
        self.provider_class
    }
}
