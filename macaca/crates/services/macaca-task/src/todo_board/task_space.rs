//! Application-level task workspace (`TaskSpace`) and progress aggregates.
//!
//! Plan-side orchestration uses `TaskSpace` to assign work across agent boards,
//! review completions, manage goals, and compute session-scoped progress snapshots.

use std::sync::Arc;

use chrono::Utc;
use macaca_proto::{
    ApplicationId, TaskGraphOwner, TaskId, TodoGoal, TodoGoalStatus, TodoItem, TodoReviewResult,
    TodoStatus,
};
use tracing::info;

use crate::dependency::{DefaultTaskDependencyResolver, TaskDependencyResolver};
use crate::lifecycle::{DefaultTodoLifecyclePolicy, TodoLifecyclePolicy};
use crate::todo_store::TodoStore;

/// Progress summary for an application's task space.
#[derive(Debug, Clone, Default)]
pub struct ProgressSummary {
    pub total: usize,
    pub pending: usize,
    pub assigned: usize,
    pub in_progress: usize,
    pub pending_review: usize,
    pub needs_optimization: usize,
    pub completed: usize,
    pub blocked: usize,
    pub failed: usize,
    pub cancelled: usize,
}

/// Application-level task workspace. Only the Plan Agent / Coordinator may access this.
pub struct TaskSpace {
    app_id: ApplicationId,
    session_id: Option<String>,
    store: Arc<TodoStore>,
    lifecycle_policy: Arc<dyn TodoLifecyclePolicy>,
    dependency_resolver: Arc<dyn TaskDependencyResolver>,
}

impl TaskSpace {
    #[deprecated(note = "Use TaskSpace::for_session instead")]
    pub fn new(app_id: ApplicationId, session_id: Option<String>, store: Arc<TodoStore>) -> Self {
        Self::for_session(app_id, session_id, store)
    }

    pub fn for_session(
        app_id: ApplicationId,
        session_id: Option<String>,
        store: Arc<TodoStore>,
    ) -> Self {
        Self::with_components(
            app_id,
            session_id,
            store,
            Arc::new(DefaultTodoLifecyclePolicy),
            Arc::new(DefaultTaskDependencyResolver),
        )
    }

    pub fn with_components(
        app_id: ApplicationId,
        session_id: Option<String>,
        store: Arc<TodoStore>,
        lifecycle_policy: Arc<dyn TodoLifecyclePolicy>,
        dependency_resolver: Arc<dyn TaskDependencyResolver>,
    ) -> Self {
        Self {
            app_id,
            session_id,
            store,
            lifecycle_policy,
            dependency_resolver,
        }
    }

    /// Access the underlying TodoStore.
    pub fn store(&self) -> &Arc<TodoStore> {
        &self.store
    }

    /// Create a new task and assign it to an agent's board.
    #[deprecated(note = "Use TaskSpace::create_task_assignment instead")]
    pub async fn create_and_assign(
        &self,
        agent: &str,
        created_by: &str,
        title: impl Into<String>,
        description: impl Into<String>,
        acceptance_criteria: Vec<String>,
        priority: u8,
        depends_on: Vec<TaskId>,
        parent_task: Option<TaskId>,
    ) -> TodoItem {
        self.create_task_assignment(
            agent,
            created_by,
            title,
            description,
            acceptance_criteria,
            priority,
            depends_on,
            parent_task,
        )
        .await
    }

    pub async fn create_task_assignment(
        &self,
        agent: &str,
        created_by: &str,
        title: impl Into<String>,
        description: impl Into<String>,
        acceptance_criteria: Vec<String>,
        priority: u8,
        depends_on: Vec<TaskId>,
        parent_task: Option<TaskId>,
    ) -> TodoItem {
        self.create_task_assignment_with_graph_owner(
            agent,
            created_by,
            title,
            description,
            acceptance_criteria,
            priority,
            depends_on,
            parent_task,
            TaskGraphOwner::TaskServiceNative,
        )
        .await
    }

