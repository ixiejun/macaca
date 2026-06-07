//! Main Observer entry: Agent Execution result → Skill proposal side-effect.

use std::sync::Arc;

use macaca_proto::{AgentExecutionResult, AgentExecutionStatus};

use crate::state::AppState;

use super::forwarder::forward_skill_experience_proposal_command;
use super::projection::task_result_from_agent_execution_result;
use super::proposal_builder::build_skill_experience_proposal_command;
use super::types::SkillSelfEvolutionObservation;

pub(crate) async fn observe_agent_execution_result_for_skill_self_evolution(
    state: &Arc<AppState>,
    result: &AgentExecutionResult,
) -> Option<SkillSelfEvolutionObservation> {
    if result.status != AgentExecutionStatus::Completed {
        return Some(SkillSelfEvolutionObservation {
            status: "skipped_non_completed_agent_execution",
            task_id: result.task_id.map(|task_id| task_id.to_string()),
            proposal_id: None,
            reason: Some(format!(
                "agent execution status was {}",
                result.status.as_str()
            )),
        });
    }

    let task_result = task_result_from_agent_execution_result(result);
    let command = match build_skill_experience_proposal_command(
        &result.application_id,
        &result.session_id,
        &result.target_agent,
        &task_result,
        &result.trace.trace_id,
    ) {
        Some(command) => command,
        None => {
            return Some(SkillSelfEvolutionObservation {
                status: "skipped_missing_evidence",
                task_id: Some(task_result.task_id.to_string()),
                proposal_id: None,
                reason: Some("task completion had no bounded replay evidence".into()),
            });
        }
    };

    forward_skill_experience_proposal_command(state, command, &task_result, result).await
}
