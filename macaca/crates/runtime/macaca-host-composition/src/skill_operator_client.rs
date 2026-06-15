//! Host-owned Skill operator lifecycle client facade.
//!
//! Operator lifecycle commands remain tied to Skill runtime DTOs, so this
//! extension stays in host composition until those DTOs become pure protocol
//! contracts. The implementation still uses the generic SDK service-call
//! envelope, preserving trace and audit admission at the service boundary.

use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::runtime_host::{
    SkillOperatorCatalogListCommand, SkillOperatorCatalogListResult, SkillOperatorChangedCommand,
    SkillOperatorChangedResult, SkillOperatorConfigWriteCommand, SkillOperatorConfigWriteResult,
    SkillOperatorEnablementCommand, SkillOperatorEnablementResult,
    SkillOperatorMarkdownReadCommand, SkillOperatorMarkdownReadResult, SkillOperatorUnwatchCommand,
    SkillOperatorWatchCommand, SkillOperatorWatchResult, SKILL_OPERATOR_CATALOG_LIST_COMMAND,
    SKILL_OPERATOR_CHANGED_COMMAND, SKILL_OPERATOR_CONFIG_WRITE_COMMAND,
    SKILL_OPERATOR_ENABLEMENT_COMMAND, SKILL_OPERATOR_MARKDOWN_READ_COMMAND,
    SKILL_OPERATOR_UNWATCH_COMMAND, SKILL_OPERATOR_WATCH_COMMAND, SKILL_SERVICE_ID,
};
use crate::skill_client::{ServiceBackedSkillClient, UnavailableSystemSkillClient};

/// Extension trait for service-owned Skill operator lifecycle operations.
#[async_trait]
pub trait SystemSkillOperatorClient: Send + Sync {
    async fn operator_catalog_list(
        &self,
        command: SkillOperatorCatalogListCommand,
    ) -> MacacaResult<SkillOperatorCatalogListResult>;
    async fn operator_markdown_read(
        &self,
        command: SkillOperatorMarkdownReadCommand,
    ) -> MacacaResult<SkillOperatorMarkdownReadResult>;
    async fn operator_config_write(
        &self,
        command: SkillOperatorConfigWriteCommand,
    ) -> MacacaResult<SkillOperatorConfigWriteResult>;
    async fn operator_watch(
        &self,
        command: SkillOperatorWatchCommand,
    ) -> MacacaResult<SkillOperatorWatchResult>;
    async fn operator_unwatch(
        &self,
        command: SkillOperatorUnwatchCommand,
    ) -> MacacaResult<SkillOperatorWatchResult>;
    async fn operator_changed(
        &self,
        command: SkillOperatorChangedCommand,
    ) -> MacacaResult<SkillOperatorChangedResult>;
    async fn operator_enablement(
        &self,
        command: SkillOperatorEnablementCommand,
    ) -> MacacaResult<SkillOperatorEnablementResult>;
}

#[async_trait]
impl SystemSkillOperatorClient for UnavailableSystemSkillClient {
    async fn operator_catalog_list(
        &self,
        command: SkillOperatorCatalogListCommand,
    ) -> MacacaResult<SkillOperatorCatalogListResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "host skill operator client returning empty catalog"
        );
        Ok(SkillOperatorCatalogListResult {
            skills: Vec::new(),
            source_count: 0,
            skipped_shadowed: 0,
            visibility_generation: 0,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn operator_markdown_read(
        &self,
        command: SkillOperatorMarkdownReadCommand,
    ) -> MacacaResult<SkillOperatorMarkdownReadResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            skill_name = %command.skill_name,
            "host skill operator client unavailable for markdown read"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn operator_config_write(
        &self,
        command: SkillOperatorConfigWriteCommand,
    ) -> MacacaResult<SkillOperatorConfigWriteResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            skill_name = %command.skill_name,
            "host skill operator client unavailable for config write"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn operator_watch(
        &self,
        command: SkillOperatorWatchCommand,
    ) -> MacacaResult<SkillOperatorWatchResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            "host skill operator client unavailable for watch"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn operator_unwatch(
        &self,
        command: SkillOperatorUnwatchCommand,
    ) -> MacacaResult<SkillOperatorWatchResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            "host skill operator client unavailable for unwatch"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn operator_changed(
        &self,
        command: SkillOperatorChangedCommand,
    ) -> MacacaResult<SkillOperatorChangedResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            skill_name = %command.skill_name,
            "host skill operator client unavailable for changed notification"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn operator_enablement(
        &self,
        command: SkillOperatorEnablementCommand,
    ) -> MacacaResult<SkillOperatorEnablementResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            skill_name = %command.skill_name,
            "host skill operator client unavailable for enablement"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }
}

#[async_trait]
impl SystemSkillOperatorClient for ServiceBackedSkillClient {
    async fn operator_catalog_list(
        &self,
        command: SkillOperatorCatalogListCommand,
    ) -> MacacaResult<SkillOperatorCatalogListResult> {
        call(
            &self.service,
            SKILL_OPERATOR_CATALOG_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn operator_markdown_read(
        &self,
        command: SkillOperatorMarkdownReadCommand,
    ) -> MacacaResult<SkillOperatorMarkdownReadResult> {
        call(
            &self.service,
            SKILL_OPERATOR_MARKDOWN_READ_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn operator_config_write(
        &self,
        command: SkillOperatorConfigWriteCommand,
    ) -> MacacaResult<SkillOperatorConfigWriteResult> {
        call(
            &self.service,
            SKILL_OPERATOR_CONFIG_WRITE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn operator_watch(
        &self,
        command: SkillOperatorWatchCommand,
    ) -> MacacaResult<SkillOperatorWatchResult> {
        call(
            &self.service,
            SKILL_OPERATOR_WATCH_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn operator_unwatch(
        &self,
        command: SkillOperatorUnwatchCommand,
    ) -> MacacaResult<SkillOperatorWatchResult> {
        call(
            &self.service,
            SKILL_OPERATOR_UNWATCH_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn operator_changed(
        &self,
        command: SkillOperatorChangedCommand,
    ) -> MacacaResult<SkillOperatorChangedResult> {
        call(
            &self.service,
            SKILL_OPERATOR_CHANGED_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn operator_enablement(
        &self,
        command: SkillOperatorEnablementCommand,
    ) -> MacacaResult<SkillOperatorEnablementResult> {
        call(
            &self.service,
            SKILL_OPERATOR_ENABLEMENT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

async fn call<T, R>(
    service: &std::sync::Arc<dyn macaca_sdk::SystemServiceClient>,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    // Operator commands use the same generic envelope as normal Skill commands
    // so trace-required middleware and audit decorators observe one path.
    let service_command = macaca_sdk::ServiceCallCommand::new(
        SKILL_SERVICE_ID,
        command_name,
        serde_json::to_value(payload)?,
    )?
    .with_trace(trace);
    let result = service.call_service(&service_command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
