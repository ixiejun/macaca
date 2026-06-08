//! Per-agent task panel (`TaskBoard`).
//!
//! Each agent owns an isolated board backed by [`TodoStore`]. Workers pull work via
//! session-scoped sequential claim rules while lifecycle transitions flow through
//! injectable [`TodoLifecyclePolicy`] and [`TaskDependencyResolver`] strategies.

use std::sync::Arc;

use chrono::Utc;
use macaca_proto::{ApplicationId, TaskId, TodoItem, TodoStatus};
use tracing::info;

use crate::dependency::{DefaultTaskDependencyResolver, TaskDependencyResolver};
use crate::lifecycle::{DefaultTodoLifecyclePolicy, TodoLifecyclePolicy};
use crate::todo_store::TodoStore;

/// An agent's isolated task panel. The agent can only see and modify its own board.
pub struct TaskBoard {
    app_id: ApplicationId,
    agent_name: String,
    session_id: Option<String>,
    store: Arc<TodoStore>,
    lifecycle_policy: Arc<dyn TodoLifecyclePolicy>,
    dependency_resolver: Arc<dyn TaskDependencyResolver>,
}

impl TaskBoard {
    #[deprecated(note = "Use TaskBoard::for_agent instead")]
    pub fn new(
        app_id: ApplicationId,
        agent_name: impl Into<String>,
        session_id: Option<String>,
        store: Arc<TodoStore>,
    ) -> Self {
        Self::for_agent(app_id, agent_name, session_id, store)
    }

    pub fn for_agent(
        app_id: ApplicationId,
        agent_name: impl Into<String>,
        session_id: Option<String>,
        store: Arc<TodoStore>,
    ) -> Self {
        Self::with_components(
            app_id,
            agent_name,
            session_id,
            store,
            Arc::new(DefaultTodoLifecyclePolicy),
            Arc::new(DefaultTaskDependencyResolver),
        )
    }

