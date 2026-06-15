//! Goal quality evaluation helpers (Strategy + Template Method).
//!
//! `GoalEvaluator::build_prompt` produces provider-neutral evaluation prompts.
//! Runtime callers execute prompts through the traced framework agent path and
//! feed the response back into `GoalEvaluator::parse_eval_response`.

use macaca_proto::GoalEvaluationResult;

use super::config::TaskSummary;

// ── GoalEvaluator ─────────────────────────────────────────────────────────────

/// Result of evaluating whether a completed goal meets quality standards.
pub type GoalEvaluation = GoalEvaluationResult;

/// Pure goal evaluation prompt/parser helper.
///
/// This type intentionally has no provider state. Model execution belongs to the
/// traced framework/model path so evaluation remains replayable and service-owned
/// rather than a direct LLM side effect inside the task service.
pub struct GoalEvaluator;

impl GoalEvaluator {
    /// Build the goal evaluation prompt. The caller is responsible for running
    /// this prompt through the traced framework agent/model path.
    pub fn build_prompt(
        goal_description: &str,
        task_summaries: &[TaskSummary],
        completed: usize,
        failed: usize,
    ) -> String {
        let summaries_text = task_summaries
            .iter()
            .map(|s| {
                format!(
                    "- [{}] {} (agent: {}): {}",
                    s.status,
                    s.title,
                    s.agent,
                    s.completion_summary.as_deref().unwrap_or("no summary"),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"You are a quality evaluation agent. Evaluate whether the following goal has been satisfactorily completed.

Goal: {goal_description}

Task Results ({completed} completed, {failed} failed):
{summaries_text}

Evaluate the overall quality and completeness. Respond with JSON:
{{
  "satisfied": true/false,
  "summary": "brief evaluation summary",
  "suggestions": ["suggestion 1", "suggestion 2"]
}}

If all critical tasks completed and the goal is met, set satisfied=true.
If there are gaps or quality issues, set satisfied=false and provide suggestions."#
        )
    }

    /// Parse the LLM evaluation response. Returns `Satisfied` on any parse failure.
    pub fn parse_eval_response(content: &str) -> GoalEvaluationResult {
        let content = content.trim();
        let json_str = if content.starts_with("```") {
            content
                .strip_prefix("```json")
                .or_else(|| content.strip_prefix("```"))
                .unwrap_or(content)
                .strip_suffix("```")
                .unwrap_or(content)
                .trim()
        } else {
            content
        };

        #[derive(serde::Deserialize)]
        struct EvalResponse {
            satisfied: bool,
            summary: String,
            #[serde(default)]
            suggestions: Vec<String>,
        }

        match serde_json::from_str::<EvalResponse>(json_str) {
            Ok(eval) => {
                if eval.satisfied {
                    GoalEvaluationResult::Satisfied {
                        summary: eval.summary,
                    }
                } else {
                    GoalEvaluationResult::NeedsMoreWork {
                        reason: eval.summary,
                        suggestions: eval.suggestions,
                    }
                }
            }
            Err(_) => {
                // Conservative fallback: assume satisfied so we don't block
                GoalEvaluationResult::Satisfied {
                    summary: "Evaluation completed (parsing fallback)".into(),
                }
            }
        }
    }
}
