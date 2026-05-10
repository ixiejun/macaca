//! TaskBoard (per-agent panel) and TaskSpace (per-application workspace).
//!
//! - Each agent has its own TaskBoard (isolated from other agents).
//! - The Plan Agent / Coordinator accesses TaskSpace which spans all boards.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::Utc;
use macaca_proto::{
    ApplicationId, TaskId, TodoGoal, TodoGoalStatus, TodoItem, TodoReviewResult, TodoStatus,
};

use crate::dependency::{DefaultTaskDependencyResolver, TaskDependencyResolver};
use crate::lifecycle::{DefaultTodoLifecyclePolicy, TodoLifecyclePolicy};
use crate::todo_store::TodoStore;

/// Detect cycles in a dependency graph. Returns Err with a message if a cycle is found.
pub fn detect_cycles(tasks: &[TodoItem]) -> Result<(), String> {
    let id_set: HashSet<TaskId> = tasks.iter().map(|t| t.id).collect();
    let mut visited = HashSet::new();
    let mut in_stack = HashSet::new();

    fn dfs(
        node: TaskId,
        deps: &HashMap<TaskId, Vec<TaskId>>,
        visited: &mut HashSet<TaskId>,
        in_stack: &mut HashSet<TaskId>,
    ) -> Result<(), String> {
        visited.insert(node);
        in_stack.insert(node);

        if let Some(neighbors) = deps.get(&node) {
            for &dep in neighbors {
                if in_stack.contains(&dep) {
                    return Err(format!("Cycle detected involving task {}", dep));
                }
                if !visited.contains(&dep) {
                    dfs(dep, deps, visited, in_stack)?;
                }
            }
        }

        in_stack.remove(&node);
        Ok(())
    }

    // Build adjacency: task -> its dependencies (only within this batch)
    let deps_map: HashMap<TaskId, Vec<TaskId>> = tasks
        .iter()
        .map(|t| {
            let relevant_deps: Vec<TaskId> = t
                .depends_on
                .iter()
                .filter(|d| id_set.contains(d))
                .copied()
                .collect();
            (t.id, relevant_deps)
        })
        .collect();

    for task in tasks {
        if !visited.contains(&task.id) {
            dfs(task.id, &deps_map, &mut visited, &mut in_stack)?;
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// TaskBoard — an individual agent's task panel
// ─────────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// TaskSpace — the application-level workspace (Plan Agent view)
// ─────────────────────────────────────────────────────────────────────────────

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
        item.acceptance_criteria = acceptance_criteria;
        item.depends_on = depends_on.clone();
        item.parent_task = parent_task;
        let all_todos = self.list_all_internal().await;
        item.status = self
            .dependency_resolver
            .initial_status_for_new_task(&depends_on, &all_todos);

        self.store.save_todo(&item).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_persist::RedbStore;
    use tempfile::tempdir;

    async fn setup() -> (ApplicationId, Arc<TodoStore>) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.redb");
        let dir = Box::leak(Box::new(dir));
        let _ = dir;
        let store = Arc::new(RedbStore::open(db_path).unwrap());
        let todo_store = Arc::new(TodoStore::new(store));
        (ApplicationId::new(), todo_store)
    }

    #[tokio::test]
    async fn board_claim_by_sequence_number() {
        let (app_id, store) = setup().await;
        let board = TaskBoard::for_agent(app_id.clone(), "backend", None, Arc::clone(&store));
        let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

        // Created in order: seq 1, 2, 3 (auto-assigned)
        space
            .create_task_assignment(
                "backend",
                "coord",
                "First task",
                "desc",
                vec![],
                3,
                vec![],
                None,
            )
            .await;
        space
            .create_task_assignment(
                "backend",
                "coord",
                "Second task",
                "desc",
                vec![],
                9,
                vec![],
                None,
            )
            .await;
        space
            .create_task_assignment(
                "backend",
                "coord",
                "Third task",
                "desc",
                vec![],
                5,
                vec![],
                None,
            )
            .await;

        // Should claim by sequence order, not priority
        let claimed = board.claim_next_task().await.unwrap();
        assert_eq!(claimed.title, "First task");
        assert_eq!(claimed.sequence_number, 1);
        assert_eq!(claimed.status, TodoStatus::Assigned);

        // seq 1 is now Assigned (not terminal), so claim_next blocks
        let claimed2 = board.claim_next_task().await;
        assert!(claimed2.is_none(), "Should block because seq 1 is Assigned");
    }

    #[tokio::test]
    async fn board_submit_and_review() {
        let (app_id, store) = setup().await;
        let board = TaskBoard::for_agent(app_id.clone(), "backend", None, Arc::clone(&store));
        let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

        let task = space
            .create_task_assignment(
                "backend",
                "coord",
                "Write API",
                "Create REST",
                vec!["returns 200".into()],
                7,
                vec![],
                None,
            )
            .await;

        // Agent claims and starts
        board.claim_next_task().await;
        board.mark_task_in_progress(&task.id).await;

        // Agent submits for review
        board
            .submit_task_for_review(&task.id, "Done, all tests pass".into())
            .await;

        // Plan Agent reviews — pass
        let result = TodoReviewResult {
            passed: true,
            feedback: "Looks good".into(),
            verified_criteria: vec![("returns 200".into(), true)],
        };
        space.apply_review_result(&task.id, "backend", result).await;

        let updated = store
            .get_todo(&app_id, &None, "backend", &task.id)
            .await
            .unwrap();
        assert_eq!(updated.status, TodoStatus::Completed);
    }

    #[tokio::test]
    async fn review_fail_triggers_optimization() {
        let (app_id, store) = setup().await;
        let board = TaskBoard::for_agent(app_id.clone(), "backend", None, Arc::clone(&store));
        let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

        let task = space
            .create_task_assignment(
                "backend",
                "coord",
                "Fix bug",
                "Fix it",
                vec![],
                5,
                vec![],
                None,
            )
            .await;
        board.claim_next_task().await;
        board.mark_task_in_progress(&task.id).await;
        board
            .submit_task_for_review(&task.id, "I think it's fixed".into())
            .await;

        let result = TodoReviewResult {
            passed: false,
            feedback: "Missing edge case handling".into(),
            verified_criteria: vec![],
        };
        space.apply_review_result(&task.id, "backend", result).await;

        let updated = store
            .get_todo(&app_id, &None, "backend", &task.id)
            .await
            .unwrap();
        assert_eq!(updated.status, TodoStatus::NeedsOptimization);
        assert!(updated.optimization_suggestions.is_some());
    }

    #[tokio::test]
    async fn dependency_blocking_and_unblocking() {
        let (app_id, store) = setup().await;
        let board = TaskBoard::for_agent(app_id.clone(), "backend", None, Arc::clone(&store));
        let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

        // Task A (no deps)
        let task_a = space
            .create_task_assignment(
                "backend",
                "coord",
                "Task A",
                "First",
                vec![],
                9,
                vec![],
                None,
            )
            .await;
        // Task B depends on A
        let task_b = space
            .create_task_assignment(
                "backend",
                "coord",
                "Task B",
                "Second",
                vec![],
                7,
                vec![task_a.id],
                None,
            )
            .await;

        // B should be blocked
        let b = store
            .get_todo(&app_id, &None, "backend", &task_b.id)
            .await
            .unwrap();
        assert_eq!(b.status, TodoStatus::Blocked);

        // Complete A
        board.claim_next_task().await;
        board.mark_task_in_progress(&task_a.id).await;
        board
            .submit_task_for_review(&task_a.id, "Done".into())
            .await;
        space
            .apply_review_result(
                &task_a.id,
                "backend",
                TodoReviewResult {
                    passed: true,
                    feedback: "OK".into(),
                    verified_criteria: vec![],
                },
            )
            .await;

        // B should now be Pending
        let b = store
            .get_todo(&app_id, &None, "backend", &task_b.id)
            .await
            .unwrap();
        assert_eq!(b.status, TodoStatus::Pending);
    }

    #[tokio::test]
    async fn progress_summary() {
        let (app_id, store) = setup().await;
        let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

        space
            .create_task_assignment("backend", "coord", "T1", "d", vec![], 5, vec![], None)
            .await;
        space
            .create_task_assignment("backend", "coord", "T2", "d", vec![], 5, vec![], None)
            .await;
        space
            .create_task_assignment("frontend", "coord", "T3", "d", vec![], 5, vec![], None)
            .await;

        let progress = space.overall_progress().await;
        assert_eq!(progress.total, 3);
        assert_eq!(progress.pending, 3);
        assert!(space.all_tasks_done().await == false);
    }

    #[tokio::test]
    async fn goal_lifecycle() {
        let (app_id, store) = setup().await;
        let space = TaskSpace::for_session(app_id, None, Arc::clone(&store));

        let goal = space.push_goal("Build auth system").await;
        assert_eq!(space.list_goals().await.len(), 1);

        let popped = space.pop_goal().await.unwrap();
        assert_eq!(popped.description, "Build auth system");

        space.complete_goal(&goal.id).await;
        let goals = space.list_goals().await;
        assert_eq!(goals[0].status, TodoGoalStatus::Completed);
    }

    #[tokio::test]
    async fn board_does_not_claim_goal_tasks_until_goal_in_progress() {
        let (app_id, store) = setup().await;
        let board = TaskBoard::for_agent(app_id.clone(), "backend", None, Arc::clone(&store));
        let space = TaskSpace::for_session(app_id.clone(), None, Arc::clone(&store));

        let goal = space.push_goal("Build feature").await;
        let _ = space
            .pop_goal()
            .await
            .expect("goal should move to decomposing");
        space
            .create_task_assignment(
                "backend",
                "planner",
                "Implement backend",
                "Build API",
                vec![],
                5,
                vec![],
                Some(goal.id),
            )
            .await;

        assert!(
            board.claim_next_task().await.is_none(),
            "tasks under a Decomposing goal must not be claimed"
        );

        store
            .update_goal_status(&app_id, &goal.id, TodoGoalStatus::InProgress)
            .await;
        let claimed = board
            .claim_next_task()
            .await
            .expect("task should be claimable after goal enters InProgress");
        assert_eq!(claimed.title, "Implement backend");
        assert_eq!(claimed.status, TodoStatus::Assigned);
    }

    #[tokio::test]
    async fn session_scoped_space() {
        let (app_id, store) = setup().await;
        let sess_a = Some("session-a".to_string());
        let sess_b = Some("session-b".to_string());

        let space_a = TaskSpace::for_session(app_id.clone(), sess_a, Arc::clone(&store));
        let space_b = TaskSpace::for_session(app_id.clone(), sess_b, Arc::clone(&store));

        space_a
            .create_task_assignment(
                "backend",
                "coord",
                "Task A",
                "desc",
                vec![],
                5,
                vec![],
                None,
            )
            .await;
        space_b
            .create_task_assignment(
                "backend",
                "coord",
                "Task B",
                "desc",
                vec![],
                5,
                vec![],
                None,
            )
            .await;

        // Each space only sees its own session's tasks
        let a_tasks = space_a.list_all().await;
        assert_eq!(a_tasks.len(), 1);
        assert_eq!(a_tasks[0].title, "Task A");

        let b_tasks = space_b.list_all().await;
        assert_eq!(b_tasks.len(), 1);
        assert_eq!(b_tasks[0].title, "Task B");
    }
}
