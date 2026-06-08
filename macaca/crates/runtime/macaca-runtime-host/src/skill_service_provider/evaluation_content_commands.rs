//! Self-evolution evaluation and content mutation command handlers.
//!
//! Evaluation commands are pure transformations over typed records.  Content
//! mutation delegates to `LocalSkillContentMutationStrategy` for filesystem IO.

use macaca_proto::{ServiceCallResult, ServiceCommand, ServiceResult, TraceContext};
use macaca_skill::{
    SelfEvolutionReportBuilder, SelfEvolutionScoringPolicy, SkillContentMutationCommand,
    SkillEvaluationCheckpointAppendCommand, SkillEvaluationCheckpointAppendResult,
    SkillEvaluationReportCommand, SkillEvaluationReportResult, SkillEvaluationScoreCommand,
    SkillEvaluationScoreResult,
};

use crate::skill_service_codec::{decode, service_result, to_value};

use super::SkillSystemServiceProvider;

impl SkillSystemServiceProvider {
    /// Append a bounded evaluation checkpoint without persisting raw skill bodies.
    pub(crate) async fn handle_evaluation_checkpoint_append(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillEvaluationCheckpointAppendCommand = decode(command.payload)?;
        let result = SkillEvaluationCheckpointAppendResult::from_command(&typed);
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            evaluation_id = %result.record.evaluation_id,
            task_family_id = %result.record.task_family_id,
            checkpoint_kind = ?typed.checkpoint.kind,
            checkpoint_count = result.checkpoint_count,
            audit_event_count = result.audit_event_count,
            "skill self-evolution evaluation checkpoint appended"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Score a self-evolution evaluation record using the shared scoring policy.
    pub(crate) async fn handle_evaluation_score(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillEvaluationScoreCommand = decode(command.payload)?;
        let score = SelfEvolutionScoringPolicy::score(&typed.record);
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            evaluation_id = %typed.record.evaluation_id,
            task_family_id = %typed.record.task_family_id,
            passed = score.passed,
            reason_count = score.reason_codes.len(),
            "skill self-evolution evaluation scored"
        );
        Ok(service_result(
            to_value(SkillEvaluationScoreResult { score })?,
            typed.trace,
        ))
    }

    /// Build JSON and optional markdown evaluation reports from scored records.
    pub(crate) async fn handle_evaluation_report(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillEvaluationReportCommand = decode(command.payload)?;
        let result = SkillEvaluationReportResult {
            score: typed.score.clone(),
            json_report: SelfEvolutionReportBuilder::json_summary(&typed.record, &typed.score),
            markdown_report: typed.include_markdown.then(|| {
                SelfEvolutionReportBuilder::markdown_summary(&typed.record, &typed.score)
            }),
        };
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            evaluation_id = %typed.record.evaluation_id,
            task_family_id = %typed.record.task_family_id,
            include_markdown = typed.include_markdown,
            passed = typed.score.passed,
            "skill self-evolution evaluation report built"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Apply a bounded content mutation through the injected mutation strategy.
    pub(crate) async fn handle_content_mutate(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillContentMutationCommand = decode(command.payload)?;
        let result = self.content_mutation.apply(typed.clone()).await?;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            skill_id = %typed.skill_id,
            relative_path = %typed.relative_path.display(),
            mutation_kind = ?typed.kind,
            status = ?result.status,
            mutated = result.mutated,
            evidence_refs = result.evidence_ids.len(),
            policy_decision_refs = result.policy_decision_refs.len(),
            "skill content mutation command completed"
        );
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Acknowledge a provider-local cleanup request (no durable state mutation).
    pub(crate) async fn handle_cleanup(
        &self,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        tracing::info!(trace_id = %trace.trace_id, "skill service cleanup completed");
        Ok(service_result(
            serde_json::json!({"status": "cleaned"}),
            trace,
        ))
    }
}
