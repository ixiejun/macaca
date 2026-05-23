//! Runtime-backed implementation of the SDK Skill facade.
//!
//! Keeping the service-backed adapter in its own file prevents the public
//! `skill_client` contract from becoming a large mixed-responsibility module.

use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use macaca_skill::{
    SkillAliasResolveCommand, SkillAliasResolveResult, SkillAliasSnapshotCommand,
    SkillAliasSnapshotResult, SkillAliasUpsertCommand, SkillAliasUpsertResult, SkillCleanupCommand,
    SkillCurationDryRunCommand, SkillCurationDryRunResult, SkillCurationLifecycleAction,
    SkillCurationLifecycleCommand, SkillCurationLifecycleResult, SkillCurationRollbackCommand,
    SkillCurationRollbackResult, SkillCurationRunCommand, SkillCurationRunResult,
    SkillCurationSnapshotCommand, SkillCurationSnapshotResult, SkillEvaluationReportCommand,
    SkillEvaluationReportResult, SkillEvaluationScoreCommand, SkillEvaluationScoreResult,
    SkillEvolutionPromoteDraftCommand, SkillEvolutionPromoteDraftResult,
    SkillEvolutionProposePatchCommand, SkillEvolutionProposePatchResult,
    SkillEvolutionRejectDraftCommand, SkillEvolutionRejectDraftResult, SkillExecutableLoadCommand,
    SkillExecutableLoadResult, SkillExperienceProposalCommand, SkillExperienceProposalResult,
    SkillExperienceProposalSnapshotCommand, SkillExperienceProposalSnapshotResult,
    SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotResult, SkillServiceSnapshot,
    SkillServiceSnapshotCommand, SkillSnapshotServiceCommand, SkillSnapshotServiceResult,
    SkillStatusCommand, SkillStatusResult, SkillToolCatalogCommand, SkillToolCatalogResult,
    SkillToolInvokeCommand, SkillToolInvokeResult, SKILL_ALIAS_RESOLVE_COMMAND,
    SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_ALIAS_UPSERT_COMMAND, SKILL_CLEANUP_COMMAND,
    SKILL_CURATION_ARCHIVE_COMMAND, SKILL_CURATION_DRY_RUN_COMMAND, SKILL_CURATION_PIN_COMMAND,
    SKILL_CURATION_QUARANTINE_COMMAND, SKILL_CURATION_REJECT_COMMAND,
    SKILL_CURATION_RELEASE_QUARANTINE_COMMAND, SKILL_CURATION_RESTORE_COMMAND,
    SKILL_CURATION_ROLLBACK_COMMAND, SKILL_CURATION_RUN_COMMAND, SKILL_CURATION_SNAPSHOT_COMMAND,
    SKILL_CURATION_UNPIN_COMMAND, SKILL_EVALUATION_REPORT_COMMAND, SKILL_EVALUATION_SCORE_COMMAND,
    SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
    SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND, SKILL_EVOLUTION_REJECT_DRAFT_COMMAND,
    SKILL_EVOLUTION_SNAPSHOT_COMMAND, SKILL_EXECUTABLE_LOAD_COMMAND,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND, SKILL_SERVICE_ID,
    SKILL_SERVICE_SNAPSHOT_COMMAND, SKILL_SNAPSHOT_COMMAND, SKILL_STATUS_COMMAND,
    SKILL_TOOL_CATALOG_COMMAND, SKILL_TOOL_INVOKE_COMMAND,
};

use crate::service_client::ServiceCallCommand;
use crate::skill_client::{ServiceBackedSkillClient, SystemSkillClient};

