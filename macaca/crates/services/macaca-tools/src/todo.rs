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
        match self.board.claim_next_task().await {
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
        let ok = self.board.mark_task_in_progress(&task_id).await;
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
        let ok = self.board.submit_task_for_review(&task_id, summary).await;
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
            "the", "and", "for", "with", "todo", "task", "agent", "work", "this", "that", "from",
            "into", "your", "our",
        ];
        let stopwords: HashSet<&str> = STOPWORDS.iter().copied().collect();

        let lower = text.to_ascii_lowercase();
        let mut tokens: HashSet<String> = lower
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .filter(|t| t.len() >= 3 && !stopwords.contains(*t))
            .map(ToString::to_string)
            .collect();

        // Add lightweight multilingual capability hints for profile
        // classification (e.g. foundation dependency inference).
        //
        // Hints use provider-neutral capability dimensions only — never
        // application-specific agent role names (Strategy: capability tagging).
        for (needle, hints) in [
            ("架构", &["architecture", "design"][..]),
            ("设计", &["design"][..]),
            ("规范", &["specification", "spec"][..]),
            ("接口", &["interface", "api"][..]),
            ("数据模型", &["data", "model"][..]),
            ("技术风险", &["technical", "risk", "analysis"][..]),
            ("前端", &["ui", "presentation"][..]),
            ("后端", &["api", "service"][..]),
        ] {
            if text.contains(needle) {
                tokens.extend(hints.iter().map(|hint| hint.to_string()));
            }
        }

        tokens
    }

    fn is_foundation_profile(profile: &[String]) -> bool {
        let tokens = Self::tokenize(&profile.join(" "));
        let keywords = [
            "architecture",
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

    async fn create_one(&self, input: Value) -> MacacaResult<Value> {
        let agent = input["agent"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                macaca_proto::MacacaError::Task("missing required field: agent".into())
            })?;
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

        let resolved_agent = agent.to_string();
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
        if self.active_goal_id.is_some() {
            let title_norm = Self::normalize_title(title);
            if let Some(existing) = existing_scope.iter().find(|item| {
                item.assigned_agent == resolved_agent
                    && Self::normalize_title(&item.title) == title_norm
                    && !Self::is_terminal_status(item.status)
            }) {
                tracing::info!(
                    task_id = %existing.id,
                    agent = %resolved_agent,
                    goal_id = ?self.active_goal_id,
                    title = %existing.title,
                    "Deduplicated create_todo call within active goal"
                );
                return Ok(json!({
                    "task_id": existing.id.to_string(),
                    "agent": resolved_agent,
                    "requested_agent": agent,
                    "status": existing.status,
                    "priority": existing.priority,
                    "dependency_count": existing.depends_on.len(),
                    "auto_inferred_dependencies": 0,
                    "deduplicated": true,
                    "routing_reason": Value::Null,
                }));
            }
        }

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
            .create_task_assignment(
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
            "deduplicated": false,
            "routing_reason": Value::Null,
        }))
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
        self.create_one(input).await
    }
}

/// Create multiple todos in one tool call.
///
/// This is optimized for planner decomposition: the planner can persist the
/// whole task graph in a single ReAct action instead of spending one LLM round
/// trip per task.
pub struct CreateTodosTool {
    pub create_todo: CreateTodoTool,
}

#[async_trait]
impl Tool for CreateTodosTool {
    fn name(&self) -> &str {
        "create_todos"
    }

    fn description(&self) -> &str {
        "Create multiple tasks and assign each to a specific agent's task board in one call. Preferred for goal decomposition."
    }

