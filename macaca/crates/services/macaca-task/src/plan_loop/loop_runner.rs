//! Plan Agent scheduling heartbeat (Template Method + Observer).
//!
//! `PlanLoop` polls `TaskSpace` on an interval or immediate wakeup, then emits
//! `PlanEvent` messages for downstream consumers. Trace logs mark wakeup, shutdown,
//! goal discovery, review batching, anomaly detection, and goal-completion checks.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tracing::{info, warn};

use macaca_proto::types::{TodoGoalStatus, TodoStatus};

use crate::todo_board::TaskSpace;

use super::config::{PlanLoopConfig, TaskSummary};
use super::events::PlanEvent;

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
    pub fn with_components(space: Arc<TaskSpace>, config: PlanLoopConfig) -> Self {
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

    pub async fn run_with_default_template(
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

            self.process_new_goals(&event_tx).await;
            let progress = self
                .emit_pending_reviews(&event_tx, &mut review_emitted)
                .await;
            let progress = match progress {
                Some(progress) => progress,
                None => self.space.overall_progress().await,
            };
            self.emit_progress_anomalies(&event_tx, &progress, &mut last_failed_count)
                .await;
            self.emit_goal_completion_checks(&event_tx, &mut goal_eval_emitted)
                .await;
            self.emit_all_tasks_done_fallback(&event_tx, &progress)
                .await;
        }
    }

    async fn process_new_goals(&self, event_tx: &tokio::sync::mpsc::Sender<PlanEvent>) {
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
    }

    async fn emit_pending_reviews(
        &self,
        event_tx: &tokio::sync::mpsc::Sender<PlanEvent>,
        review_emitted: &mut HashSet<macaca_proto::TaskId>,
    ) -> Option<crate::todo_board::ProgressSummary> {
        let reviews = self.space.pending_reviews().await;
        let current_review_ids: HashSet<macaca_proto::TaskId> =
            reviews.iter().map(|t| t.id).collect();
        review_emitted.retain(|id| current_review_ids.contains(id));
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
        None
    }

    async fn emit_progress_anomalies(
        &self,
        event_tx: &tokio::sync::mpsc::Sender<PlanEvent>,
        progress: &crate::todo_board::ProgressSummary,
        last_failed_count: &mut usize,
    ) {
        if progress.total > 0 && progress.failed > 0 && progress.failed != *last_failed_count {
            warn!(
                failed = progress.failed,
                "Tasks have failed, may need human intervention"
            );
            let _ = event_tx
                .send(PlanEvent::AnomalyDetected {
                    message: format!("{} tasks failed (exceeded max attempts)", progress.failed),
                })
                .await;
            *last_failed_count = progress.failed;
        }
    }

    async fn emit_goal_completion_checks(
        &self,
        event_tx: &tokio::sync::mpsc::Sender<PlanEvent>,
        goal_eval_emitted: &mut HashSet<macaca_proto::TaskId>,
    ) {
        let goals = self.space.list_goals().await;
        goal_eval_emitted.retain(|id| {
            goals
                .iter()
                .any(|g| g.id == *id && g.status == TodoGoalStatus::InProgress)
        });
        for goal in &goals {
            if goal.status != TodoGoalStatus::InProgress || goal_eval_emitted.contains(&goal.id) {
                continue;
            }

            let all_todos = self.space.list_all().await;
            let goal_tasks: Vec<_> = all_todos
                .iter()
                .filter(|t| t.parent_task == Some(goal.id))
                .collect();

            if goal_tasks.is_empty() {
                continue;
            }

            let all_done = goal_tasks.iter().all(|t| {
                matches!(
                    t.status,
                    TodoStatus::Completed | TodoStatus::Failed | TodoStatus::Cancelled
                )
            });
            if !all_done {
                continue;
            }

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

            self.space
                .store()
                .update_goal_status(&goal.application_id, &goal.id, TodoGoalStatus::Evaluating)
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

    async fn emit_all_tasks_done_fallback(
        &self,
        event_tx: &tokio::sync::mpsc::Sender<PlanEvent>,
        progress: &crate::todo_board::ProgressSummary,
    ) {
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
