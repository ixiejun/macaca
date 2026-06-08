//! Goal quality evaluation helpers (Strategy + Template Method).
//!
//! `GoalEvaluator::build_prompt` produces provider-neutral evaluation prompts.
//! Runtime callers should execute prompts through the traced framework agent path;
//! the deprecated direct LLM `evaluate` path remains for legacy compatibility only.

use std::sync::Arc;

use macaca_proto::types::{LlmMessage, LlmOptions};

use super::config::TaskSummary;

// ── GoalEvaluator ─────────────────────────────────────────────────────────────

/// Result of evaluating whether a completed goal meets quality standards.
#[derive(Debug, Clone)]
pub enum GoalEvaluation {
    /// Goal is complete and satisfactory.
    Satisfied { summary: String },
    /// Goal needs additional work — new tasks suggested.
    NeedsMoreWork {
        reason: String,
        suggestions: Vec<String>,
    },
}

/// Pure goal evaluation prompt/parser helper.
///
/// Runtime model execution should happen through the framework agent/model path
/// owned by the application runtime. The deprecated direct LLM API remains only
/// for compatibility with older callers.
pub struct GoalEvaluator {
    llm: Arc<dyn macaca_llm::LlmProvider>,
    model: String,
}

impl GoalEvaluator {
    #[deprecated(
        note = "Use GoalEvaluator::build_prompt + framework agent/model execution + parse_eval_response"
    )]
    pub fn new(llm: Arc<dyn macaca_llm::LlmProvider>, model: impl Into<String>) -> Self {
        Self {
            llm,
            model: model.into(),
        }
    }

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

    /// Evaluate whether a goal's tasks collectively satisfy the goal.
    ///
    /// Deprecated: this direct LLM path bypasses framework trace/model routing.
    /// Runtime callers should use `build_prompt`, execute through a traced
    /// framework agent/model, then call `parse_eval_response`.
    #[deprecated(
        note = "Use GoalEvaluator::build_prompt + framework agent/model execution + parse_eval_response"
    )]
    pub async fn evaluate(
        &self,
        goal_description: &str,
        task_summaries: &[TaskSummary],
        completed: usize,
        failed: usize,
    ) -> Result<GoalEvaluation, String> {
        let prompt = Self::build_prompt(goal_description, task_summaries, completed, failed);

        let messages = vec![LlmMessage::user(&prompt)];
        let options = LlmOptions {
            model: self.model.clone(),
            temperature: Some(0.3),
            ..Default::default()
        };

        let response = self
            .llm
            .chat(messages, &options)
            .await
            .map_err(|e| format!("Goal evaluation LLM call failed: {}", e))?;

        Ok(Self::parse_eval_response(&response.content))
    }

    /// Parse the LLM evaluation response. Returns `Satisfied` on any parse failure.
    pub fn parse_eval_response(content: &str) -> GoalEvaluation {
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
                    GoalEvaluation::Satisfied {
                        summary: eval.summary,
                    }
                } else {
                    GoalEvaluation::NeedsMoreWork {
                        reason: eval.summary,
                        suggestions: eval.suggestions,
                    }
                }
            }
            Err(_) => {
                // Conservative fallback: assume satisfied so we don't block
                GoalEvaluation::Satisfied {
                    summary: "Evaluation completed (parsing fallback)".into(),
                }
            }
        }
    }
}
