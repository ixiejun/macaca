//! Host-owned Skill service client facade.
//!
//! Skill command DTOs still include runtime-host governance and materialization
//! types, so this Facade belongs in host composition until those DTOs are moved
//! into `macaca-proto`. The implementations remain thin Adapters over the
//! generic SDK `SystemServiceClient` and preserve trace-bearing service calls.

mod service_backed;
mod support;
mod unavailable;

pub use service_backed::ServiceBackedSkillClient;
pub use unavailable::UnavailableSystemSkillClient;

use async_trait::async_trait;
use macaca_proto::MacacaResult;

use crate::runtime_host::{
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
};

/// Focused Skill client consumed by Web and host composition adapters.
#[async_trait]
pub trait SystemSkillClient: Send + Sync {
    async fn snapshot(
        &self,
        command: SkillSnapshotServiceCommand,
    ) -> MacacaResult<SkillSnapshotServiceResult>;
    async fn load_executable(
        &self,
        command: SkillExecutableLoadCommand,
    ) -> MacacaResult<SkillExecutableLoadResult>;
    async fn tool_catalog(
        &self,
        command: SkillToolCatalogCommand,
    ) -> MacacaResult<SkillToolCatalogResult>;
    async fn invoke_tool(
        &self,
        command: SkillToolInvokeCommand,
    ) -> MacacaResult<SkillToolInvokeResult>;
    async fn status(&self, command: SkillStatusCommand) -> MacacaResult<SkillStatusResult>;
    async fn service_snapshot(
        &self,
        command: SkillServiceSnapshotCommand,
    ) -> MacacaResult<SkillServiceSnapshot>;
    async fn record_governance_usage(
        &self,
        command: SkillGovernanceRecordUsageCommand,
    ) -> MacacaResult<SkillGovernanceRecordUsageResult>;
    async fn governance_snapshot(
        &self,
        command: SkillGovernanceSnapshotCommand,
    ) -> MacacaResult<SkillGovernanceSnapshotResult>;
    async fn curation_dry_run(
        &self,
        command: SkillCurationDryRunCommand,
    ) -> MacacaResult<SkillCurationDryRunResult>;
    async fn curation_run(
        &self,
        command: SkillCurationRunCommand,
    ) -> MacacaResult<SkillCurationRunResult>;
    async fn curation_snapshot(
        &self,
        command: SkillCurationSnapshotCommand,
    ) -> MacacaResult<SkillCurationSnapshotResult>;
    async fn curation_rollback(
        &self,
        command: SkillCurationRollbackCommand,
    ) -> MacacaResult<SkillCurationRollbackResult>;
    async fn curation_lifecycle(
        &self,
        action: SkillCurationLifecycleAction,
        command: SkillCurationLifecycleCommand,
    ) -> MacacaResult<SkillCurationLifecycleResult>;
    async fn alias_upsert(
        &self,
        command: SkillAliasUpsertCommand,
    ) -> MacacaResult<SkillAliasUpsertResult>;
    async fn alias_resolve(
        &self,
        command: SkillAliasResolveCommand,
    ) -> MacacaResult<SkillAliasResolveResult>;
    async fn alias_snapshot(
        &self,
        command: SkillAliasSnapshotCommand,
    ) -> MacacaResult<SkillAliasSnapshotResult>;
    async fn propose_skill_experience(
        &self,
        command: SkillExperienceProposalCommand,
    ) -> MacacaResult<SkillExperienceProposalResult>;
    async fn propose_skill_patch(
        &self,
        command: SkillEvolutionProposePatchCommand,
    ) -> MacacaResult<SkillEvolutionProposePatchResult>;
    async fn promote_skill_draft(
        &self,
        command: SkillEvolutionPromoteDraftCommand,
    ) -> MacacaResult<SkillEvolutionPromoteDraftResult>;
    async fn reject_skill_draft(
        &self,
        command: SkillEvolutionRejectDraftCommand,
    ) -> MacacaResult<SkillEvolutionRejectDraftResult>;
    async fn skill_experience_snapshot(
        &self,
        command: SkillExperienceProposalSnapshotCommand,
    ) -> MacacaResult<SkillExperienceProposalSnapshotResult>;
    async fn process_skill_proposals(
        &self,
        command: SkillProposalProcessingRunCommand,
    ) -> MacacaResult<SkillProposalProcessingRunResult>;
    async fn skill_proposal_processing_snapshot(
        &self,
        command: SkillProposalProcessingSnapshotCommand,
    ) -> MacacaResult<SkillProposalProcessingSnapshotResult>;
    async fn run_autonomous_materialization(
        &self,
        command: SkillAutonomousMaterializationRunCommand,
    ) -> MacacaResult<SkillAutonomousMaterializationRunResult>;
    async fn autonomous_materialization_snapshot(
        &self,
        command: SkillAutonomousMaterializationSnapshotCommand,
    ) -> MacacaResult<SkillAutonomousMaterializationSnapshotResult>;
    async fn append_self_evolution_checkpoint(
        &self,
        command: SkillEvaluationCheckpointAppendCommand,
    ) -> MacacaResult<SkillEvaluationCheckpointAppendResult>;
    async fn evaluate_self_evolution(
        &self,
        command: SkillEvaluationScoreCommand,
    ) -> MacacaResult<SkillEvaluationScoreResult>;
    async fn self_evolution_evaluation_report(
        &self,
        command: SkillEvaluationReportCommand,
    ) -> MacacaResult<SkillEvaluationReportResult>;
    async fn cleanup(&self, command: SkillCleanupCommand) -> MacacaResult<serde_json::Value>;
}
