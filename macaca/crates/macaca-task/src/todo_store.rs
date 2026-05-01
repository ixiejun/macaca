//! Persistent storage for the Todo task board system.
//!
//! Uses RedbStore with prefix-based keys for application, session, and agent isolation:
//! - `todo/{app_id}/{session_id}/{agent}/{task_id}` → TodoItem
//! - `goal/{app_id}/{session_id}/{goal_id}`         → TodoGoal
//!
//! When session_id is None, the literal segment `_global_` is used so that
//! existing cross-session queries (PlanLoop, WorkerLoop) continue to work by
//! scanning the full `todo/{app_id}/` prefix.

use std::sync::Arc;

use macaca_persist::PersistBackend;
use macaca_proto::{ApplicationId, TaskId, TodoGoal, TodoGoalStatus, TodoItem, TodoStatus};

/// Key prefixes for todo storage.
const TODO_PREFIX: &str = "todo/";
const GOAL_PREFIX: &str = "goal/";

/// Segment used in storage keys when no session_id is present.
const GLOBAL_SESSION: &str = "_global_";

/// Persistent store for TodoItems and TodoGoals.
///
/// Every status change is immediately persisted — no batching, no timers.
/// This ensures any restart recovers the full task board state.
pub struct TodoStore {
    store: Arc<dyn PersistBackend>,
}

impl TodoStore {
    pub fn new(store: Arc<dyn PersistBackend>) -> Self {
        Self { store }
    }

    // ── Key helpers ──────────────────────────────────────────────────────

    fn session_seg(session_id: &Option<String>) -> &str {
        session_id.as_deref().unwrap_or(GLOBAL_SESSION)
    }

    fn todo_key(
        app_id: &ApplicationId,
        session_id: &Option<String>,
        agent: &str,
        task_id: &TaskId,
    ) -> String {
        let sess = Self::session_seg(session_id);
        format!("{}{}/{}/{}/{}", TODO_PREFIX, app_id, sess, agent, task_id)
    }

    fn agent_prefix(app_id: &ApplicationId, session_id: &Option<String>, agent: &str) -> String {
        let sess = Self::session_seg(session_id);
        format!("{}{}/{}/{}/", TODO_PREFIX, app_id, sess, agent)
    }

    fn session_prefix(app_id: &ApplicationId, session_id: &str) -> String {
        format!("{}{}/{}/", TODO_PREFIX, app_id, session_id)
    }

    fn app_prefix(app_id: &ApplicationId) -> String {
        format!("{}{}/", TODO_PREFIX, app_id)
    }

    fn goal_key(app_id: &ApplicationId, session_id: &Option<String>, goal_id: &TaskId) -> String {
        let sess = Self::session_seg(session_id);
        format!("{}{}/{}/{}", GOAL_PREFIX, app_id, sess, goal_id)
    }

    fn goal_prefix_for_session(app_id: &ApplicationId, session_id: &str) -> String {
        format!("{}{}/{}/", GOAL_PREFIX, app_id, session_id)
    }

    fn goal_prefix(app_id: &ApplicationId) -> String {
        format!("{}{}/", GOAL_PREFIX, app_id)
    }

    // ── TodoItem CRUD ───────────────────────────────────────────────────

    /// Save (create or update) a todo item. Called on every status change.
    pub async fn save_todo(&self, item: &TodoItem) {
        let key = Self::todo_key(
            &item.application_id,
            &item.session_id,
            &item.assigned_agent,
            &item.id,
        );
        if let Ok(data) = serde_json::to_vec(item) {
            let _ = self.store.set(&key, &data).await;
        }
    }

    /// Get a single todo by ID.
    /// When session_id is None (cross-session mode), searches all sessions for the task.
    pub async fn get_todo(
        &self,
        app_id: &ApplicationId,
        session_id: &Option<String>,
        agent: &str,
        task_id: &TaskId,
    ) -> Option<TodoItem> {
        if session_id.is_some() {
            let key = Self::todo_key(app_id, session_id, agent, task_id);
            return self
                .store
                .get(&key)
                .await
                .ok()
                .flatten()
                .and_then(|data| serde_json::from_slice(&data).ok());
        }
        // Cross-session: scan all todos for this app+agent and find by id
        self.list_agent_todos(app_id, &None, agent)
            .await
            .into_iter()
            .find(|t| t.id == *task_id)
    }

