//! Task prompt preparation commands.
//!
//! These handlers keep prompt templates and parsing rules inside the Task
//! Service owner while exposing only typed protocol DTOs to shells and SDK
//! clients. The functions are pure: they do not call providers, mutate stores,
//! or inspect application-specific business data.

use macaca_proto::{
    BuildDecompositionPromptCommand, BuildGoalEvaluationPromptCommand, GoalEvaluationResult,
    ParseGoalEvaluationCommand, TaskPromptResult,
};
use tracing::info;

use crate::decompose::build_decomposition_prompt;
use crate::plan_loop::GoalEvaluator;

use super::TaskServiceRuntime;

impl<S> TaskServiceRuntime<S>
where
    S: super::TaskServiceExecutionStrategy + 'static,
{
    /// Build the canonical decomposition prompt from a provider-neutral
    /// application planning contract.
    pub async fn build_decomposition_prompt(
        &self,
        command: BuildDecompositionPromptCommand,
    ) -> Result<TaskPromptResult, String> {
        info!(
            workflow = %command.planning_contract.workflow_name,
            worker_count = command.planning_contract.worker_agents.len(),
            trace_id = command
                .trace
                .as_ref()
                .map(|trace| trace.trace_id.as_str())
                .unwrap_or("none"),
            "task service building decomposition prompt"
        );
        Ok(TaskPromptResult {
            prompt: build_decomposition_prompt(
                &command.goal_description,
                &command.planning_contract,
            ),
        })
    }

    /// Build the canonical goal evaluation prompt without running a model.
    pub async fn build_goal_evaluation_prompt(
        &self,
        command: BuildGoalEvaluationPromptCommand,
    ) -> Result<TaskPromptResult, String> {
        info!(
            completed = command.completed_count,
            failed = command.failed_count,
            summary_count = command.task_summaries.len(),
            trace_id = command
                .trace
                .as_ref()
                .map(|trace| trace.trace_id.as_str())
                .unwrap_or("none"),
            "task service building goal evaluation prompt"
        );
        Ok(TaskPromptResult {
            prompt: GoalEvaluator::build_prompt(
                &command.goal_description,
                &command.task_summaries,
                command.completed_count,
                command.failed_count,
            ),
        })
    }

    /// Parse a goal evaluation response into the protocol-owned result enum.
    pub async fn parse_goal_evaluation(
        &self,
        command: ParseGoalEvaluationCommand,
    ) -> Result<GoalEvaluationResult, String> {
        info!(
            content_len = command.content.len(),
            trace_id = command
                .trace
                .as_ref()
                .map(|trace| trace.trace_id.as_str())
                .unwrap_or("none"),
            "task service parsing goal evaluation response"
        );
        Ok(GoalEvaluator::parse_eval_response(&command.content))
    }
}