    pub fn with_components(
        app_id: ApplicationId,
        agent_name: impl Into<String>,
        session_id: Option<String>,
        store: Arc<TodoStore>,
        lifecycle_policy: Arc<dyn TodoLifecyclePolicy>,
        dependency_resolver: Arc<dyn TaskDependencyResolver>,
    ) -> Self {
        Self {
            app_id,
            agent_name: agent_name.into(),
            session_id,
            store,
            lifecycle_policy,
            dependency_resolver,
        }
    }

    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }

    /// Claim the next task in sequence order. Returns None if no tasks available
    /// or if the lowest-sequence task is not yet claimable (Blocked/InProgress/Assigned).
    /// Sequential ordering is enforced PER SESSION — different sessions are independent.
    /// Atomically transitions Pending → Assigned.
    #[deprecated(note = "Use TaskBoard::claim_next_task instead")]
    pub async fn claim_next(&self) -> Option<TodoItem> {
        self.claim_next_task().await
    }

    pub async fn claim_next_task(&self) -> Option<TodoItem> {
        let all = self
            .store
            .list_agent_todos(&self.app_id, &self.session_id, &self.agent_name)
            .await;
        let goals = self.store.list_goals(&self.app_id).await;

        // Group tasks by session_id, then apply sequential logic per session
        let mut by_session: std::collections::BTreeMap<String, Vec<TodoItem>> =
            std::collections::BTreeMap::new();
        for task in all {
            let key = task
                .session_id
                .clone()
                .unwrap_or_else(|| "_global_".to_string());
            by_session.entry(key).or_default().push(task);
        }

        // Sort sessions by most recent task created_at (newest first)
        let mut session_order: Vec<(String, Vec<TodoItem>)> = by_session.into_iter().collect();
        session_order.sort_by(|a, b| {
            let a_latest = a.1.iter().map(|t| t.created_at).max();
            let b_latest = b.1.iter().map(|t| t.created_at).max();
            b_latest.cmp(&a_latest) // newest session first
        });

        // Try each session (newest first): find the first one with a claimable task
        for (_session, mut tasks) in session_order {
            // Sort by sequence_number ascending within this session
            tasks.sort_by_key(|t| t.sequence_number);

            for task in &tasks {
                match task.status {
                    // Terminal states — skip, look at next in sequence
                    TodoStatus::Completed | TodoStatus::Cancelled | TodoStatus::Failed => continue,
                    // Claimable — take it
                    TodoStatus::Pending => {
                        if !self.dependency_resolver.can_claim_task(task, &goals) {
                            tracing::debug!(
                                agent = %self.agent_name,
                                task_id = %task.id,
                                parent_task = ?task.parent_task,
                                "Task pending but parent goal is not InProgress; skipping claim"
                            );
                            break; // Try next session
                        }
                        let mut claimed = task.clone();
                        let Some(next_status) = self.lifecycle_policy.on_claim(&claimed) else {
                            break;
                        };
                        claimed.status = next_status;
                        claimed.updated_at = Utc::now();
                        self.store.save_todo(&claimed).await;
                        info!(
                            app_id = %self.app_id.0,
                            agent = %self.agent_name,
                            task_id = %claimed.id,
                            session_id = ?self.session_id,
                            sequence = claimed.sequence_number,
                            status = ?claimed.status,
                            "task board sequential claim persisted"
                        );
                        return Some(claimed);
                    }
                    // Blocked or already in progress — stop FOR THIS SESSION, try next session
                    TodoStatus::Blocked
                    | TodoStatus::Assigned
                    | TodoStatus::InProgress
                    | TodoStatus::PendingReview
                    | TodoStatus::NeedsOptimization => {
                        break; // Try next session
                    }
                }
            }
        }
        None
    }

    /// Claim a specific pending task on this agent board.
    ///
    /// Host-side service bridges already receive a durable task id from the
    /// Task Service when they create explicit assignments.  Re-claiming "the
    /// next" task after that point is unsafe because another session or another
    /// task for the same agent can become earlier in sequence order.  This
    /// method applies the same dependency and lifecycle policy as
    /// [`claim_next_task`], but anchors the transition to the caller-provided
    /// task id so orchestration remains traceable and session-isolated.
    pub async fn claim_task(&self, task_id: &TaskId) -> Option<TodoItem> {
        let task = self
            .store
            .get_todo(&self.app_id, &self.session_id, &self.agent_name, task_id)
            .await?;
        if task.status != TodoStatus::Pending {
            tracing::debug!(
                agent = %self.agent_name,
                task_id = %task.id,
                status = ?task.status,
                "Task claim ignored because the requested task is not pending"
            );
            return None;
        }
        let goals = self.store.list_goals(&self.app_id).await;
        if !self.dependency_resolver.can_claim_task(&task, &goals) {
            tracing::debug!(
                agent = %self.agent_name,
                task_id = %task.id,
                parent_task = ?task.parent_task,
                "Task claim ignored because dependencies or parent goal are not ready"
            );
            return None;
        }
        let Some(next_status) = self.lifecycle_policy.on_claim(&task) else {
            return None;
        };
        let mut claimed = task;
        claimed.status = next_status;
        claimed.updated_at = Utc::now();
        self.store.save_todo(&claimed).await;
        info!(
            app_id = %self.app_id.0,
            agent = %self.agent_name,
            task_id = %claimed.id,
            session_id = ?self.session_id,
            status = ?claimed.status,
            "task board anchored claim persisted"
        );
        Some(claimed)
    }

    /// Mark a task as in-progress. Called by the agent after claiming.
    #[deprecated(note = "Use TaskBoard::mark_task_in_progress instead")]
    pub async fn start_task(&self, task_id: &TaskId) -> bool {
        self.mark_task_in_progress(task_id).await
    }

    pub async fn mark_task_in_progress(&self, task_id: &TaskId) -> bool {
        if let Some(mut task) = self
            .store
            .get_todo(&self.app_id, &self.session_id, &self.agent_name, task_id)
            .await
        {
            if let Some(next_status) = self.lifecycle_policy.on_start(&task) {
                task.status = next_status;
                task.attempt_count += 1;
                task.updated_at = Utc::now();
                self.store.save_todo(&task).await;
                info!(
                    app_id = %self.app_id.0,
                    agent = %self.agent_name,
                    task_id = %task.id,
                    attempt = task.attempt_count,
                    status = ?task.status,
                    "task board marked in progress"
                );
                return true;
            }
        }
        false
    }

    /// Submit a completed task for Plan Agent review.
    #[deprecated(note = "Use TaskBoard::submit_task_for_review instead")]
    pub async fn submit_for_review(&self, task_id: &TaskId, summary: String) -> bool {
        self.submit_task_for_review(task_id, summary).await
    }

    pub async fn submit_task_for_review(&self, task_id: &TaskId, summary: String) -> bool {
        if let Some(mut task) = self
            .store
            .get_todo(&self.app_id, &self.session_id, &self.agent_name, task_id)
            .await
        {
            if let Some(next_status) = self.lifecycle_policy.on_submit_for_review(&task) {
                task.status = next_status;
                task.completion_summary = Some(summary);
                task.updated_at = Utc::now();
                self.store.save_todo(&task).await;
                info!(
                    app_id = %self.app_id.0,
                    agent = %self.agent_name,
                    task_id = %task.id,
                    status = ?task.status,
                    "task board submitted for review"
                );
                return true;
            }
        }
        false
    }

    /// Update progress on the current task.
    pub async fn update_progress(&self, task_id: &TaskId, message: String) -> bool {
        if let Some(mut task) = self
            .store
            .get_todo(&self.app_id, &self.session_id, &self.agent_name, task_id)
            .await
        {
            if task.status == TodoStatus::InProgress {
                task.progress_notes.push(message);
                task.updated_at = Utc::now();
                self.store.save_todo(&task).await;
                return true;
            }
        }
        false
    }

    /// Mark a task as failed (exceeded max attempts).
    #[deprecated(note = "Use TaskBoard::fail_task instead")]
    pub async fn mark_failed(&self, task_id: &TaskId, error: String) -> bool {
        self.fail_task(task_id, error).await
    }

    pub async fn fail_task(&self, task_id: &TaskId, error: String) -> bool {
        if let Some(mut task) = self
            .store
            .get_todo(&self.app_id, &self.session_id, &self.agent_name, task_id)
            .await
        {
            task.status = self.lifecycle_policy.on_mark_failed(&task);
            task.review_feedback = Some(error);
            task.updated_at = Utc::now();
            self.store.save_todo(&task).await;
            info!(
                app_id = %self.app_id.0,
                agent = %self.agent_name,
                task_id = %task.id,
                status = ?task.status,
                "task board marked failed"
            );
            return true;
        }
        false
    }

    /// Get the current in-progress task (if any).
    pub async fn current_task(&self) -> Option<TodoItem> {
        self.store
            .list_agent_todos_by_status(
                &self.app_id,
                &self.session_id,
                &self.agent_name,
                TodoStatus::InProgress,
            )
            .await
            .into_iter()
            .next()
    }

    /// Check if there are pending tasks to claim.
    pub async fn has_pending_tasks(&self) -> bool {
        !self
            .store
            .list_agent_todos_by_status(
                &self.app_id,
                &self.session_id,
                &self.agent_name,
                TodoStatus::Pending,
            )
            .await
            .is_empty()
    }

    /// Get tasks that need optimization (for retry).
    pub async fn needs_optimization_tasks(&self) -> Vec<TodoItem> {
        self.store
            .list_agent_todos_by_status(
                &self.app_id,
                &self.session_id,
                &self.agent_name,
                TodoStatus::NeedsOptimization,
            )
            .await
    }

    /// List all tasks on this board.
    pub async fn list_all(&self) -> Vec<TodoItem> {
        self.store
            .list_agent_todos(&self.app_id, &self.session_id, &self.agent_name)
            .await
    }
}
