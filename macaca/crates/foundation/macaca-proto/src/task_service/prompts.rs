//! Task Service prompt and evaluation DTOs.

use serde::{Deserialize, Serialize};

use crate::{ApplicationTaskPlanningContract, TraceContext};

use super::TaskSummary;

/// Command to build the canonical goal decomposition prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildDecompositionPromptCommand {
    pub goal_description: String,
    pub planning_contract: ApplicationTaskPlanningContract,
    pub trace: Option<TraceContext>,
}

/// Prompt result returned by task prompt-preparation commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPromptResult {
    pub prompt: String,
}

/// Command to build the canonical goal evaluation prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildGoalEvaluationPromptCommand {
    pub goal_description: String,
    #[serde(default)]
    pub task_summaries: Vec<TaskSummary>,
    pub completed_count: usize,
    pub failed_count: usize,
    pub trace: Option<TraceContext>,
}

/// Command to parse a planner/model goal evaluation response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseGoalEvaluationCommand {
    pub content: String,
    pub trace: Option<TraceContext>,
}

/// Parsed goal evaluation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalEvaluationResult {
    Satisfied {
        summary: String,
    },
    NeedsMoreWork {
        reason: String,
        suggestions: Vec<String>,
    },
}
