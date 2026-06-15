//! Plan-agent `create_todo` tool with dependency resolution and deduplication.
//!
//! **Strategy pattern**: assignment validation, title/agent-ref dependency resolution,
//! and foundation-profile inference are pluggable policies on [`CreateTodoTool`].
//! **Command pattern**: [`Tool`] implementation wraps `create_one` for single-task creation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{MacacaResult, TaskId, TodoItem, TodoStatus};
use macaca_task::TaskSpace;
use serde_json::{json, Value};

use crate::tool::{Tool, ToolCommand};

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

    pub(super) async fn create_one(&self, input: Value) -> MacacaResult<Value> {
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
    fn tool_schema(&self) -> Value {
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
    async fn invoke(&self, command: ToolCommand) -> MacacaResult<Value> {
        self.create_one(command.input).await
    }
}