#[async_trait]
impl SystemSkillClient for ServiceBackedSkillClient {
    async fn snapshot(
        &self,
        command: SkillSnapshotServiceCommand,
    ) -> MacacaResult<SkillSnapshotServiceResult> {
        call(
            &self.service,
            SKILL_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn load_executable(
        &self,
        command: SkillExecutableLoadCommand,
    ) -> MacacaResult<SkillExecutableLoadResult> {
        call(
            &self.service,
            SKILL_EXECUTABLE_LOAD_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn tool_catalog(
        &self,
        command: SkillToolCatalogCommand,
    ) -> MacacaResult<SkillToolCatalogResult> {
        call(
            &self.service,
            SKILL_TOOL_CATALOG_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn invoke_tool(
        &self,
        command: SkillToolInvokeCommand,
    ) -> MacacaResult<SkillToolInvokeResult> {
        call(
            &self.service,
            SKILL_TOOL_INVOKE_COMMAND,
            command.invocation.trace.clone(),
            command,
        )
        .await
    }

    async fn status(&self, command: SkillStatusCommand) -> MacacaResult<SkillStatusResult> {
        call(
            &self.service,
            SKILL_STATUS_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn service_snapshot(
        &self,
        command: SkillServiceSnapshotCommand,
    ) -> MacacaResult<SkillServiceSnapshot> {
        call(
            &self.service,
            SKILL_SERVICE_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn record_governance_usage(
        &self,
        command: SkillGovernanceRecordUsageCommand,
    ) -> MacacaResult<SkillGovernanceRecordUsageResult> {
        call(
            &self.service,
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn governance_snapshot(
        &self,
        command: SkillGovernanceSnapshotCommand,
    ) -> MacacaResult<SkillGovernanceSnapshotResult> {
        call(
            &self.service,
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn curation_dry_run(
        &self,
        command: SkillCurationDryRunCommand,
    ) -> MacacaResult<SkillCurationDryRunResult> {
        call(
            &self.service,
            SKILL_CURATION_DRY_RUN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn curation_run(
        &self,
        command: SkillCurationRunCommand,
    ) -> MacacaResult<SkillCurationRunResult> {
        call(
            &self.service,
            SKILL_CURATION_RUN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn curation_snapshot(
        &self,
        command: SkillCurationSnapshotCommand,
    ) -> MacacaResult<SkillCurationSnapshotResult> {
        call(
            &self.service,
            SKILL_CURATION_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn curation_rollback(
        &self,
        command: SkillCurationRollbackCommand,
    ) -> MacacaResult<SkillCurationRollbackResult> {
        call(
            &self.service,
            SKILL_CURATION_ROLLBACK_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn curation_lifecycle(
        &self,
        action: SkillCurationLifecycleAction,
        command: SkillCurationLifecycleCommand,
    ) -> MacacaResult<SkillCurationLifecycleResult> {
        let command_name = curation_lifecycle_command_name(&action)?;
        call(&self.service, command_name, command.trace.clone(), command).await
    }

    async fn alias_upsert(
        &self,
        command: SkillAliasUpsertCommand,
    ) -> MacacaResult<SkillAliasUpsertResult> {
        call(
            &self.service,
            SKILL_ALIAS_UPSERT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn alias_resolve(
        &self,
        command: SkillAliasResolveCommand,
    ) -> MacacaResult<SkillAliasResolveResult> {
        call(
            &self.service,
            SKILL_ALIAS_RESOLVE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn alias_snapshot(
        &self,
        command: SkillAliasSnapshotCommand,
    ) -> MacacaResult<SkillAliasSnapshotResult> {
        call(
            &self.service,
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn propose_skill_experience(
        &self,
        command: SkillExperienceProposalCommand,
    ) -> MacacaResult<SkillExperienceProposalResult> {
        call(
            &self.service,
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn propose_skill_patch(
        &self,
        command: SkillEvolutionProposePatchCommand,
    ) -> MacacaResult<SkillEvolutionProposePatchResult> {
        call(
            &self.service,
            SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn promote_skill_draft(
        &self,
        command: SkillEvolutionPromoteDraftCommand,
    ) -> MacacaResult<SkillEvolutionPromoteDraftResult> {
        call(
            &self.service,
            SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn reject_skill_draft(
        &self,
        command: SkillEvolutionRejectDraftCommand,
    ) -> MacacaResult<SkillEvolutionRejectDraftResult> {
        call(
            &self.service,
            SKILL_EVOLUTION_REJECT_DRAFT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn skill_experience_snapshot(
        &self,
        command: SkillExperienceProposalSnapshotCommand,
    ) -> MacacaResult<SkillExperienceProposalSnapshotResult> {
        call(
            &self.service,
            SKILL_EVOLUTION_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn evaluate_self_evolution(
        &self,
        command: SkillEvaluationScoreCommand,
    ) -> MacacaResult<SkillEvaluationScoreResult> {
        call(
            &self.service,
            SKILL_EVALUATION_SCORE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn self_evolution_evaluation_report(
        &self,
        command: SkillEvaluationReportCommand,
    ) -> MacacaResult<SkillEvaluationReportResult> {
        call(
            &self.service,
            SKILL_EVALUATION_REPORT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn cleanup(&self, command: SkillCleanupCommand) -> MacacaResult<serde_json::Value> {
        call(
            &self.service,
            SKILL_CLEANUP_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

fn curation_lifecycle_command_name(
    action: &SkillCurationLifecycleAction,
) -> MacacaResult<&'static str> {
    Ok(match action {
        SkillCurationLifecycleAction::Pin => SKILL_CURATION_PIN_COMMAND,
        SkillCurationLifecycleAction::Unpin => SKILL_CURATION_UNPIN_COMMAND,
        SkillCurationLifecycleAction::Archive => SKILL_CURATION_ARCHIVE_COMMAND,
        SkillCurationLifecycleAction::Restore => SKILL_CURATION_RESTORE_COMMAND,
        SkillCurationLifecycleAction::Quarantine => SKILL_CURATION_QUARANTINE_COMMAND,
        SkillCurationLifecycleAction::ReleaseQuarantine => {
            SKILL_CURATION_RELEASE_QUARANTINE_COMMAND
        }
        SkillCurationLifecycleAction::Supersede => {
            return Err(MacacaError::Config(
                "supersede requires a SkillCurationSupersedeCommand with alias evidence".into(),
            ));
        }
        SkillCurationLifecycleAction::Reject => SKILL_CURATION_REJECT_COMMAND,
    })
}

async fn call<T, R>(
    service: &std::sync::Arc<dyn crate::service_client::SystemServiceClient>,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let service_command = ServiceCallCommand::new(
        SKILL_SERVICE_ID,
        command_name,
        serde_json::to_value(payload)?,
    )?
    .with_trace(trace);
    let result = service.call_service(&service_command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
