//! Runtime-backed Skill client implemented over the generic SDK service client.
//!
//! This module is the Adapter half of the SDK Skill facade: it maps typed Skill
//! command DTOs onto `SystemServiceClient::call_service` without importing Skill
//! service internals.  The Skill runtime-host remains the sole owner of executable
//! skill materialization and governance state.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use macaca_skill::{
    SkillAliasResolveCommand, SkillAliasResolveResult, SkillAliasSnapshotCommand,
    SkillAliasSnapshotResult, SkillAliasUpsertCommand, SkillAliasUpsertResult,
    SkillAutonomousMaterializationRunCommand, SkillAutonomousMaterializationRunResult,
    SkillAutonomousMaterializationSnapshotCommand, SkillAutonomousMaterializationSnapshotResult,
    SkillCleanupCommand, SkillCurationDryRunCommand, SkillCurationDryRunResult,
    SkillCurationLifecycleAction, SkillCurationLifecycleCommand, SkillCurationLifecycleResult,
    SkillCurationRollbackCommand, SkillCurationRollbackResult, SkillCurationRunCommand,
    SkillCurationRunResult, SkillCurationSnapshotCommand, SkillCurationSnapshotResult,
    SkillEvaluationCheckpointAppendCommand, SkillEvaluationCheckpointAppendResult,
    SkillEvaluationReportCommand, SkillEvaluationReportResult, SkillEvaluationScoreCommand,
    SkillEvaluationScoreResult, SkillEvolutionPromoteDraftCommand,
    SkillEvolutionPromoteDraftResult, SkillEvolutionProposePatchCommand,
    SkillEvolutionProposePatchResult, SkillEvolutionRejectDraftCommand,
    SkillEvolutionRejectDraftResult, SkillExecutableLoadCommand, SkillExecutableLoadResult,
    SkillExperienceProposalCommand, SkillExperienceProposalResult,
    SkillExperienceProposalSnapshotCommand, SkillExperienceProposalSnapshotResult,
    SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotResult,
    SkillProposalProcessingRunCommand, SkillProposalProcessingRunResult,
    SkillProposalProcessingSnapshotCommand, SkillProposalProcessingSnapshotResult,
    SkillServiceSnapshot, SkillServiceSnapshotCommand, SkillSnapshotServiceCommand,
    SkillSnapshotServiceResult, SkillStatusCommand, SkillStatusResult, SkillToolCatalogCommand,
    SkillToolCatalogResult, SkillToolInvokeCommand, SkillToolInvokeResult,
    SKILL_ALIAS_RESOLVE_COMMAND, SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_ALIAS_UPSERT_COMMAND,
    SKILL_CLEANUP_COMMAND, SKILL_CURATION_DRY_RUN_COMMAND, SKILL_CURATION_ROLLBACK_COMMAND,
    SKILL_CURATION_RUN_COMMAND, SKILL_CURATION_SNAPSHOT_COMMAND,
    SKILL_EVALUATION_CHECKPOINT_APPEND_COMMAND, SKILL_EVALUATION_REPORT_COMMAND,
    SKILL_EVALUATION_SCORE_COMMAND, SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
    SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_SNAPSHOT_COMMAND,
    SKILL_EVOLUTION_PROCESSING_RUN_COMMAND, SKILL_EVOLUTION_PROCESSING_SNAPSHOT_COMMAND,
    SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
    SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND, SKILL_EVOLUTION_REJECT_DRAFT_COMMAND,
    SKILL_EVOLUTION_SNAPSHOT_COMMAND, SKILL_EXECUTABLE_LOAD_COMMAND,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
    SKILL_SERVICE_SNAPSHOT_COMMAND, SKILL_SNAPSHOT_COMMAND, SKILL_STATUS_COMMAND,
    SKILL_TOOL_CATALOG_COMMAND, SKILL_TOOL_INVOKE_COMMAND,
};

use crate::service_client::SystemServiceClient;

use super::support::{call, curation_lifecycle_command_name};
use super::SystemSkillClient;

/// Runtime-backed Skill client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedSkillClient {
    /// Shared generic service client used by sibling SDK modules (e.g. operator extension).
    pub(crate) service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedSkillClient {
    /// Create a service-backed client from an existing generic service client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

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

    async fn process_skill_proposals(
        &self,
        command: SkillProposalProcessingRunCommand,
    ) -> MacacaResult<SkillProposalProcessingRunResult> {
        call(
            &self.service,
            SKILL_EVOLUTION_PROCESSING_RUN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn skill_proposal_processing_snapshot(
        &self,
        command: SkillProposalProcessingSnapshotCommand,
    ) -> MacacaResult<SkillProposalProcessingSnapshotResult> {
        call(
            &self.service,
            SKILL_EVOLUTION_PROCESSING_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn run_autonomous_materialization(
        &self,
        command: SkillAutonomousMaterializationRunCommand,
    ) -> MacacaResult<SkillAutonomousMaterializationRunResult> {
        call(
            &self.service,
            SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn autonomous_materialization_snapshot(
        &self,
        command: SkillAutonomousMaterializationSnapshotCommand,
    ) -> MacacaResult<SkillAutonomousMaterializationSnapshotResult> {
        call(
            &self.service,
            SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn append_self_evolution_checkpoint(
        &self,
        command: SkillEvaluationCheckpointAppendCommand,
    ) -> MacacaResult<SkillEvaluationCheckpointAppendResult> {
        call(
            &self.service,
            SKILL_EVALUATION_CHECKPOINT_APPEND_COMMAND,
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

