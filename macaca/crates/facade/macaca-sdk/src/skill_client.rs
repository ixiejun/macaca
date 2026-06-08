//! SDK Skill client facade (**Facade** module root) for Route C S6.
//!
//! Skill service contracts live in `macaca-skill` because the Skill runtime-host
//! owns executable materialization, governance, and evolution lanes.  The SDK
//! remains a thin client: it does not construct Skill runtimes or load SKILL.md
//! files directly.
//!
//! # Module tree (P5 iteration 95)
//! - `support` — generic service-call bridge (`call`, lifecycle command routing)
//! - `unavailable` — Null Object client for composition roots without Skill runtime
//! - `service_backed` — Adapter over `SystemServiceClient`
//! - `tests` — contract tests for unavailable vs service-backed boundary

mod service_backed;
mod support;
mod unavailable;

#[cfg(test)]
mod tests;

use async_trait::async_trait;
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
};
use macaca_proto::MacacaResult;

pub use service_backed::ServiceBackedSkillClient;
pub use unavailable::UnavailableSystemSkillClient;

/// Focused Skill client consumed by Web, CLI, framework, and applications.
///
/// The trait is the stable SDK boundary.  Implementations use either the Null
/// Object (`UnavailableSystemSkillClient`) or the service Adapter
/// (`ServiceBackedSkillClient`) depending on composition-root wiring.
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
