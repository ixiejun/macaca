//! Plan Agent scheduling loop.
//!
//! Continuously: process goals → review completed tasks → schedule new work.
//! Runs as a background tokio task, tied to the application's lifetime.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tracing::{info, warn};

use macaca_proto::types::{LlmMessage, LlmOptions, TodoGoalStatus, TodoStatus};

use crate::todo_board::TaskSpace;

/// Configuration for the Plan Agent loop.
pub struct PlanLoopConfig {
    /// How often the plan agent checks for work (default: 30s).
    pub check_interval: Duration,
    /// Maximum number of reviews per cycle (default: 10).
    pub max_reviews_per_cycle: usize,
}

impl Default for PlanLoopConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(5),
            max_reviews_per_cycle: 10,
        }
    }
}

/// Summary of a completed task for goal evaluation.
#[derive(Debug, Clone)]
pub struct TaskSummary {
    pub title: String,
    pub agent: String,
    pub status: String,
    pub completion_summary: Option<String>,
}

/// The Plan Agent's autonomous scheduling loop.
///
/// This loop:
/// 1. Pops pending goals and emits them for LLM decomposition
/// 2. Checks for tasks in PendingReview status
/// 3. Monitors overall progress and anomalies
/// 4. Detects when all tasks for a goal are done and requests evaluation
///
/// Note: The actual LLM calls for decomposition and review are NOT done here.
/// Instead, the Plan Agent runs as a regular agent with access to `create_todo`,
/// `review_todo`, and `check_todo_progress` tools. This loop only handles
/// the scheduling heartbeat — waking the Plan Agent when there is work to do.
pub struct PlanLoop {
    space: Arc<TaskSpace>,
    config: PlanLoopConfig,
    /// Notify handle to wake the loop immediately when new work arrives.
    notify: Arc<tokio::sync::Notify>,
}

/// Handle to wake PlanLoop immediately (e.g., when a goal is created or review submitted).
#[derive(Clone)]
pub struct PlanLoopWaker {
    notify: Arc<tokio::sync::Notify>,
}

impl PlanLoopWaker {
    /// Wake the plan loop immediately to check for new work.
    pub fn wake(&self) {
        self.notify.notify_one();
    }
}

