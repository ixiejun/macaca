//! Forward bounded Skill proposal commands through the SDK facade (best-effort).

use std::sync::Arc;

use macaca_proto::AgentExecutionResult;
use macaca_runtime_host::executor::TaskResult;
use macaca_sdk::skill::SkillExperienceProposalCommand;

use crate::state::AppState;

use super::types::SkillSelfEvolutionObservation;

/// Forward a sanitized Skill proposal command and return bounded audit state.
///
/// The observer is best-effort: the original Agent Execution result has already
/// been produced, so missing Skill service providers, policy rejections, or store
/// failures are logged and surfaced as observer status without changing the task
/// success/failure result.
pub(crate) async fn forward_skill_experience_proposal_command(
    state: &Arc<AppState>,
    command: SkillExperienceProposalCommand,
    task_result: &TaskResult,
    source_result: &AgentExecutionResult,
) -> Option<SkillSelfEvolutionObservation> {
    tracing::info!(
        trace_id = %command.trace.trace_id,
        task_id = %command.candidate.task_id,
        session_id = %source_result.session_id,
        agent = %source_result.target_agent,
        artifact_count = task_result.artifacts.len(),
        "Forwarding verified Agent Execution completion to Skill self-evolution service"
    );

    match state.skill_client.propose_skill_experience(command).await {
        Ok(result) => {
            let proposal_id = result.proposal.proposal_id.clone();
            tracing::info!(
                proposal_id = %result.proposal.proposal_id,
                mutated = result.mutated,
                destination = ?result.proposal.destination,
                route_status = %result.destination_route.status.as_str(),
                "Skill self-evolution proposal recorded from Agent Execution completion"
            );
            Some(SkillSelfEvolutionObservation {
                status: "proposal_created",
                task_id: Some(result.proposal.task_id),
                proposal_id: Some(proposal_id),
                reason: None,
            })
        }
        Err(error) => {
            tracing::warn!(
                task_id = %task_result.task_id,
                session_id = %source_result.session_id,
                error = %error,
                "Skill self-evolution proposal forwarding failed"
            );
            Some(SkillSelfEvolutionObservation {
                status: "proposal_failed",
                task_id: Some(task_result.task_id.to_string()),
                proposal_id: None,
                reason: Some("skill service rejected or failed the proposal command".into()),
            })
        }
    }
}