    /// Delete a todo item.
    /// When session_id is None, looks up the item first to find its actual session_id.
    pub async fn delete_todo(
        &self,
        app_id: &ApplicationId,
        session_id: &Option<String>,
        agent: &str,
        task_id: &TaskId,
    ) {
        if session_id.is_some() {
            let key = Self::todo_key(app_id, session_id, agent, task_id);
            let _ = self.store.delete(&key).await;
        } else {
            // Find the item to get its real session_id for the correct key
            if let Some(item) = self.get_todo(app_id, &None, agent, task_id).await {
                let key = Self::todo_key(app_id, &item.session_id, agent, task_id);
                let _ = self.store.delete(&key).await;
            }
        }
    }

    /// List all todos for a specific agent within an application and session.
    /// When session_id is None, scans ALL sessions for this agent (cross-session mode).
    pub async fn list_agent_todos(
        &self,
        app_id: &ApplicationId,
        session_id: &Option<String>,
        agent: &str,
    ) -> Vec<TodoItem> {
        if session_id.is_some() {
            // Session-scoped: exact prefix match
            let prefix = Self::agent_prefix(app_id, session_id, agent);
            self.load_items_by_prefix(&prefix).await
        } else {
            // Cross-session: scan all todos for this app, filter by agent
            self.list_all_todos(app_id)
                .await
                .into_iter()
                .filter(|t| t.assigned_agent == agent)
                .collect()
        }
    }

    /// List agent todos filtered by status.
    pub async fn list_agent_todos_by_status(
        &self,
        app_id: &ApplicationId,
        session_id: &Option<String>,
        agent: &str,
        status: TodoStatus,
    ) -> Vec<TodoItem> {
        self.list_agent_todos(app_id, session_id, agent)
            .await
            .into_iter()
            .filter(|t| t.status == status)
            .collect()
    }

    /// List ALL todos across all agents and sessions for an application (Plan Agent use).
    pub async fn list_all_todos(&self, app_id: &ApplicationId) -> Vec<TodoItem> {
        let prefix = Self::app_prefix(app_id);
        self.load_items_by_prefix(&prefix).await
    }

    /// List all todos for a specific session (cross-agent, single session).
    pub async fn list_all_todos_for_session(
        &self,
        app_id: &ApplicationId,
        session_id: &str,
    ) -> Vec<TodoItem> {
        let prefix = Self::session_prefix(app_id, session_id);
        self.load_items_by_prefix(&prefix).await
    }

    /// List todos awaiting Plan Agent review across all agents/sessions.
    pub async fn list_pending_reviews(&self, app_id: &ApplicationId) -> Vec<TodoItem> {
        self.list_all_todos(app_id)
            .await
            .into_iter()
            .filter(|t| t.status == TodoStatus::PendingReview)
            .collect()
    }

    /// Rollback any IN_PROGRESS tasks to PENDING (for crash recovery).
    pub async fn rollback_in_progress(&self, app_id: &ApplicationId) {
        let all = self.list_all_todos(app_id).await;
        for mut item in all {
            if item.status == TodoStatus::InProgress || item.status == TodoStatus::Assigned {
                item.status = TodoStatus::Pending;
                item.updated_at = chrono::Utc::now();
                self.save_todo(&item).await;
            }
        }
    }

    /// Get the maximum sequence_number for a given agent+session scope.
    /// Returns 0 if no tasks exist.
    pub async fn get_max_sequence_number(
        &self,
        app_id: &ApplicationId,
        session_id: &Option<String>,
        agent: &str,
    ) -> u32 {
        self.list_agent_todos(app_id, session_id, agent)
            .await
            .iter()
            .map(|t| t.sequence_number)
            .max()
            .unwrap_or(0)
    }

    /// Migrate legacy TodoItems that have sequence_number == 0.
    /// Assigns sequence numbers based on created_at order within each agent+session.
    /// Runs once per app_id; uses a meta key to avoid re-running.
    pub async fn migrate_sequence_numbers(&self, app_id: &ApplicationId) {
        let meta_key = format!("meta/{}/seq_migrated", app_id);
        if let Ok(Some(_)) = self.store.get(&meta_key).await {
            return; // Already migrated
        }

        let all = self.list_all_todos(app_id).await;
        let needs_migration: Vec<&TodoItem> =
            all.iter().filter(|t| t.sequence_number == 0).collect();

        if needs_migration.is_empty() {
            // Mark as migrated even if nothing to do
            let _ = self.store.set(&meta_key, b"1").await;
            return;
        }

        // Group by (session_id, agent) and sort by created_at
        let mut groups: std::collections::HashMap<(String, String), Vec<TodoItem>> =
            std::collections::HashMap::new();
        for item in needs_migration {
            let key = (
                item.session_id.clone().unwrap_or_else(|| "_global_".into()),
                item.assigned_agent.clone(),
            );
            groups.entry(key).or_default().push(item.clone());
        }

        for (_key, mut items) in groups {
            items.sort_by_key(|t| t.created_at);
            for (i, mut item) in items.into_iter().enumerate() {
                item.sequence_number = (i + 1) as u32;
                self.save_todo(&item).await;
            }
        }

        let _ = self.store.set(&meta_key, b"1").await;
        tracing::info!(app_id = %app_id, "Migrated legacy tasks with sequence_number = 0");
    }

