//! Todo task board tools for Worker Agents and Plan Agents.
//!
//! Worker tools: claim_task, start_task, update_task_progress, submit_task_for_review, list_my_tasks
//! Plan tools:   create_todo, review_todo, check_todo_progress

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{MacacaResult, TaskId, TodoGoal, TodoItem, TodoStatus};
use macaca_task::{TaskBoard, TaskSpace};
use serde_json::{json, Value};

use crate::tool::Tool;

// ─────────────────────────────────────────────────────────────────────────────
// Worker Agent Tools
// ─────────────────────────────────────────────────────────────────────────────

/// Claim the highest-priority pending task from the agent's board.
pub struct ClaimTaskTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for ClaimTaskTool {
    fn name(&self) -> &str {
        "claim_task"
    }
    fn description(&self) -> &str {
        "Claim the highest-priority pending task from your task board. Returns the task details or null if no tasks available."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }
    async fn execute(&self, _input: Value) -> MacacaResult<Value> {
        match self.board.claim_next().await {
            Some(task) => Ok(json!({
                "task_id": task.id.to_string(),
                "title": task.title,
                "description": task.description,
                "acceptance_criteria": task.acceptance_criteria,
                "priority": task.priority,
                "context": task.context,
                "optimization_suggestions": task.optimization_suggestions,
                "attempt": task.attempt_count,
            })),
            None => {
                Ok(json!({ "status": "no_tasks", "message": "No pending tasks on your board" }))
            }
        }
    }
}

/// Mark a claimed task as in-progress.
pub struct StartTaskTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for StartTaskTool {
    fn name(&self) -> &str {
        "start_task"
    }
    fn description(&self) -> &str {
        "Mark a claimed task as in-progress. Call this after claim_task before starting work."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "task_id": { "type": "string", "description": "Task ID to start" } },
            "required": ["task_id"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(
            uuid::Uuid::parse_str(task_id_str)
                .map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?,
        );
        let ok = self.board.start_task(&task_id).await;
        Ok(json!({ "success": ok, "task_id": task_id_str }))
    }
}

/// Update progress on the current in-progress task.
pub struct UpdateTaskProgressTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for UpdateTaskProgressTool {
    fn name(&self) -> &str {
        "update_task_progress"
    }
    fn description(&self) -> &str {
        "Update progress on the current in-progress task."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID" },
                "message": { "type": "string", "description": "Progress update message" }
            },
            "required": ["task_id", "message"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(
            uuid::Uuid::parse_str(task_id_str)
                .map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?,
        );
        let message = input["message"].as_str().unwrap_or_default().to_string();
        let ok = self.board.update_progress(&task_id, message).await;
        Ok(json!({ "success": ok }))
    }
}

/// Submit a completed task for Plan Agent review.
pub struct SubmitTaskForReviewTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for SubmitTaskForReviewTool {
    fn name(&self) -> &str {
        "submit_task_for_review"
    }
    fn description(&self) -> &str {
        "Submit a completed task for review by the Plan Agent. Include a summary of what was done."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID to submit" },
                "summary": { "type": "string", "description": "Summary of completed work" }
            },
            "required": ["task_id", "summary"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(
            uuid::Uuid::parse_str(task_id_str)
                .map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?,
        );
        let summary = input["summary"].as_str().unwrap_or_default().to_string();
        let ok = self.board.submit_for_review(&task_id, summary).await;
        Ok(json!({ "success": ok, "status": if ok { "pending_review" } else { "error" } }))
    }
}

/// List all tasks on the agent's board.
pub struct ListMyTasksTool {
    pub board: Arc<TaskBoard>,
}

