//! Strategy boundary for replaceable browser automation adapters.
use macaca_proto::domain_pack_contract::developer_browser_automation::{
    BrowserProviderCapability, DEVELOPER_BROWSER_AUTOMATION_COMMANDS,
};
use macaca_proto::{DomainPackProviderCapabilityState, ServiceError, ServiceResult};
use std::collections::BTreeSet;

pub trait DeveloperBrowserAutomationProviderStrategy: Send + Sync {
    fn validate_command(&self, command: &str) -> ServiceResult<()>;
    fn capability(&self) -> BrowserProviderCapability;
}

#[derive(Debug, Clone)]
pub struct ConfiguredDeveloperBrowserAutomationStrategy {
    commands: BTreeSet<String>,
    capability: BrowserProviderCapability,
}

impl ConfiguredDeveloperBrowserAutomationStrategy {
    pub fn mock() -> Self {
        Self::with_commands(DEVELOPER_BROWSER_AUTOMATION_COMMANDS.iter().copied())
    }
    pub fn with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let commands: BTreeSet<String> = commands.into_iter().map(Into::into).collect();
        Self {
            capability: BrowserProviderCapability {
                provider_class: "mock".into(),
                supports_contexts: true,
                supports_actions: true,
                supports_artifacts: true,
                state: DomainPackProviderCapabilityState::Preview,
            },
            commands,
        }
    }
    pub fn unavailable() -> Self {
        let mut strategy = Self::with_commands(std::iter::empty::<String>());
        strategy.capability.provider_class = "unavailable".into();
        strategy.capability.supports_contexts = false;
        strategy.capability.supports_actions = false;
        strategy.capability.supports_artifacts = false;
        strategy.capability.state = DomainPackProviderCapabilityState::Unavailable;
        strategy
    }
}

impl DeveloperBrowserAutomationProviderStrategy for ConfiguredDeveloperBrowserAutomationStrategy {
    fn validate_command(&self, command: &str) -> ServiceResult<()> {
        self.commands
            .contains(command)
            .then_some(())
            .ok_or_else(|| ServiceError::UnsupportedCommand("browser_command_unsupported".into()))
    }
    fn capability(&self) -> BrowserProviderCapability {
        self.capability.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strategy_supports_command_gaps_and_unavailable() {
        let strategy =
            ConfiguredDeveloperBrowserAutomationStrategy::with_commands(["browser.open_page"]);
        assert!(strategy.validate_command("browser.open_page").is_ok());
        assert!(strategy
            .validate_command("browser.evaluate_request")
            .is_err());
        assert_eq!(
            ConfiguredDeveloperBrowserAutomationStrategy::unavailable()
                .capability()
                .state,
            DomainPackProviderCapabilityState::Unavailable
        );
    }
}