    // ── TodoGoal CRUD ───────────────────────────────────────────────────

    /// Save a goal.
    pub async fn save_goal(&self, goal: &TodoGoal) {
        let key = Self::goal_key(&goal.application_id, &goal.session_id, &goal.id);
        if let Ok(data) = serde_json::to_vec(goal) {
            let _ = self.store.set(&key, &data).await;
        }
    }

    /// Update a goal's status by ID.
    pub async fn update_goal_status(
        &self,
        app_id: &ApplicationId,
        goal_id: &macaca_proto::TaskId,
        status: TodoGoalStatus,
    ) {
        let goals = self.list_goals(app_id).await;
        if let Some(mut goal) = goals.into_iter().find(|g| g.id == *goal_id) {
            goal.status = status;
            self.save_goal(&goal).await;
        }
    }

    /// Pop the next pending goal (FIFO by created_at). Cross-session, for PlanLoop.
    pub async fn pop_pending_goal(&self, app_id: &ApplicationId) -> Option<TodoGoal> {
        let goals = self.list_goals(app_id).await;
        let mut pending: Vec<_> = goals
            .into_iter()
            .filter(|g| g.status == TodoGoalStatus::Pending)
            .collect();
        pending.sort_by_key(|g| g.created_at);
        if let Some(mut goal) = pending.into_iter().next() {
            goal.status = TodoGoalStatus::Decomposing;
            self.save_goal(&goal).await;
            Some(goal)
        } else {
            None
        }
    }

    /// List all goals for an application (cross-session).
    pub async fn list_goals(&self, app_id: &ApplicationId) -> Vec<TodoGoal> {
        let prefix = Self::goal_prefix(app_id);
        let keys = self.store.list_keys(&prefix).await.unwrap_or_default();
        let mut goals = Vec::new();
        for key in keys {
            if let Ok(Some(data)) = self.store.get(&key).await {
                if let Ok(goal) = serde_json::from_slice::<TodoGoal>(&data) {
                    goals.push(goal);
                }
            }
        }
        goals
    }

    /// List all goals for a specific session.
    pub async fn list_goals_for_session(
        &self,
        app_id: &ApplicationId,
        session_id: &str,
    ) -> Vec<TodoGoal> {
        let prefix = Self::goal_prefix_for_session(app_id, session_id);
        let keys = self.store.list_keys(&prefix).await.unwrap_or_default();
        let mut goals = Vec::new();
        for key in keys {
            if let Ok(Some(data)) = self.store.get(&key).await {
                if let Ok(goal) = serde_json::from_slice::<TodoGoal>(&data) {
                    goals.push(goal);
                }
            }
        }
        goals
    }

    // ── Internal helpers ────────────────────────────────────────────────

    async fn load_items_by_prefix(&self, prefix: &str) -> Vec<TodoItem> {
        let keys = self.store.list_keys(prefix).await.unwrap_or_default();
        let mut items = Vec::new();
        for key in keys {
            if let Ok(Some(data)) = self.store.get(&key).await {
                if let Ok(item) = serde_json::from_slice::<TodoItem>(&data) {
                    items.push(item);
                }
            }
        }
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_persist::RedbStore;
    use tempfile::tempdir;

    async fn test_store() -> TodoStore {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.redb");
        // Keep dir alive by leaking (test only)
        let dir = Box::leak(Box::new(dir));
        let _ = dir;
        let store = RedbStore::open(db_path).unwrap();
        TodoStore::new(Arc::new(store))
    }

    #[tokio::test]
    async fn save_and_load_todo() {
        let store = test_store().await;
        let app_id = ApplicationId::new();
        let item = TodoItem::new(
            app_id.clone(),
            None,
            "backend",
            "coordinator",
            "Write API",
            "Create REST API",
            8,
        );
        let task_id = item.id;

        store.save_todo(&item).await;

        let loaded = store.get_todo(&app_id, &None, "backend", &task_id).await;
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.title, "Write API");
        assert_eq!(loaded.priority, 8);
    }