impl PlanLoop {
    pub fn new(space: Arc<TaskSpace>, config: PlanLoopConfig) -> Self {
        Self {
            space,
            config,
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Get a waker handle that can be used to wake this loop immediately.
    pub fn waker(&self) -> PlanLoopWaker {
        PlanLoopWaker {
            notify: Arc::clone(&self.notify),
        }
    }

    /// Run the plan loop until shutdown is signaled.
    ///
    /// Returns a `PlanEvent` each cycle describing what needs attention.
    /// The caller (or an LLM-driven agent) acts on these events.
    pub async fn run(
        &self,
        shutdown: Arc<AtomicBool>,
        event_tx: tokio::sync::mpsc::Sender<PlanEvent>,
    ) {
        info!("Plan loop started");
        let mut last_failed_count: usize = 0;
        let mut review_emitted: HashSet<macaca_proto::TaskId> = HashSet::new();
        let mut goal_eval_emitted: HashSet<macaca_proto::TaskId> = HashSet::new();

        loop {
            // Wait for either: heartbeat timeout OR immediate wakeup notification
            tokio::select! {
                _ = tokio::time::sleep(self.config.check_interval) => {}
                _ = self.notify.notified() => {
                    info!("Plan loop woken up by event");
                }
            }
            if shutdown.load(Ordering::SeqCst) {
                info!("Plan loop shutting down");
                break;
            }

            // 1. Check for new goals
            if let Some(goal) = self.space.pop_goal().await {
                info!(goal_id = %goal.id, "New goal found, requesting decomposition");
                let _ = event_tx
                    .send(PlanEvent::GoalReady {
                        goal_id: goal.id,
                        description: goal.description,
                        session_id: goal.session_id,
                    })
                    .await;
            }

            // 2. Check for tasks needing review (with deduplication)
            let reviews = self.space.pending_reviews().await;
            // Clean up: remove task IDs that are no longer PendingReview
            let current_review_ids: HashSet<macaca_proto::TaskId> =
                reviews.iter().map(|t| t.id).collect();
            review_emitted.retain(|id| current_review_ids.contains(id));
            // Only emit ReviewNeeded for NEW tasks not yet emitted
            let new_reviews: Vec<_> = reviews
                .into_iter()
                .filter(|t| !review_emitted.contains(&t.id))
                .take(self.config.max_reviews_per_cycle)
                .collect();
            if !new_reviews.is_empty() {
                info!(count = new_reviews.len(), "New tasks pending review");
                for task in new_reviews {
                    review_emitted.insert(task.id);
                    let _ = event_tx
                        .send(PlanEvent::ReviewNeeded {
                            task_id: task.id,
                            agent: task.assigned_agent.clone(),
                            title: task.title.clone(),
                            summary: task.completion_summary.clone().unwrap_or_default(),
                            criteria: task.acceptance_criteria.clone(),
                            session_id: task.session_id.clone(),
                        })
                        .await;
                }
            }

            // 3. Check overall progress — only emit anomaly when failed count INCREASES
            let progress = self.space.overall_progress().await;
            if progress.total > 0 && progress.failed > 0 && progress.failed != last_failed_count {
                warn!(
                    failed = progress.failed,
                    "Tasks have failed, may need human intervention"
                );
                let _ = event_tx
                    .send(PlanEvent::AnomalyDetected {
                        message: format!(
                            "{} tasks failed (exceeded max attempts)",
                            progress.failed
                        ),
                    })
                    .await;
                last_failed_count = progress.failed;
            }

            // 4. Check goals with all tasks completed (goal-aware completion detection)
            let goals = self.space.list_goals().await;
            // Clean up eval dedup: remove goals that are no longer InProgress
            goal_eval_emitted.retain(|id| {
                goals
                    .iter()
                    .any(|g| g.id == *id && g.status == TodoGoalStatus::InProgress)
            });
            for goal in &goals {
                // Only evaluate InProgress goals; skip Decomposing/Evaluating/Completed/Failed
                if goal.status != TodoGoalStatus::InProgress {
                    continue;
                }
                // Dedup: skip if already emitted EvaluateGoalCompletion for this goal
                if goal_eval_emitted.contains(&goal.id) {
                    continue;
                }

                let all_todos = self.space.list_all().await;
                let goal_tasks: Vec<_> = all_todos
                    .iter()
                    .filter(|t| t.parent_task == Some(goal.id))
                    .collect();

                if !goal_tasks.is_empty() {
                    let all_done = goal_tasks.iter().all(|t| {
                        matches!(
                            t.status,
                            TodoStatus::Completed | TodoStatus::Failed | TodoStatus::Cancelled
                        )
                    });

                    if all_done {
                        let summaries: Vec<TaskSummary> = goal_tasks
                            .iter()
                            .map(|t| TaskSummary {
                                title: t.title.clone(),
                                agent: t.assigned_agent.clone(),
                                status: format!("{:?}", t.status),
                                completion_summary: t.completion_summary.clone(),
                            })
                            .collect();

                        let completed = goal_tasks
                            .iter()
                            .filter(|t| t.status == TodoStatus::Completed)
                            .count();
                        let failed = goal_tasks
                            .iter()
                            .filter(|t| t.status == TodoStatus::Failed)
                            .count();

                        info!(
                            goal_id = %goal.id,
                            completed,
                            failed,
                            "All tasks for goal done, requesting evaluation"
                        );

                        // Mark as Evaluating to prevent duplicate evaluation
                        self.space
                            .store()
                            .update_goal_status(
                                &goal.application_id,
                                &goal.id,
                                TodoGoalStatus::Evaluating,
                            )
                            .await;
                        goal_eval_emitted.insert(goal.id);

                        let _ = event_tx
                            .send(PlanEvent::EvaluateGoalCompletion {
                                goal_id: goal.id,
                                goal_description: goal.description.clone(),
                                completed_count: completed,
                                failed_count: failed,
                                task_summaries: summaries,
                                session_id: goal.session_id.clone(),
                            })
                            .await;
                    }
                }
            }

            // 5. Fallback: check if ALL tasks across all goals are done
            if progress.total > 0 && self.space.all_tasks_done().await {
                info!("All tasks completed, checking if more work needed");
                let _ = event_tx
                    .send(PlanEvent::AllTasksDone {
                        completed: progress.completed,
                        failed: progress.failed,
                    })
                    .await;
            }
        }
    }
}

/// Events emitted by the Plan loop for the Plan Agent to act on.
#[derive(Debug, Clone)]
pub enum PlanEvent {
    /// A new goal is ready for LLM decomposition into tasks.
    GoalReady {
        goal_id: macaca_proto::TaskId,
        description: String,
        session_id: Option<String>,
    },
    /// A task needs Plan Agent review.
    ReviewNeeded {
        task_id: macaca_proto::TaskId,
        agent: String,
        title: String,
        summary: String,
        criteria: Vec<String>,
        session_id: Option<String>,
    },
    /// All tasks are done — decide whether to generate new work.
    AllTasksDone { completed: usize, failed: usize },
    /// An anomaly was detected (failed tasks, timeouts, etc.).
    AnomalyDetected { message: String },
    /// All tasks for a specific goal are done — request quality evaluation.
    EvaluateGoalCompletion {
        goal_id: macaca_proto::TaskId,
        goal_description: String,
        completed_count: usize,
        failed_count: usize,
        task_summaries: Vec<TaskSummary>,
        session_id: Option<String>,
    },
    /// Goal has been fully verified and marked complete.
    GoalCompleted {
        goal_id: macaca_proto::TaskId,
        description: String,
    },
}

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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── TaskSummary ──

    #[test]
    fn test_task_summary_creation() {
        let summary = TaskSummary {
            title: "Implement API".into(),
            agent: "backend".into(),
            status: "Completed".into(),
            completion_summary: Some("All endpoints implemented and tested".into()),
        };
        assert_eq!(summary.title, "Implement API");
        assert_eq!(summary.agent, "backend");
        assert_eq!(summary.status, "Completed");
        assert!(summary.completion_summary.is_some());
    }

    #[test]
    fn test_task_summary_no_completion_summary() {
        let summary = TaskSummary {
            title: "Design DB schema".into(),
            agent: "architect".into(),
            status: "Failed".into(),
            completion_summary: None,
        };
        assert!(summary.completion_summary.is_none());
    }

    // ── GoalEvaluation parsing ──

    #[test]
    fn test_goal_evaluation_prompt_builder_preserves_contract() {
        let summaries = vec![TaskSummary {
            title: "Implement API".into(),
            agent: "backend".into(),
            status: "Completed".into(),
            completion_summary: Some("API endpoints implemented".into()),
        }];

        let prompt = GoalEvaluator::build_prompt("Build weather app", &summaries, 1, 0);

        assert!(prompt.contains("Goal: Build weather app"));
        assert!(prompt.contains("Task Results (1 completed, 0 failed):"));
        assert!(prompt
            .contains("- [Completed] Implement API (agent: backend): API endpoints implemented"));
        assert!(prompt.contains("\"satisfied\": true/false"));
        assert!(prompt.contains("\"summary\": \"brief evaluation summary\""));
        assert!(prompt.contains("\"suggestions\": [\"suggestion 1\", \"suggestion 2\"]"));
    }

    #[test]
    fn test_goal_evaluation_satisfied_json() {
        let json =
            r#"{"satisfied": true, "summary": "All tasks passed, goal met.", "suggestions": []}"#;
        let result = GoalEvaluator::parse_eval_response(json);
        match result {
            GoalEvaluation::Satisfied { summary } => {
                assert_eq!(summary, "All tasks passed, goal met.");
            }
            GoalEvaluation::NeedsMoreWork { .. } => panic!("expected Satisfied"),
        }
    }

    #[test]
    fn test_goal_evaluation_needs_work_json() {
        let json = r#"{
            "satisfied": false,
            "summary": "Missing error handling in API endpoints.",
            "suggestions": ["Add 400/500 error responses", "Add input validation"]
        }"#;
        let result = GoalEvaluator::parse_eval_response(json);
        match result {
            GoalEvaluation::NeedsMoreWork {
                reason,
                suggestions,
            } => {
                assert_eq!(reason, "Missing error handling in API endpoints.");
                assert_eq!(suggestions.len(), 2);
                assert_eq!(suggestions[0], "Add 400/500 error responses");
            }
            GoalEvaluation::Satisfied { .. } => panic!("expected NeedsMoreWork"),
        }
    }