    fn parameters_schema(&self) -> Value {
        let item_schema = crate::tool::ToolSchemaProvider::tool_schema(&self.create_todo);
        json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "minItems": 1,
                    "description": "Tasks to create, in dependency order. Each item uses the same schema as create_todo.",
                    "items": item_schema
                }
            },
            "required": ["tasks"]
        })
    }

    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let tasks = input["tasks"].as_array().ok_or_else(|| {
            macaca_proto::MacacaError::Task("missing required field: tasks".into())
        })?;
        if tasks.is_empty() {
            return Err(macaca_proto::MacacaError::Task(
                "tasks must contain at least one task".into(),
            ));
        }

        let mut created = Vec::with_capacity(tasks.len());
        for task in tasks {
            created.push(self.create_todo.create_one(task.clone()).await?);
        }

        Ok(json!({
            "count": created.len(),
            "created": created,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_persist::RedbStore;
    use macaca_task::TodoStore;
    use tempfile::tempdir;

    /// Provider-neutral fixture agent ids (Object Mother pattern).
    const FIXTURE_ENTRY_SUPERVISOR: &str = "entry-agent";
    const FIXTURE_PLAN_AGENT: &str = "plan-agent";
    const FIXTURE_DESIGN_AGENT: &str = "fixture-design";
    const FIXTURE_API_AGENT: &str = "fixture-api";
    const FIXTURE_UI_AGENT: &str = "fixture-ui";

    async fn exec_tool(tool: &dyn Tool, input: Value) -> MacacaResult<Value> {
        crate::tool::ToolCommandExecutor::execute_command(
            tool,
            crate::tool::ToolCommand::new(input),
        )
        .await
    }

    #[tokio::test]
    async fn create_todo_rejects_supervisor_agents() {
        let dir = tempdir().expect("tempdir");
        let db = RedbStore::open(dir.path().join("todo-tests.redb")).expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let space = Arc::new(TaskSpace::for_session(
            macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
            Some("session".into()),
            store,
        ));
        let tool = CreateTodoTool {
            space,
            coordinator_name: FIXTURE_PLAN_AGENT.into(),
            disallowed_assignees: vec![
                FIXTURE_ENTRY_SUPERVISOR.into(),
                FIXTURE_PLAN_AGENT.into(),
            ],
            assignee_capabilities: HashMap::new(),
            active_goal_id: None,
        };

        let err = exec_tool(
            &tool,
            json!({
                "agent": FIXTURE_ENTRY_SUPERVISOR,
                "title": "Should fail",
                "description": "Supervisor must not get TaskBoard work"
            }),
        )
        .await
        .expect_err("supervisor assignment should be rejected");

        assert!(
            err.to_string()
                .contains("cannot assign tasks to supervisor agent"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn create_todo_requires_agent_field() {
        let dir = tempdir().expect("tempdir");
        let db =
            RedbStore::open(dir.path().join("todo-tests-missing-agent.redb")).expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let space = Arc::new(TaskSpace::for_session(
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

        let err = exec_tool(
            &tool,
            json!({
                "title": "Missing agent",
                "description": "should fail when agent is not provided"
            }),
        )
        .await
        .expect_err("missing agent must be rejected");

        assert!(
            err.to_string().contains("missing required field: agent"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn create_todo_preserves_requested_agent_even_when_profile_differs() {
        let dir = tempdir().expect("tempdir");
        let db =
            RedbStore::open(dir.path().join("todo-tests-preserve-agent.redb")).expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let space = Arc::new(TaskSpace::for_session(
            macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
            Some("session".into()),
            store,
        ));
        let mut profiles = HashMap::new();
        profiles.insert(
            FIXTURE_DESIGN_AGENT.to_string(),
            vec![
                "design_analysis".into(),
                "architecture specification interface planning".into(),
            ],
        );
        profiles.insert(
            FIXTURE_API_AGENT.to_string(),
            vec![
                "api_development".into(),
                "rest api server database integration".into(),
            ],
        );
        let tool = CreateTodoTool {
            space: Arc::clone(&space),
            coordinator_name: FIXTURE_PLAN_AGENT.into(),
            disallowed_assignees: vec![],
            assignee_capabilities: profiles,
            active_goal_id: None,
        };

        let out = exec_tool(
            &tool,
            json!({
                "agent": FIXTURE_DESIGN_AGENT,
                "title": "Implement API service layer",
                "description": "Build REST endpoints and database integration"
            }),
        )
        .await
        .expect("create_todo should succeed");

        assert_eq!(
            out["agent"].as_str().unwrap_or_default(),
            FIXTURE_DESIGN_AGENT,
            "create_todo must preserve plan agent's explicit assignee"
        );
        assert_eq!(
            out["requested_agent"].as_str().unwrap_or_default(),
            FIXTURE_DESIGN_AGENT
        );
        assert!(out["routing_reason"].is_null());
        let all = space.list_all().await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].assigned_agent, FIXTURE_DESIGN_AGENT);
    }

    #[tokio::test]
    async fn create_todo_preserves_requested_agent_for_foundation_tasks() {
        let dir = tempdir().expect("tempdir");
        let db = RedbStore::open(dir.path().join("todo-tests-foundation-preserve.redb"))
            .expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let space = Arc::new(TaskSpace::for_session(
            macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
            Some("session".into()),
            store,
        ));
        let mut profiles = HashMap::new();
        profiles.insert(
            FIXTURE_DESIGN_AGENT.to_string(),
            vec![
                "architecture_design Define system boundaries and technical constraints".into(),
                "interface_contract_design Define API and data contracts".into(),
            ],
        );
        profiles.insert(
            FIXTURE_API_AGENT.to_string(),
            vec!["api_development Implement REST API with PostgreSQL".into()],
        );
        let tool = CreateTodoTool {
            space: Arc::clone(&space),
            coordinator_name: FIXTURE_PLAN_AGENT.into(),
            disallowed_assignees: vec![],
            assignee_capabilities: profiles,
            active_goal_id: None,
        };

        let out = exec_tool(&tool, json!({
                "agent": FIXTURE_API_AGENT,
                "title": "设计项目架构和API规范",
                "description": "设计整体项目架构、接口规范、数据模型和契约。输出架构设计文档。"
            }))
            .await
            .expect("create_todo should succeed");

        assert_eq!(
            out["agent"].as_str().unwrap_or_default(),
            FIXTURE_API_AGENT,
            "create_todo must not override plan agent's explicit assignee"
        );
        assert_eq!(
            out["requested_agent"].as_str().unwrap_or_default(),
            FIXTURE_API_AGENT
        );
        assert!(out["routing_reason"].is_null());
        let all = space.list_all().await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].assigned_agent, FIXTURE_API_AGENT);
    }

    #[tokio::test]
    async fn create_todo_auto_adds_foundation_dependencies_for_goal() {
        let dir = tempdir().expect("tempdir");
        let db =
            RedbStore::open(dir.path().join("todo-tests-foundation-deps.redb")).expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let app_id = macaca_proto::ApplicationId(uuid::Uuid::new_v4());
        let goal_id = macaca_proto::TaskId::new();
        let space = Arc::new(TaskSpace::for_session(
            app_id,
            Some("session".into()),
            Arc::clone(&store),
        ));
        let mut profiles = HashMap::new();
        profiles.insert(
            FIXTURE_DESIGN_AGENT.to_string(),
            vec!["design_analysis architecture specification".into()],
        );
        profiles.insert(
            FIXTURE_UI_AGENT.to_string(),
            vec!["ui_development react nextjs presentation".into()],
        );
        let tool = CreateTodoTool {
            space: Arc::clone(&space),
            coordinator_name: FIXTURE_PLAN_AGENT.into(),
            disallowed_assignees: vec![],
            assignee_capabilities: profiles,
            active_goal_id: Some(goal_id),
        };

        let design_task = exec_tool(
            &tool,
            json!({
                "agent": FIXTURE_DESIGN_AGENT,
                "title": "Define architecture and interfaces",
                "description": "Produce design spec and API contracts"
            }),
        )
        .await
        .expect("design task create");
        let design_id = macaca_proto::TaskId(
            uuid::Uuid::parse_str(design_task["task_id"].as_str().unwrap_or_default())
                .expect("design task id"),
        );

        let ui_task = exec_tool(
            &tool,
            json!({
                "agent": FIXTURE_UI_AGENT,
                "title": "Implement UI from spec",
                "description": "Build pages and connect to API layer"
            }),
        )
        .await
        .expect("ui task create");

        assert!(
            ui_task["auto_inferred_dependencies"]
                .as_u64()
                .unwrap_or_default()
                >= 1,
            "ui task should get inferred dependency on design task"
        );

        let all = space.list_all().await;
        let ui_item = all
            .into_iter()
            .find(|t| t.id.to_string() == ui_task["task_id"].as_str().unwrap_or_default())
            .expect("ui task exists");
        assert_eq!(ui_item.status, TodoStatus::Blocked);
        assert!(
            ui_item.depends_on.contains(&design_id),
            "ui task should depend on design task"
        );
    }

    #[tokio::test]
    async fn create_todo_deduplicates_same_goal_agent_and_title() {
        let dir = tempdir().expect("tempdir");
        let db = RedbStore::open(dir.path().join("todo-tests-dedupe.redb")).expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let goal_id = macaca_proto::TaskId::new();
        let space = Arc::new(TaskSpace::for_session(
            macaca_proto::ApplicationId(uuid::Uuid::new_v4()),
            Some("session".into()),
            Arc::clone(&store),
        ));
        let tool = CreateTodoTool {
            space: Arc::clone(&space),
            coordinator_name: FIXTURE_PLAN_AGENT.into(),
            disallowed_assignees: vec![],
            assignee_capabilities: HashMap::new(),
            active_goal_id: Some(goal_id),
        };

        let first = exec_tool(
            &tool,
            json!({
                "agent": "news_fact_checker",
                "title": "DeepSeek V4 fact check",
                "description": "Verify claims"
            }),
        )
        .await
        .expect("first create_todo should succeed");
        let second = exec_tool(
            &tool,
            json!({
                "agent": "news_fact_checker",
                "title": "  deepseek v4 fact check  ",
                "description": "Verify claims again"
            }),
        )
        .await
        .expect("duplicate create_todo should return existing task");

        assert_eq!(first["task_id"], second["task_id"]);
        assert_eq!(second["deduplicated"].as_bool(), Some(true));
        let all = space.list_all().await;
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn create_todos_creates_multiple_tasks_in_one_call() {
        let dir = tempdir().expect("tempdir");
        let db = RedbStore::open(dir.path().join("todo-tests-batch.redb")).expect("open redb");
        let store = Arc::new(TodoStore::new(Arc::new(db)));
        let app_id = macaca_proto::ApplicationId(uuid::Uuid::new_v4());
        let goal_id = macaca_proto::TaskId::new();
        let space = Arc::new(TaskSpace::for_session(
            app_id,
            Some("session".into()),
            Arc::clone(&store),
        ));
        let tool = CreateTodosTool {
            create_todo: CreateTodoTool {
                space: Arc::clone(&space),
                coordinator_name: FIXTURE_PLAN_AGENT.into(),
                disallowed_assignees: vec![],
                assignee_capabilities: HashMap::new(),
                active_goal_id: Some(goal_id),
            },
        };

        let out = exec_tool(
            &tool,
            json!({
                "tasks": [
                    {
                        "agent": "news_researcher",
                        "title": "Collect sources",
                        "description": "Gather source material",
                        "priority": 9
                    },
                    {
                        "agent": "news_writer",
                        "title": "Draft article",
                        "description": "Write the first draft",
                        "priority": 8,
                        "depends_on_titles": ["Collect sources"]
                    }
                ]
            }),
        )
        .await
        .expect("create_todos should succeed");

        assert_eq!(out["count"].as_u64(), Some(2));
        let all = space.list_all().await;
        assert_eq!(all.len(), 2);
        let writer = all
            .iter()
            .find(|task| task.assigned_agent == "news_writer")
            .expect("writer task");
        assert_eq!(writer.depends_on.len(), 1);
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
        let ok = self
            .space
            .apply_review_result(&task_id, agent, result)
            .await;
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
