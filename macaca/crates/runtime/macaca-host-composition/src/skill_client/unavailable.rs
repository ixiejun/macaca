//! Null-object Skill client used when no runtime-backed service is installed.
//!
//! The type follows the Null Object pattern: shells and tests can depend on
//! [`super::SystemSkillClient`] without branching on `Option`.  Read-only queries
//! return empty sanitized snapshots; mutating commands fail with a stable config
//! error so callers can surface a neutral unavailable state.

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
    SkillToolCatalogResult, SkillToolInvokeCommand, SkillToolInvokeResult, SKILL_SERVICE_ID,
};
use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use super::SystemSkillClient;

/// Null-object Skill client used when no runtime-backed service is installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemSkillClient;

#[async_trait]
impl SystemSkillClient for UnavailableSystemSkillClient {
    async fn snapshot(
        &self,
        command: SkillSnapshotServiceCommand,
    ) -> MacacaResult<SkillSnapshotServiceResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk skill client unavailable for snapshot");
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn load_executable(
        &self,
        command: SkillExecutableLoadCommand,
    ) -> MacacaResult<SkillExecutableLoadResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk skill client unavailable for executable load");
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn tool_catalog(
        &self,
        command: SkillToolCatalogCommand,
    ) -> MacacaResult<SkillToolCatalogResult> {
        info!(trace_id = %command.trace.trace_id, "sdk skill client returning empty catalog");
        Ok(SkillToolCatalogResult::new(Vec::new()))
    }