    /// Create a task assignment with an explicit service-owned graph marker.
    ///
    /// The graph owner is not part of application product behavior.  It is a
    /// Macaca service boundary marker used by task snapshots, audit records,
    /// and application-execution terminal projection to separate
    /// authoritative execution tasks from compatibility or diagnostic board
    /// entries.  Callers must pass service categories only; application names,
    /// workflow names, provider names, and business-domain identifiers do not
    /// belong here.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_task_assignment_with_graph_owner(
        &self,
        agent: &str,
        created_by: &str,
        title: impl Into<String>,
        description: impl Into<String>,
        acceptance_criteria: Vec<String>,
        priority: u8,
        depends_on: Vec<TaskId>,
        parent_task: Option<TaskId>,
        graph_owner: TaskGraphOwner,
    ) -> TodoItem {
        self.create_task_assignment_with_graph_scope(
            agent,
            created_by,
            title,
            description,
            acceptance_criteria,
            priority,
            depends_on,
            parent_task,
            graph_owner,
            None,
        )
        .await
    }

    /// Create a task assignment with an explicit graph owner and graph id.
    ///
    /// This lower-level helper is used by service-runtime commands that need
    /// graph admission guarantees.  `graph_id` is intentionally an opaque
    /// service-owned correlation key; the Task Service stores it for replay,
    /// audit, and terminal projection, but never interprets it as application
    /// business data.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_task_assignment_with_graph_scope(
        &self,
        agent: &str,
        created_by: &str,
        title: impl Into<String>,
        description: impl Into<String>,
        acceptance_criteria: Vec<String>,
        priority: u8,
        depends_on: Vec<TaskId>,
        parent_task: Option<TaskId>,
        graph_owner: TaskGraphOwner,
        graph_id: Option<String>,
    ) -> TodoItem {
        // Auto-assign sequence_number: next after current max for this agent+session
        let max_seq = self
            .store
            .get_max_sequence_number(&self.app_id, &self.session_id, agent)
            .await;
        let seq = max_seq + 1;

        let mut item = TodoItem::new(
            self.app_id.clone(),
            self.session_id.clone(),
            agent,
            created_by,
            title,
            description,
            priority,
        );
        item.sequence_number = seq;
        item.graph_owner = graph_owner;
        item.graph_id = graph_id;
        item.acceptance_criteria = acceptance_criteria;
        item.depends_on = depends_on.clone();
        item.parent_task = parent_task;
        let all_todos = self.list_all_internal().await;
        item.status = self
            .dependency_resolver
            .initial_status_for_new_task(&depends_on, &all_todos);

        self.store.save_todo(&item).await;
        info!(
            app_id = %self.app_id.0,
            session_id = ?self.session_id,
            task_id = %item.id,
            assigned_agent = %item.assigned_agent,
            created_by = %item.created_by,
            sequence = item.sequence_number,
            status = ?item.status,
            graph_owner = %item.graph_owner.as_str(),
            graph_id = item.graph_id.as_deref().unwrap_or("none"),
            "task space assignment created"
        );
        item
    }

    /// Review a task submitted by an agent.
    /// Searches across sessions (reviewer may not know the originating session).
    #[deprecated(note = "Use TaskSpace::apply_review_result instead")]
    pub async fn review_task(
        &self,
        task_id: &TaskId,
        agent: &str,
        result: TodoReviewResult,
    ) -> bool {
        self.apply_review_result(task_id, agent, result).await
    }

    pub async fn apply_review_result(
        &self,
        task_id: &TaskId,
        agent: &str,
        result: TodoReviewResult,
    ) -> bool {
        // Session-scoped lookup when this TaskSpace has a session; otherwise scan the whole app.
        let task_opt = if self.session_id.is_some() {
            self.store
                .get_todo(&self.app_id, &self.session_id, agent, task_id)
                .await
        } else {
            self.store
                .list_all_todos(&self.app_id)
                .await
                .into_iter()
                .find(|t| t.id == *task_id && t.assigned_agent == agent)
        };

        if let Some(mut task) = task_opt {
            if task.status != TodoStatus::PendingReview {
                return false;
            }
            task.review_feedback = Some(result.feedback.clone());
            task.updated_at = Utc::now();

            task.status = self.lifecycle_policy.on_review(&task, &result);
            if task.status == TodoStatus::NeedsOptimization {
                task.optimization_suggestions = Some(result.feedback);
            }
            self.store.save_todo(&task).await;

            // If completed, unblock dependents
            if task.status == TodoStatus::Completed {
                self.unblock_dependents(&task.id).await;
            }

            info!(
                app_id = %self.app_id.0,
                session_id = ?self.session_id,
                task_id = %task.id,
                agent = %agent,
                review_passed = result.passed,
                status = ?task.status,
                "task space review applied"
            );
            return true;
        }
        false
    }

    /// Skip a task (Pending/Blocked → Cancelled). Triggers dependency re-evaluation.
    #[deprecated(note = "Use TaskSpace::cancel_task instead")]
    pub async fn skip_task(&self, task_id: &TaskId) -> bool {
        self.cancel_task(task_id).await
    }

    pub async fn cancel_task(&self, task_id: &TaskId) -> bool {
        let all = self.list_all_internal().await;
        if let Some(mut task) = all.iter().find(|t| t.id == *task_id).cloned() {
            let Some(next_status) = self.lifecycle_policy.on_skip(&task) else {
                return false;
            };
            task.status = next_status;
            task.updated_at = Utc::now();
            self.store.save_todo(&task).await;
            // Re-evaluate dependents (they'll stay Blocked since dep is Cancelled, not Completed)
            self.reevaluate_dependents(task_id).await;
            info!(
                app_id = %self.app_id.0,
                session_id = ?self.session_id,
                task_id = %task.id,
                status = ?task.status,
                "task space task cancelled"
            );
            true
        } else {
            false
        }
    }

    /// Unblock tasks that depended on a now-completed task.
    /// Only unblocks if ALL dependencies are Completed.
    async fn unblock_dependents(&self, completed_id: &TaskId) {
        // Same scope as this TaskSpace: one chat/session only when session_id is set.
        let all = self.list_all_internal().await;
        let ready_ids = self
            .dependency_resolver
            .blocked_tasks_ready_after_completion(&all, completed_id);

        for mut item in all {
            if ready_ids.contains(&item.id) {
                item.status = TodoStatus::Pending;
                item.updated_at = Utc::now();
                self.store.save_todo(&item).await;
            }
        }
    }

    /// Re-evaluate blocked dependents after a task is cancelled/failed.
    /// Dependents remain Blocked (their dep is not Completed).
    async fn reevaluate_dependents(&self, _changed_id: &TaskId) {
        // Blocked tasks stay blocked — their dependency was cancelled, not completed.
        // PlanLoop will detect this via AnomalyDetected when it checks for stuck tasks.
    }

    /// Get all tasks pending review. Cross-session — PlanLoop needs this.
    pub async fn pending_reviews(&self) -> Vec<TodoItem> {
        self.store.list_pending_reviews(&self.app_id).await
    }

    /// Check if all tasks in the space are terminal (completed/cancelled/failed).
    pub async fn all_tasks_done(&self) -> bool {
        let all = self.list_all_internal().await;
        if all.is_empty() {
            return true;
        }
        all.iter().all(|t| {
            matches!(
                t.status,
                TodoStatus::Completed | TodoStatus::Cancelled | TodoStatus::Failed
            )
        })
    }

    /// Compute progress summary.
    pub async fn overall_progress(&self) -> ProgressSummary {
        let all = self.list_all_internal().await;
        let mut s = ProgressSummary {
            total: all.len(),
            ..Default::default()
        };
        for item in &all {
            match item.status {
                TodoStatus::Pending => s.pending += 1,
                TodoStatus::Assigned => s.assigned += 1,
                TodoStatus::InProgress => s.in_progress += 1,
                TodoStatus::PendingReview => s.pending_review += 1,
                TodoStatus::NeedsOptimization => s.needs_optimization += 1,
                TodoStatus::Completed => s.completed += 1,
                TodoStatus::Blocked => s.blocked += 1,
                TodoStatus::Failed => s.failed += 1,
                TodoStatus::Cancelled => s.cancelled += 1,
            }
        }
        s
    }

    /// List all todos. Session-scoped when session_id is set, cross-session otherwise.
    pub async fn list_all(&self) -> Vec<TodoItem> {
        self.list_all_internal().await
    }

    // ── Goal management ─────────────────────────────────────────────────

    /// Submit a new high-level goal.
    pub async fn push_goal(&self, description: impl Into<String>) -> TodoGoal {
        let goal = TodoGoal::new(self.app_id.clone(), self.session_id.clone(), description);
        self.store.save_goal(&goal).await;
        info!(
            app_id = %self.app_id.0,
            session_id = ?self.session_id,
            goal_id = %goal.id,
            status = ?goal.status,
            "task space goal pushed"
        );
        goal
    }

    /// Pop the next pending goal for decomposition. Cross-session, for PlanLoop.
    pub async fn pop_goal(&self) -> Option<TodoGoal> {
        self.store.pop_pending_goal(&self.app_id).await
    }

    /// Mark a goal as completed.
    pub async fn complete_goal(&self, goal_id: &TaskId) {
        let goals = self.store.list_goals(&self.app_id).await;
        if let Some(mut goal) = goals.into_iter().find(|g| g.id == *goal_id) {
            goal.status = TodoGoalStatus::Completed;
            self.store.save_goal(&goal).await;
            info!(
                app_id = %self.app_id.0,
                goal_id = %goal.id,
                status = ?goal.status,
                "task space goal completed"
            );
        }
    }

    /// List all goals (cross-session).
    pub async fn list_goals(&self) -> Vec<TodoGoal> {
        self.store.list_goals(&self.app_id).await
    }

    /// Reassign a task from one agent to another.
    ///
    /// Deletes from old agent's key, updates `assigned_agent`, resets status
    /// to `Pending`, and saves under the new agent's key.
    pub async fn reassign_task(&self, task_id: &TaskId, old_agent: &str, new_agent: &str) -> bool {
        if let Some(mut task) = self
            .store
            .get_todo(&self.app_id, &self.session_id, old_agent, task_id)
            .await
        {
            // Delete from old agent's key
            self.store
                .delete_todo(&self.app_id, &self.session_id, old_agent, task_id)
                .await;

            // Update agent and reset status
            task.assigned_agent = new_agent.to_string();
            task.status = TodoStatus::Pending;
            task.updated_at = Utc::now();

            // Save under new agent's key (session_id is preserved in the item)
            self.store.save_todo(&task).await;
            info!(
                app_id = %self.app_id.0,
                session_id = ?self.session_id,
                task_id = %task.id,
                old_agent = %old_agent,
                new_agent = %new_agent,
                status = ?task.status,
                "task space task reassigned"
            );
            true
        } else {
            false
        }
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    /// Returns session-scoped todos when session_id is set, cross-session otherwise.
    async fn list_all_internal(&self) -> Vec<TodoItem> {
        match &self.session_id {
            Some(sid) => {
                self.store
                    .list_all_todos_for_session(&self.app_id, sid)
                    .await
            }
            None => self.store.list_all_todos(&self.app_id).await,
        }
    }
}