    #[tokio::test]
    async fn list_by_agent_and_status() {
        let store = test_store().await;
        let app_id = ApplicationId::new();

        let item1 = TodoItem::new(
            app_id.clone(),
            None,
            "backend",
            "coord",
            "Task 1",
            "Desc 1",
            5,
        );
        let mut item2 = TodoItem::new(
            app_id.clone(),
            None,
            "backend",
            "coord",
            "Task 2",
            "Desc 2",
            9,
        );
        item2.status = TodoStatus::InProgress;
        let item3 = TodoItem::new(
            app_id.clone(),
            None,
            "frontend",
            "coord",
            "Task 3",
            "Desc 3",
            7,
        );

        store.save_todo(&item1).await;
        store.save_todo(&item2).await;
        store.save_todo(&item3).await;

        let backend_todos = store.list_agent_todos(&app_id, &None, "backend").await;
        assert_eq!(backend_todos.len(), 2);

        let frontend_todos = store.list_agent_todos(&app_id, &None, "frontend").await;
        assert_eq!(frontend_todos.len(), 1);

        let pending = store
            .list_agent_todos_by_status(&app_id, &None, "backend", TodoStatus::Pending)
            .await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].title, "Task 1");
    }

    #[tokio::test]
    async fn rollback_in_progress() {
        let store = test_store().await;
        let app_id = ApplicationId::new();

        let mut item = TodoItem::new(
            app_id.clone(),
            None,
            "backend",
            "coord",
            "Running task",
            "Desc",
            5,
        );
        item.status = TodoStatus::InProgress;
        store.save_todo(&item).await;

        store.rollback_in_progress(&app_id).await;

        let loaded = store
            .get_todo(&app_id, &None, "backend", &item.id)
            .await
            .unwrap();
        assert_eq!(loaded.status, TodoStatus::Pending);
    }

    #[tokio::test]
    async fn goal_lifecycle() {
        let store = test_store().await;
        let app_id = ApplicationId::new();

        let goal = TodoGoal::new(app_id.clone(), None, "Build auth system");
        store.save_goal(&goal).await;

        let goals = store.list_goals(&app_id).await;
        assert_eq!(goals.len(), 1);

        let popped = store.pop_pending_goal(&app_id).await;
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().status, TodoGoalStatus::Decomposing);

        // No more pending
        let popped2 = store.pop_pending_goal(&app_id).await;
        assert!(popped2.is_none());
    }

    #[tokio::test]
    async fn session_isolation() {
        let store = test_store().await;
        let app_id = ApplicationId::new();
        let sess_a = Some("session-a".to_string());
        let sess_b = Some("session-b".to_string());

        let item_a = TodoItem::new(
            app_id.clone(),
            sess_a.clone(),
            "backend",
            "coord",
            "Task A",
            "Desc",
            5,
        );
        let item_b = TodoItem::new(
            app_id.clone(),
            sess_b.clone(),
            "backend",
            "coord",
            "Task B",
            "Desc",
            5,
        );

        store.save_todo(&item_a).await;
        store.save_todo(&item_b).await;

        // Session-scoped queries are isolated
        let sess_a_items = store.list_all_todos_for_session(&app_id, "session-a").await;
        assert_eq!(sess_a_items.len(), 1);
        assert_eq!(sess_a_items[0].title, "Task A");

        let sess_b_items = store.list_all_todos_for_session(&app_id, "session-b").await;
        assert_eq!(sess_b_items.len(), 1);
        assert_eq!(sess_b_items[0].title, "Task B");

        // Cross-session query returns both
        let all = store.list_all_todos(&app_id).await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn goal_session_isolation() {
        let store = test_store().await;
        let app_id = ApplicationId::new();
        let sess = Some("sess-1".to_string());

        let goal = TodoGoal::new(app_id.clone(), sess.clone(), "Scoped goal");
        store.save_goal(&goal).await;

        let scoped = store.list_goals_for_session(&app_id, "sess-1").await;
        assert_eq!(scoped.len(), 1);

        let all = store.list_goals(&app_id).await;
        assert_eq!(all.len(), 1);
    }
}