    #[test]
    fn test_goal_evaluation_needs_work_no_suggestions() {
        let json = r#"{"satisfied": false, "summary": "Incomplete implementation."}"#;
        let result = GoalEvaluator::parse_eval_response(json);
        match result {
            GoalEvaluation::NeedsMoreWork { suggestions, .. } => {
                assert!(suggestions.is_empty());
            }
            GoalEvaluation::Satisfied { .. } => panic!("expected NeedsMoreWork"),
        }
    }

    #[test]
    fn test_goal_evaluation_fallback_on_bad_json() {
        let result = GoalEvaluator::parse_eval_response("not valid json at all");
        match result {
            GoalEvaluation::Satisfied { summary } => {
                assert!(summary.contains("fallback"));
            }
            GoalEvaluation::NeedsMoreWork { .. } => panic!("expected Satisfied fallback"),
        }
    }

    #[test]
    fn test_goal_evaluation_fallback_on_empty_string() {
        let result = GoalEvaluator::parse_eval_response("");
        match result {
            GoalEvaluation::Satisfied { .. } => {}
            GoalEvaluation::NeedsMoreWork { .. } => panic!("expected Satisfied fallback"),
        }
    }

    #[test]
    fn test_goal_evaluation_parses_markdown_code_block() {
        let json =
            "```json\n{\"satisfied\": true, \"summary\": \"Done\", \"suggestions\": []}\n```";
        let result = GoalEvaluator::parse_eval_response(json);
        match result {
            GoalEvaluation::Satisfied { summary } => assert_eq!(summary, "Done"),
            GoalEvaluation::NeedsMoreWork { .. } => panic!("expected Satisfied"),
        }
    }

