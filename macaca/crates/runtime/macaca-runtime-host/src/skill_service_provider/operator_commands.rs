//! Skill operator lifecycle command handlers.
//!
//! Operator commands are delegated to `SkillOperatorLifecycle`, keeping filesystem
//! watch semantics and markdown/config IO out of the provider adapter.

use macaca_proto::{ServiceCallResult, ServiceCommand, ServiceResult};
use macaca_skill::{
    SkillOperatorCatalogListCommand, SkillOperatorChangedCommand, SkillOperatorConfigWriteCommand,
    SkillOperatorEnablementCommand, SkillOperatorMarkdownReadCommand, SkillOperatorUnwatchCommand,
    SkillOperatorWatchCommand,
};

use crate::skill_service_codec::{decode, service_result, to_value};

use super::SkillSystemServiceProvider;

impl SkillSystemServiceProvider {
    /// List operator catalog entries visible to the current scope.
    pub(crate) async fn handle_operator_catalog_list(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillOperatorCatalogListCommand = decode(command.payload)?;
        let result = self.operator_lifecycle.catalog_list(typed.clone()).await?;
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Read operator markdown through the lifecycle strategy.
    pub(crate) async fn handle_operator_markdown_read(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillOperatorMarkdownReadCommand = decode(command.payload)?;
        let result = self.operator_lifecycle.markdown_read(typed.clone()).await?;
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Persist operator configuration through the lifecycle strategy.
    pub(crate) async fn handle_operator_config_write(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillOperatorConfigWriteCommand = decode(command.payload)?;
        let result = self.operator_lifecycle.config_write(typed.clone()).await?;
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Start watching an operator path for change notifications.
    pub(crate) async fn handle_operator_watch(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillOperatorWatchCommand = decode(command.payload)?;
        let result = self.operator_lifecycle.watch(typed.clone()).await;
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Stop watching an operator path.
    pub(crate) async fn handle_operator_unwatch(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillOperatorUnwatchCommand = decode(command.payload)?;
        let result = self.operator_lifecycle.unwatch(typed.clone()).await;
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Report operator change events detected by the lifecycle strategy.
    pub(crate) async fn handle_operator_changed(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillOperatorChangedCommand = decode(command.payload)?;
        let result = self.operator_lifecycle.changed(typed.clone()).await?;
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Toggle operator enablement without mutating governance records directly.
    pub(crate) async fn handle_operator_enablement(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillOperatorEnablementCommand = decode(command.payload)?;
        let result = self.operator_lifecycle.enablement(typed.clone()).await?;
        Ok(service_result(to_value(result)?, typed.trace))
    }
}