    async fn invoke_tool(
        &self,
        command: SkillToolInvokeCommand,
    ) -> MacacaResult<SkillToolInvokeResult> {
        warn!(
            trace_id = %command.invocation.trace.trace_id,
            tool = %command.invocation.tool_name,
            "sdk skill client unavailable for invocation"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn status(&self, command: SkillStatusCommand) -> MacacaResult<SkillStatusResult> {
        info!(trace_id = %command.trace.trace_id, "sdk skill client returning unavailable status");
        Ok(SkillStatusResult {
            service_id: SKILL_SERVICE_ID.into(),
            healthy: false,
            snapshot_skills: 0,
            executable_skills: 0,
            telemetry_aggregate: Default::default(),
            captured_at: chrono::Utc::now(),
        })
    }

    async fn service_snapshot(
        &self,
        command: SkillServiceSnapshotCommand,
    ) -> MacacaResult<SkillServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk skill client returning unavailable snapshot");
        Ok(SkillServiceSnapshot::unavailable(
            "runtime-backed Skill service is not installed",
        ))
    }

    async fn record_governance_usage(
        &self,
        command: SkillGovernanceRecordUsageCommand,
    ) -> MacacaResult<SkillGovernanceRecordUsageResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            skill_id = %command.observation.skill_id,
            "sdk skill client unavailable for governance usage recording"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn governance_snapshot(
        &self,
        command: SkillGovernanceSnapshotCommand,
    ) -> MacacaResult<SkillGovernanceSnapshotResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk skill client returning empty governance snapshot"
        );
        Ok(SkillGovernanceSnapshotResult {
            records: Vec::new(),
            telemetry_aggregate: Default::default(),
            captured_at: chrono::Utc::now(),
        })
    }

    async fn curation_dry_run(
        &self,
        command: SkillCurationDryRunCommand,
    ) -> MacacaResult<SkillCurationDryRunResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk skill client returning unavailable curation dry-run"
        );
        Ok(SkillCurationDryRunResult {
            recommendations: Vec::new(),
            semantic_analysis_status: "unavailable: Skill service is unavailable".into(),
            semantic_review: crate::runtime_host::SkillSemanticReviewResult::unavailable(
                chrono::Utc::now(),
            ),
            mutated: false,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn curation_run(
        &self,
        command: SkillCurationRunCommand,
    ) -> MacacaResult<SkillCurationRunResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            dry_run = command.dry_run,
            "sdk skill client unavailable for curation run"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn curation_snapshot(
        &self,
        command: SkillCurationSnapshotCommand,
    ) -> MacacaResult<SkillCurationSnapshotResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            "sdk skill client unavailable for curation snapshot"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn curation_rollback(
        &self,
        command: SkillCurationRollbackCommand,
    ) -> MacacaResult<SkillCurationRollbackResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            rollback_ref = %command.rollback_ref,
            "sdk skill client unavailable for curation rollback"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn curation_lifecycle(
        &self,
        action: SkillCurationLifecycleAction,
        command: SkillCurationLifecycleCommand,
    ) -> MacacaResult<SkillCurationLifecycleResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            skill_id = %command.skill_id,
            action = ?action,
            "sdk skill client unavailable for curation lifecycle command"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn alias_upsert(
        &self,
        command: SkillAliasUpsertCommand,
    ) -> MacacaResult<SkillAliasUpsertResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            source_skill_id = %command.record.source_skill_id,
            "sdk skill client unavailable for alias upsert"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn alias_resolve(
        &self,
        command: SkillAliasResolveCommand,
    ) -> MacacaResult<SkillAliasResolveResult> {
        info!(
            trace_id = %command.trace.trace_id,
            skill_id = %command.skill_id,
            "sdk skill client returning unresolved alias"
        );
        Ok(SkillAliasResolveResult::unresolved(
            &command,
            chrono::Utc::now(),
        ))
    }

    async fn alias_snapshot(
        &self,
        command: SkillAliasSnapshotCommand,
    ) -> MacacaResult<SkillAliasSnapshotResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk skill client returning empty alias snapshot"
        );
        Ok(SkillAliasSnapshotResult {
            aliases: Vec::new(),
            captured_at: chrono::Utc::now(),
        })
    }

    async fn propose_skill_experience(
        &self,
        command: SkillExperienceProposalCommand,
    ) -> MacacaResult<SkillExperienceProposalResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            task_id = %command.candidate.task_id,
            "sdk skill client unavailable for experience proposal"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn propose_skill_patch(
        &self,
        command: SkillEvolutionProposePatchCommand,
    ) -> MacacaResult<SkillEvolutionProposePatchResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            target_skill_id = ?command.candidate.target_skill_id,
            "sdk skill client unavailable for patch proposal"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn promote_skill_draft(
        &self,
        command: SkillEvolutionPromoteDraftCommand,
    ) -> MacacaResult<SkillEvolutionPromoteDraftResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            proposal_id = %command.proposal_id,
            "sdk skill client unavailable for draft promotion"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn reject_skill_draft(
        &self,
        command: SkillEvolutionRejectDraftCommand,
    ) -> MacacaResult<SkillEvolutionRejectDraftResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            proposal_id = %command.proposal_id,
            "sdk skill client unavailable for draft rejection"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn skill_experience_snapshot(
        &self,
        command: SkillExperienceProposalSnapshotCommand,
    ) -> MacacaResult<SkillExperienceProposalSnapshotResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk skill client returning empty experience proposal snapshot"
        );
        Ok(SkillExperienceProposalSnapshotResult {
            proposals: Vec::new(),
            mutated: false,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn process_skill_proposals(
        &self,
        command: SkillProposalProcessingRunCommand,
    ) -> MacacaResult<SkillProposalProcessingRunResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            dry_run = command.dry_run,
            "sdk skill client unavailable for proposal processing run"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn skill_proposal_processing_snapshot(
        &self,
        command: SkillProposalProcessingSnapshotCommand,
    ) -> MacacaResult<SkillProposalProcessingSnapshotResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk skill client returning empty proposal processing snapshot"
        );
        Ok(SkillProposalProcessingSnapshotResult {
            records: Vec::new(),
            state_counts: Default::default(),
            duplicate_group_count: 0,
            waiting_proposal_count: 0,
            mutated: false,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn run_autonomous_materialization(
        &self,
        command: SkillAutonomousMaterializationRunCommand,
    ) -> MacacaResult<SkillAutonomousMaterializationRunResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            dry_run = command.dry_run,
            "sdk skill client unavailable for autonomous materialization run"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn autonomous_materialization_snapshot(
        &self,
        command: SkillAutonomousMaterializationSnapshotCommand,
    ) -> MacacaResult<SkillAutonomousMaterializationSnapshotResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk skill client returning empty autonomous materialization snapshot"
        );
        Ok(SkillAutonomousMaterializationSnapshotResult {
            recent_runs: Vec::new(),
            status_counts: Default::default(),
            last_run_ref: None,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn append_self_evolution_checkpoint(
        &self,
        command: SkillEvaluationCheckpointAppendCommand,
    ) -> MacacaResult<SkillEvaluationCheckpointAppendResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            evaluation_id = %command.record.evaluation_id,
            "sdk skill client unavailable for self-evolution checkpoint append"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn evaluate_self_evolution(
        &self,
        command: SkillEvaluationScoreCommand,
    ) -> MacacaResult<SkillEvaluationScoreResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            evaluation_id = %command.record.evaluation_id,
            "sdk skill client unavailable for self-evolution evaluation scoring"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn self_evolution_evaluation_report(
        &self,
        command: SkillEvaluationReportCommand,
    ) -> MacacaResult<SkillEvaluationReportResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            evaluation_id = %command.record.evaluation_id,
            "sdk skill client unavailable for self-evolution evaluation report"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn cleanup(&self, command: SkillCleanupCommand) -> MacacaResult<serde_json::Value> {
        info!(trace_id = %command.trace.trace_id, "sdk skill client cleanup no-op");
        Ok(serde_json::json!({"status": "unavailable"}))
    }
}