#[async_trait]
impl Tool for ListMyTasksTool {
    fn name(&self) -> &str {
        "list_my_tasks"
    }
    fn description(&self) -> &str {
        "List all tasks on your task board with their statuses."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }
    async fn execute(&self, _input: Value) -> MacacaResult<Value> {
        let tasks = self.board.list_all().await;
        let items: Vec<Value> = tasks
            .iter()
            .map(|t| {
                json!({
                    "task_id": t.id.to_string(),
                    "title": t.title,
                    "status": t.status,
                    "priority": t.priority,
                    "attempt_count": t.attempt_count,
                    "optimization_suggestions": t.optimization_suggestions,
                })
            })
            .collect();
        Ok(json!({ "agent": self.board.agent_name(), "tasks": items, "count": items.len() }))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Plan Agent Tools
// ─────────────────────────────────────────────────────────────────────────────

/// Create a new task and assign it to an agent's board.
pub struct CreateTodoTool {
    pub space: Arc<TaskSpace>,
    pub coordinator_name: String,
    /// Agents that should never receive executable TaskBoard tasks (e.g. entry/planner supervisors).
    #[allow(clippy::struct_field_names)]
    pub disallowed_assignees: Vec<String>,
    /// Agent capability profiles used for assignment validation and dependency inference.
    /// Key: agent name, Value: flattened capability texts.
    pub assignee_capabilities: HashMap<String, Vec<String>>,
    /// When set, every created todo will have `parent_task` = this goal id,
    /// allowing `PlanLoop` to detect that the goal has been decomposed.
    pub active_goal_id: Option<macaca_proto::TaskId>,
}

impl CreateTodoTool {
    fn normalize_title(title: &str) -> String {
        title.trim().to_ascii_lowercase()
    }

    fn tokenize(text: &str) -> HashSet<String> {
        const STOPWORDS: &[&str] = &[
            "the",
            "and",
            "for",
            "with",
            "todo",
            "task",
            "agent",
            "work",
            "this",
            "that",
            "from",
            "into",
            "your",
            "our",
        ];
        let stopwords: HashSet<&str> = STOPWORDS.iter().copied().collect();

        text.to_ascii_lowercase()
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .filter(|t| t.len() >= 3 && !stopwords.contains(*t))
            .map(ToString::to_string)
            .collect()
    }

    fn capability_score(task_tokens: &HashSet<String>, profile: &[String]) -> usize {
        let profile_tokens = Self::tokenize(&profile.join(" "));
        task_tokens.intersection(&profile_tokens).count()
    }

    fn is_foundation_profile(profile: &[String]) -> bool {
        let tokens = Self::tokenize(&profile.join(" "));
        let keywords = [
            "architecture",
            "architect",
            "design",
            "spec",
            "specification",
            "analysis",
            "analyze",
            "planning",
            "plan",
            "interface",
        ];
        keywords.iter().any(|k| tokens.contains(*k))
    }

    fn is_terminal_status(status: TodoStatus) -> bool {
        matches!(
            status,
            TodoStatus::Completed | TodoStatus::Cancelled | TodoStatus::Failed
        )
    }

    fn resolve_assignee(
        &self,
        requested: &str,
        title: &str,
        description: &str,
    ) -> (String, Option<String>) {
        if self.assignee_capabilities.is_empty() {
            return (requested.to_string(), None);
        }

        let task_tokens = Self::tokenize(&format!("{title}\n{description}"));
        if task_tokens.is_empty() {
            return (requested.to_string(), None);
        }

        let blocked: HashSet<&str> = self
            .disallowed_assignees
            .iter()
            .map(String::as_str)
            .collect();

        let requested_score = self
            .assignee_capabilities
            .get(requested)
            .map(|p| Self::capability_score(&task_tokens, p))
            .unwrap_or(0);

        let best = self
            .assignee_capabilities
            .iter()
            .filter(|(name, _)| !blocked.contains(name.as_str()))
            .map(|(name, profile)| (name.clone(), Self::capability_score(&task_tokens, profile)))
            .max_by_key(|(_, score)| *score);

        if let Some((best_agent, best_score)) = best {
            // Conservative reroute: only when requested has no semantic match and
            // another allowed agent has a clear positive match.
            if best_agent != requested && requested_score == 0 && best_score >= 2 {
                let reason = format!(
                    "rerouted by capability match (requested_score=0, best_agent={best_agent}, best_score={best_score})"
                );
                return (best_agent, Some(reason));
            }
        }

        (requested.to_string(), None)
    }

    fn resolve_title_dependencies(&self, titles: &[String], existing: &[TodoItem]) -> Vec<TaskId> {
        if titles.is_empty() {
            return Vec::new();
        }

        let mut by_title: HashMap<String, Vec<TaskId>> = HashMap::new();
        for item in existing {
            by_title
                .entry(Self::normalize_title(&item.title))
                .or_default()
                .push(item.id);
        }

        titles
            .iter()
            .flat_map(|title| {
                by_title
                    .get(&Self::normalize_title(title))
                    .cloned()
                    .unwrap_or_default()
            })
            .collect()
    }

    fn resolve_agent_ref_dependencies(
        &self,
        refs: &[macaca_proto::types::AgentTaskRef],
        existing: &[TodoItem],
    ) -> Vec<TaskId> {
        let mut out = Vec::new();
        for dep in refs {
            match dep {
                macaca_proto::types::AgentTaskRef::AllTasks { agent } => {
                    out.extend(
                        existing
                            .iter()
                            .filter(|t| t.assigned_agent == *agent)
                            .map(|t| t.id),
                    );
                }
                macaca_proto::types::AgentTaskRef::SpecificTask { agent, title } => {
                    let title_norm = Self::normalize_title(title);
                    out.extend(
                        existing
                            .iter()
                            .filter(|t| {
                                t.assigned_agent == *agent
                                    && Self::normalize_title(&t.title) == title_norm
                            })
                            .map(|t| t.id),
                    );
                }
            }
        }
        out
    }

    fn infer_foundation_dependencies(
        &self,
        assignee: &str,
        existing_scope: &[TodoItem],
        explicit: &HashSet<TaskId>,
    ) -> Vec<TaskId> {
        // Auto-inference only during active goal decomposition.
        if self.active_goal_id.is_none() {
            return Vec::new();
        }

        let Some(assignee_profile) = self.assignee_capabilities.get(assignee) else {
            return Vec::new();
        };
        if Self::is_foundation_profile(assignee_profile) {
            return Vec::new();
        }

        let foundation_agents: HashSet<String> = self
            .assignee_capabilities
            .iter()
            .filter_map(|(name, profile)| {
                if Self::is_foundation_profile(profile) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();
        if foundation_agents.is_empty() {
            return Vec::new();
        }

        existing_scope
            .iter()
            .filter(|t| {
                foundation_agents.contains(&t.assigned_agent)
                    && !Self::is_terminal_status(t.status)
                    && !explicit.contains(&t.id)
            })
            .map(|t| t.id)
            .collect()
    }
}

#[async_trait]
impl Tool for CreateTodoTool {
    fn name(&self) -> &str {
        "create_todo"
    }
    fn description(&self) -> &str {
        "Create a new task and assign it to a specific agent's task board."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "agent": { "type": "string", "description": "Target assignee agent name" },
                "title": { "type": "string", "description": "Short task title" },
                "description": { "type": "string", "description": "Detailed task description" },
                "acceptance_criteria": {
                    "type": "array", "items": { "type": "string" },
                    "description": "List of criteria that must be met for the task to pass review"
                },
                "priority": { "type": "integer", "description": "Priority 0-10, higher = more urgent", "default": 5 },
                "depends_on": {
                    "type": "array", "items": { "type": "string" },
                    "description": "Task IDs that must complete before this task can start"
                },
                "depends_on_titles": {
                    "type": "array", "items": { "type": "string" },
                    "description": "Task titles this task depends on (resolved within current scope)"
                },
                "depends_on_agents": {
                    "type": "array",
                    "description": "Cross-agent dependencies using symbolic references",
                    "items": {
                        "type": "object",
                        "properties": {
                            "type": { "type": "string", "enum": ["all_tasks", "specific_task"] },
                            "agent": { "type": "string" },
                            "title": { "type": "string" }
                        },
                        "required": ["type", "agent"]
                    }
                }
            },
            "required": ["agent", "title", "description"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let agent = input["agent"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| macaca_proto::MacacaError::Task("missing required field: agent".into()))?;
        let title = input["title"].as_str().unwrap_or_default();
        let description = input["description"].as_str().unwrap_or_default();
        let priority = input["priority"].as_u64().unwrap_or(5) as u8;
        let criteria: Vec<String> = input["acceptance_criteria"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let depends_on_ids: Vec<TaskId> = input["depends_on"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let s = v.as_str()?;
                        uuid::Uuid::parse_str(s).ok().map(TaskId)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let depends_on_titles: Vec<String> = input["depends_on_titles"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::trim))
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let depends_on_agents: Vec<macaca_proto::types::AgentTaskRef> = input["depends_on_agents"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        serde_json::from_value::<macaca_proto::types::AgentTaskRef>(v.clone()).ok()
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Goal decomposition must only create executable worker tasks.
        // Supervisors are resumed by orchestration and must not receive
        // TaskBoard work items, otherwise goals can deadlock waiting on
        // tasks that no WorkerLoop will claim.
        if self
            .disallowed_assignees
            .iter()
            .any(|blocked| blocked == agent)
        {
            return Err(macaca_proto::MacacaError::Task(format!(
                "create_todo cannot assign tasks to supervisor agent '{agent}'"
            )));
        }

        let (resolved_agent, routing_reason) = self.resolve_assignee(agent, title, description);
        let existing_scope: Vec<TodoItem> = {
            let existing = self.space.list_all().await;
            if let Some(goal_id) = self.active_goal_id {
                existing
                    .into_iter()
                    .filter(|t| t.parent_task == Some(goal_id))
                    .collect()
            } else {
                existing
            }
        };

        let mut deps: HashSet<TaskId> = depends_on_ids.into_iter().collect();
        for id in self.resolve_title_dependencies(&depends_on_titles, &existing_scope) {
            deps.insert(id);
        }
        for id in self.resolve_agent_ref_dependencies(&depends_on_agents, &existing_scope) {
            deps.insert(id);
        }
        let inferred = self.infer_foundation_dependencies(&resolved_agent, &existing_scope, &deps);
        for id in &inferred {
            deps.insert(*id);
        }
        let mut final_depends_on: Vec<TaskId> = deps.into_iter().collect();
        final_depends_on.sort_by_key(|id| id.to_string());

        let item = self
            .space
            .create_and_assign(
                &resolved_agent,
                &self.coordinator_name,
                title,
                description,
                criteria,
                priority,
                final_depends_on,
                self.active_goal_id,
            )
            .await;
        if self.active_goal_id.is_some() {
            tracing::info!(
                task_id = %item.id, agent = %agent, goal_id = ?self.active_goal_id,
                "Created todo with parent_task linked to goal"
            );
        }

        Ok(json!({
            "task_id": item.id.to_string(),
            "agent": resolved_agent,
            "requested_agent": agent,
            "status": item.status,
            "priority": priority,
            "dependency_count": item.depends_on.len(),
            "auto_inferred_dependencies": inferred.len(),
            "routing_reason": routing_reason,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_persist::RedbStore;
    use macaca_task::TodoStore;
    use tempfile::tempdir;

    #[tokio::test]
    async fn create_todo_rejects_supervisor_agents() {
        let dir = tempdir().expect("tempdir");
        let db = RedbStore::open(dir.path().join("todo-tests.redb")).expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let space = Arc::new(TaskSpace::new(
            macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
            Some("session".into()),
            store,
        ));
        let tool = CreateTodoTool {
            space,
            coordinator_name: "planner".into(),
            disallowed_assignees: vec!["coordinator".into(), "planner".into()],
            assignee_capabilities: HashMap::new(),
            active_goal_id: None,
        };

        let err = tool
            .execute(json!({
                "agent": "coordinator",
                "title": "Should fail",
                "description": "Coordinator must not get TaskBoard work"
            }))
            .await
            .expect_err("coordinator assignment should be rejected");

        assert!(
            err.to_string()
                .contains("cannot assign tasks to supervisor agent"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn create_todo_requires_agent_field() {
        let dir = tempdir().expect("tempdir");
        let db = RedbStore::open(dir.path().join("todo-tests-missing-agent.redb")).expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let space = Arc::new(TaskSpace::new(
            macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
            Some("session".into()),
            store,
        ));
        let tool = CreateTodoTool {
            space,
            coordinator_name: "entry_custom".into(),
            disallowed_assignees: vec![],
            assignee_capabilities: HashMap::new(),
            active_goal_id: None,
        };

        let err = tool
            .execute(json!({
                "title": "Missing agent",
                "description": "should fail when agent is not provided"
            }))
            .await
            .expect_err("missing agent must be rejected");

        assert!(
            err.to_string().contains("missing required field: agent"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn create_todo_reroutes_to_capability_matched_agent() {
        let dir = tempdir().expect("tempdir");
        let db = RedbStore::open(dir.path().join("todo-tests-reroute.redb")).expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let space = Arc::new(TaskSpace::new(
            macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
            Some("session".into()),
            store,
        ));
        let mut profiles = HashMap::new();
        profiles.insert(
            "architect".to_string(),
            vec![
                "design_analysis".into(),
                "architecture specification interface planning".into(),
            ],
        );
        profiles.insert(
            "backend".to_string(),
            vec![
                "backend_development".into(),
                "go golang rest api server database".into(),
            ],
        );
        let tool = CreateTodoTool {
            space: Arc::clone(&space),
            coordinator_name: "planner".into(),
            disallowed_assignees: vec![],
            assignee_capabilities: profiles,
            active_goal_id: None,
        };

        let out = tool
            .execute(json!({
                "agent": "architect",
                "title": "Implement Go backend API service",
                "description": "Build REST endpoints and database integration in golang"
            }))
            .await
            .expect("create_todo should succeed");

        assert_eq!(
            out["agent"].as_str().unwrap_or_default(),
            "backend",
            "should reroute to backend by capability match"
        );
        assert_eq!(
            out["requested_agent"].as_str().unwrap_or_default(),
            "architect"
        );
        let all = space.list_all().await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].assigned_agent, "backend");
    }

    #[tokio::test]
    async fn create_todo_auto_adds_foundation_dependencies_for_goal() {
        let dir = tempdir().expect("tempdir");
        let db = RedbStore::open(dir.path().join("todo-tests-foundation-deps.redb"))
            .expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let app_id = macaca_proto::ApplicationId(uuid::Uuid::new_v4());
        let goal_id = macaca_proto::TaskId::new();
        let space = Arc::new(TaskSpace::new(
            app_id,
            Some("session".into()),
            Arc::clone(&store),
        ));
        let mut profiles = HashMap::new();
        profiles.insert(
            "architect".to_string(),
            vec!["design_analysis architecture specification".into()],
        );
        profiles.insert(
            "frontend".to_string(),
            vec!["frontend_development react nextjs ui".into()],
        );
        let tool = CreateTodoTool {
            space: Arc::clone(&space),
            coordinator_name: "planner".into(),
            disallowed_assignees: vec![],
            assignee_capabilities: profiles,
            active_goal_id: Some(goal_id),
        };

        let arch = tool
            .execute(json!({
                "agent": "architect",
                "title": "Define architecture and interfaces",
                "description": "Produce design spec and API contracts"
            }))
            .await
            .expect("architect task create");
        let arch_id = macaca_proto::TaskId(
            uuid::Uuid::parse_str(arch["task_id"].as_str().unwrap_or_default())
                .expect("arch task id"),
        );

        let fe = tool
            .execute(json!({
                "agent": "frontend",
                "title": "Implement UI from spec",
                "description": "Build pages and connect to backend API"
            }))
            .await
            .expect("frontend task create");

        assert!(
            fe["auto_inferred_dependencies"].as_u64().unwrap_or_default() >= 1,
            "frontend task should get inferred dependency on architect task"
        );

        let all = space.list_all().await;
        let fe_task = all
            .into_iter()
            .find(|t| t.id.to_string() == fe["task_id"].as_str().unwrap_or_default())
            .expect("frontend task exists");
        assert_eq!(fe_task.status, TodoStatus::Blocked);
        assert!(
            fe_task.depends_on.contains(&arch_id),
            "frontend task should depend on architect task"
        );
    }
}

/// Fires after a successful `review_task` store update (for run_trace / analytics).
pub type OnTodoReviewed = Arc<dyn Fn(macaca_proto::TaskId, String, bool) + Send + Sync>;

/// Review a task submitted by an agent.
pub struct ReviewTodoTool {
    pub space: Arc<TaskSpace>,
    #[allow(clippy::type_complexity)]
    pub on_reviewed: Option<OnTodoReviewed>,
}

#[async_trait]
impl Tool for ReviewTodoTool {
    fn name(&self) -> &str {
        "review_todo"
    }
    fn description(&self) -> &str {
        "Review a submitted task. task_id must be a UUID from create_todo or list_agent_todos, not a title/slug."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "UUID string returned by create_todo, claim_task, or list_agent_todos — not a title or slug"
                },
                "agent": { "type": "string", "description": "Agent who owns the task" },
                "passed": { "type": "boolean", "description": "Whether the task passes review" },
                "feedback": { "type": "string", "description": "Review feedback or optimization suggestions" }
            },
            "required": ["task_id", "agent", "passed", "feedback"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"].as_str().unwrap_or_default();
        let task_id = macaca_proto::TaskId(
            uuid::Uuid::parse_str(task_id_str)
                .map_err(|_| macaca_proto::MacacaError::Task("invalid task_id".into()))?,
        );
        let agent = input["agent"].as_str().unwrap_or_default();
        let passed = input["passed"].as_bool().unwrap_or(false);
        let feedback = input["feedback"].as_str().unwrap_or_default().to_string();

        let result = macaca_proto::TodoReviewResult {
            passed,
            feedback: feedback.clone(),
            verified_criteria: vec![],
        };
        let ok = self.space.review_task(&task_id, agent, result).await;
        if ok {
            if let Some(ref cb) = self.on_reviewed {
                cb(task_id, agent.to_string(), passed);
            }
        }
        Ok(json!({
            "success": ok,
            "task_id": task_id_str,
            "passed": passed,
            "new_status": if passed { "completed" } else { "needs_optimization" },
        }))
    }
}

/// Callback invoked after a goal is created, allowing the web layer to
/// lazily start the PlanLoop without introducing a circular dependency.
pub type OnGoalCreated = Arc<dyn Fn() + Send + Sync>;

/// Called synchronously right after the goal is persisted (includes id + session_id).
pub type OnGoalRecorded = Arc<dyn Fn(TodoGoal) + Send + Sync>;

/// Create a high-level goal for the Plan Agent to decompose into tasks.
pub struct CreateGoalTool {
    pub space: Arc<TaskSpace>,
    /// Optional callback to trigger PlanLoop startup after goal creation.
    pub on_created: Option<OnGoalCreated>,
    /// Optional hook after the goal row exists (tracing).
    pub on_goal_recorded: Option<OnGoalRecorded>,
}

#[async_trait]
impl Tool for CreateGoalTool {
    fn name(&self) -> &str {
        "create_goal"
    }
    fn description(&self) -> &str {
        "Create a high-level project goal. The Plan Agent will automatically decompose it into concrete tasks and assign them to appropriate agents. Use this for complex multi-step work."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "The goal description"
                }
            },
            "required": ["description"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let description = input["description"].as_str().ok_or_else(|| {
            macaca_proto::MacacaError::Task("Missing 'description' parameter".into())
        })?;

        let goal = self.space.push_goal(description).await;

        if let Some(ref cb) = self.on_goal_recorded {
            cb(goal.clone());
        }

        // Trigger PlanLoop startup if callback is set
        if let Some(ref cb) = self.on_created {
            cb();
        }

        Ok(json!({
            "goal_id": goal.id.to_string(),
            "status": "pending",
            "message": "Goal created. The Plan Agent will decompose it into tasks."
        }))
    }
}

/// Reassign a task from one agent to another (Plan Agent only).
pub struct ReassignTaskTool {
    pub space: Arc<TaskSpace>,
}

#[async_trait]
impl Tool for ReassignTaskTool {
    fn name(&self) -> &str {
        "reassign_task"
    }
    fn description(&self) -> &str {
        "Reassign a task from one agent to another. The task status is reset to Pending so the new agent can claim it. Use when an agent cannot complete a task or the task was misrouted."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "The task ID to reassign" },
                "current_agent": { "type": "string", "description": "The agent currently assigned to the task" },
                "new_agent": { "type": "string", "description": "The agent to reassign the task to" }
            },
            "required": ["task_id", "current_agent", "new_agent"]
        })
    }
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id_str = input["task_id"]
            .as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Task("Missing 'task_id'".into()))?;
        let uuid = uuid::Uuid::parse_str(task_id_str).map_err(|_| {
            macaca_proto::MacacaError::Task(format!("Invalid task_id: {}", task_id_str))
        })?;
        let task_id = macaca_proto::TaskId(uuid);
        let current_agent = input["current_agent"]
            .as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Task("Missing 'current_agent'".into()))?;
        let new_agent = input["new_agent"]
            .as_str()
            .ok_or_else(|| macaca_proto::MacacaError::Task("Missing 'new_agent'".into()))?;

        let success = self
            .space
            .reassign_task(&task_id, current_agent, new_agent)
            .await;

        if success {
            Ok(json!({
                "task_id": task_id_str,
                "reassigned_from": current_agent,
                "reassigned_to": new_agent,
                "new_status": "pending"
            }))
        } else {
            Err(macaca_proto::MacacaError::NotFound(format!(
                "Task {} not found on agent {}'s board",
                task_id_str, current_agent
            )))
        }
    }
}

/// Check overall progress of all tasks in the application.
pub struct CheckTodoProgressTool {
    pub space: Arc<TaskSpace>,
}

#[async_trait]
impl Tool for CheckTodoProgressTool {
    fn name(&self) -> &str {
        "check_todo_progress"
    }
    fn description(&self) -> &str {
        "Check the overall progress of all tasks across all agents. When pending_review > 0, the response includes `pending_review_tasks` with `task_id` (UUID) for each task — use these with `review_todo`."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }
    async fn execute(&self, _input: Value) -> MacacaResult<Value> {
        let p = self.space.overall_progress().await;
        let reviews = self.space.pending_reviews().await;
        let pending_review_tasks: Vec<Value> = reviews
            .into_iter()
            .take(50)
            .map(|t| {
                json!({
                    "task_id": t.id.to_string(),
                    "title": t.title,
                    "assigned_agent": t.assigned_agent,
                    "session_id": t.session_id,
                })
            })
            .collect();
        Ok(json!({
            "total": p.total,
            "pending": p.pending,
            "assigned": p.assigned,
            "in_progress": p.in_progress,
            "pending_review": p.pending_review,
            "pending_review_tasks": pending_review_tasks,
            "needs_optimization": p.needs_optimization,
            "completed": p.completed,
            "blocked": p.blocked,
            "failed": p.failed,
            "cancelled": p.cancelled,
            "all_done": p.completed + p.cancelled + p.failed == p.total && p.total > 0,
        }))
    }
}