    #[test]
    fn test_goal_evaluation_parses_plain_code_block() {
        let json = "```\n{\"satisfied\": false, \"summary\": \"Needs work\", \"suggestions\": [\"fix tests\"]}\n```";
        let result = GoalEvaluator::parse_eval_response(json);
        match result {
            GoalEvaluation::NeedsMoreWork {
                reason,
                suggestions,
            } => {
                assert_eq!(reason, "Needs work");
                assert_eq!(suggestions[0], "fix tests");
            }
            GoalEvaluation::Satisfied { .. } => panic!("expected NeedsMoreWork"),
        }
    }

    // ── PlanEvent variants ──

    #[test]
    fn test_plan_event_evaluate_goal_completion_is_debug_clone() {
        let summary = TaskSummary {
            title: "T".into(),
            agent: "a".into(),
            status: "Completed".into(),
            completion_summary: None,
        };
        let event = PlanEvent::EvaluateGoalCompletion {
            goal_id: macaca_proto::TaskId::new(),
            goal_description: "Build auth".into(),
            completed_count: 3,
            failed_count: 0,
            task_summaries: vec![summary],
            session_id: None,
        };
        let cloned = event.clone();
        let debug_str = format!("{:?}", cloned);
        assert!(debug_str.contains("EvaluateGoalCompletion"));
    }

    #[test]
    fn test_plan_event_goal_completed_is_debug_clone() {
        let event = PlanEvent::GoalCompleted {
            goal_id: macaca_proto::TaskId::new(),
            description: "Goal is done".into(),
        };
        let debug_str = format!("{:?}", event.clone());
        assert!(debug_str.contains("GoalCompleted"));
    }
}
