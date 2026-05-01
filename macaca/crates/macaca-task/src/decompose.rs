use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use macaca_proto::types::{LlmMessage, LlmOptions};
use macaca_proto::{
    ApplicationPlanningAgentProfile as AppPlanningAgentProfile,
    ApplicationTaskPlanningContract as AppTaskPlanningContract, MacacaResult, Task, TaskId,
    TaskRequest,
};

/// Trait for breaking a high-level task into sub-task requests.
/// Implementations may call an LLM or apply heuristic rules.
#[async_trait]
pub trait TaskDecomposer: Send + Sync {
    /// Decompose `task` into zero or more child `TaskRequest`s.
    /// Returning an empty vec means the task should be executed as-is.
    async fn decompose(&self, task: &Task) -> MacacaResult<Vec<TaskRequest>>;
}

/// A no-op decomposer that treats every task as atomic.
pub struct SimpleDecomposer;

#[async_trait]
impl TaskDecomposer for SimpleDecomposer {
    async fn decompose(&self, _task: &Task) -> MacacaResult<Vec<TaskRequest>> {
        Ok(Vec::new())
    }
}

// ── LLM-based decomposer ──────────────────────────────────────────────────────

/// A decomposed sub-task from LLM output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedTask {
    pub title: String,
    pub description: String,
    pub assigned_agent: String,
    pub priority: u8,
    /// Execution order within this agent's task list (1-based).
    #[serde(default)]
    pub sequence: u32,
    pub acceptance_criteria: Vec<String>,
    /// Names (titles) of tasks this depends on.
    #[serde(default)]
    pub depends_on_titles: Vec<String>,
    /// Cross-agent dependency declarations (resolved to TaskIds at creation time).
    #[serde(default)]
    pub depends_on_agents: Vec<macaca_proto::types::AgentTaskRef>,
}

/// LLM-driven task decomposer.
pub struct LlmDecomposer {
    llm: Arc<dyn macaca_llm::LlmProvider>,
    model: String,
}

impl LlmDecomposer {
    pub fn new(llm: Arc<dyn macaca_llm::LlmProvider>, model: impl Into<String>) -> Self {
        Self {
            llm,
            model: model.into(),
        }
    }

    /// Decompose a goal description into structured sub-tasks.
    pub async fn decompose(
        &self,
        goal: &str,
        available_agents: &[String],
    ) -> Result<Vec<DecomposedTask>, String> {
        let contract = legacy_planning_contract(available_agents);
        self.decompose_for_contract(goal, &contract).await
    }

    /// Decompose using an application-level planning contract.
    pub async fn decompose_for_contract(
        &self,
        goal: &str,
        contract: &AppTaskPlanningContract,
    ) -> Result<Vec<DecomposedTask>, String> {
        let prompt = build_decomposition_prompt(goal, contract);
        let available_agents = contract.available_agent_names();

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
            .map_err(|e| format!("LLM decomposition failed: {}", e))?;

        let tasks = Self::parse_llm_output(&response.content)?;

        if tasks.is_empty() {
            return Err("LLM returned empty task list".into());
        }

        // Validate agents — log a warning for unknowns but don't error
        for task in &tasks {
            if !available_agents.contains(&task.assigned_agent) {
                tracing::warn!(
                    agent = %task.assigned_agent,
                    "LLM assigned task to unknown agent, will use first available"
                );
            }
        }

        Ok(tasks)
    }

    /// Parse LLM output that may be raw JSON or wrapped in a markdown code block.
    fn parse_llm_output(content: &str) -> Result<Vec<DecomposedTask>, String> {
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

        serde_json::from_str(json_str).map_err(|e| {
            format!(
                "Failed to parse LLM output as JSON: {}. Output: {}",
                e,
                &json_str[..json_str.len().min(500)]
            )
        })
    }

    /// Decompose and create tasks in a TaskSpace, resolving dependency references.
    pub async fn decompose_into_space(
        &self,
        goal: &str,
        goal_id: &TaskId,
        available_agents: &[String],
        space: &crate::todo_board::TaskSpace,
    ) -> Result<Vec<macaca_proto::types::TodoItem>, String> {
        let contract = legacy_planning_contract(available_agents);
        self.decompose_into_space_for_contract(goal, goal_id, &contract, space)
            .await
    }

    /// Decompose and create tasks in a TaskSpace using an application-level
    /// planning contract, resolving dependency references at creation time.
    pub async fn decompose_into_space_for_contract(
        &self,
        goal: &str,
        goal_id: &TaskId,
        contract: &AppTaskPlanningContract,
        space: &crate::todo_board::TaskSpace,
    ) -> Result<Vec<macaca_proto::types::TodoItem>, String> {
        let available_agents = contract.available_agent_names();
        let mut decomposed = self.decompose_for_contract(goal, contract).await?;

        // Sort by sequence within each agent to ensure correct ordering
        decomposed.sort_by_key(|t| t.sequence);

        let mut created_items = Vec::new();
        // Map title -> TaskId for dependency resolution within this batch
        let mut title_to_id: HashMap<String, TaskId> = HashMap::new();
        // Map agent_name -> Vec<TaskId> for cross-agent dependency resolution
        let mut agent_to_ids: HashMap<String, Vec<TaskId>> = HashMap::new();

        for task in &decomposed {
            // Resolve depends_on from titles to TaskIds
            let mut depends_on: Vec<TaskId> = task
                .depends_on_titles
                .iter()
                .filter_map(|title| title_to_id.get(title).copied())
                .collect();

            // Resolve cross-agent dependencies
            for agent_ref in &task.depends_on_agents {
                match agent_ref {
                    macaca_proto::types::AgentTaskRef::AllTasks { agent } => {
                        if let Some(ids) = agent_to_ids.get(agent.as_str()) {
                            for id in ids {
                                if !depends_on.contains(id) {
                                    depends_on.push(*id);
                                }
                            }
                        }
                    }
                    macaca_proto::types::AgentTaskRef::SpecificTask { agent: _, title } => {
                        if let Some(id) = title_to_id.get(title) {
                            if !depends_on.contains(id) {
                                depends_on.push(*id);
                            }
                        }
                    }
                }
            }

            let agent = if available_agents.contains(&task.assigned_agent) {
                task.assigned_agent.as_str()
            } else {
                available_agents
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("default")
            };

            let item = space
                .create_task_assignment(
                    agent,
                    "plan_agent",
                    &task.title,
                    &task.description,
                    task.acceptance_criteria.clone(),
                    task.priority,
                    depends_on,
                    Some(*goal_id),
                )
                .await;

            title_to_id.insert(task.title.clone(), item.id);
            agent_to_ids
                .entry(agent.to_string())
                .or_default()
                .push(item.id);
            created_items.push(item);
        }

        tracing::info!(
            goal = goal,
            task_count = created_items.len(),
            "Goal decomposed into tasks"
        );

        Ok(created_items)
    }
}

fn legacy_planning_contract(available_agents: &[String]) -> AppTaskPlanningContract {
    AppTaskPlanningContract {
        workflow_name: "default".into(),
        entry_agent: "entry_agent".into(),
        worker_agents: available_agents
            .iter()
            .cloned()
            .map(AppPlanningAgentProfile::legacy)
            .collect(),
    }
}

/// Build the canonical decomposition prompt from an application planning
/// contract. Runtime callers should supply a contract derived from
/// `macaca-app` instead of reconstructing planner prompt rules ad hoc.
pub fn build_decomposition_prompt(goal: &str, contract: &AppTaskPlanningContract) -> String {
    let agents_list = if contract.worker_agents.is_empty() {
        "no worker agents registered".to_string()
    } else {
        contract.available_agent_names().join(", ")
    };
    let agents_profile_text = contract.render_agent_profiles();

    format!(
        "A new project goal has been submitted under workflow `{}`. You MUST decompose it into \
         concrete tasks. Prefer one `create_todos` tool call with the full task graph; \
         use `create_todo` only as a fallback for a single additional task.\n\n\
         Goal: {}\n\n\
         Available agents: {}.\n\
         Agent dossiers:\n{}\n\n\
         IMPORTANT ASSIGNMENT CONTRACT:\n\
         - The `agent` you pass to `create_todo` is authoritative; the system will NOT automatically reroute it.\n\
         - You must choose the correct worker agent from the dossiers above.\n\
         - If the acceptance criteria require working code, files, APIs, UI, tests, or runnable behavior, assign to an implementation-capable agent rather than a design/spec-only agent.\n\
         - Use design/spec agents for architecture, interfaces, data contracts, risk analysis, and handoff documents only.\n\
         - If a task has both design and implementation work, split it into separate dependent tasks.\n\n\
         INSTRUCTIONS:\n\
         1. Briefly analyze the goal (1-2 sentences).\n\
         2. Call `create_todos` once with ALL tasks whenever possible — you MUST create at least one task.\n\
         3. Set priorities (0-10) and acceptance_criteria for each task.\n\
         4. Assign each task to the MOST CAPABLE agent based on the dossiers above; use the exact agent name.\n\
         5. Create foundational design/spec tasks first, then implementation tasks.\n\
         6. If a foundational design/spec task is needed, assign it to an agent whose capability profile includes architecture/design/spec/interface/analysis when such a profile exists.\n\
         7. Do not assign design/spec work to implementation agents solely because the task mentions API, backend, or frontend; choose the strongest capability profile.\n\
         8. Use dependencies explicitly:\n\
            - depends_on: known task IDs\n\
            - depends_on_titles: task titles already created in this decomposition\n\
            - depends_on_agents: symbolic cross-agent dependencies (all_tasks/specific_task)\n\
         9. Do NOT create duplicate tasks with the same title and agent.\n\
         10. Do NOT just describe tasks in text — you MUST invoke `create_todos` or `create_todo`.\n\
         11. After creating the complete task chain, stop calling tools and reply exactly with: DECOMPOSITION_COMPLETE.\n\n\
         Start by creating the complete task graph now.",
        contract.workflow_name,
        goal,
        agents_list,
        agents_profile_text,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{Task, TaskId, TaskPriority, TaskStatus};

    fn sample_task() -> Task {
        let now = Utc::now();
        Task {
            id: TaskId::new(),
            description: "sample".into(),
            status: TaskStatus::Pending,
            priority: TaskPriority::Normal,
            assigned_agent: None,
            subtasks: Vec::new(),
            parent: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn simple_decomposer_returns_empty() {
        let d = SimpleDecomposer;
        let task = sample_task();
        let subtasks = d.decompose(&task).await.unwrap();
        assert!(subtasks.is_empty());
    }

    #[tokio::test]
    async fn decomposer_is_object_safe() {
        let d: Box<dyn TaskDecomposer> = Box::new(SimpleDecomposer);
        let task = sample_task();
        let subtasks = d.decompose(&task).await.unwrap();
        assert!(subtasks.is_empty());
    }

    #[test]
    fn decomposed_task_roundtrip() {
        let task = DecomposedTask {
            title: "Design API".into(),
            description: "Design REST API endpoints".into(),
            assigned_agent: "architect".into(),
            priority: 9,
            sequence: 1,
            acceptance_criteria: vec!["API spec documented".into()],
            depends_on_titles: vec![],
            depends_on_agents: vec![],
        };
        let json = serde_json::to_string(&task).unwrap();
        let parsed: DecomposedTask = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.title, "Design API");
        assert_eq!(parsed.priority, 9);
    }

    #[test]
    fn parse_llm_output_json_array() {
        let output = r#"[{"title":"Task 1","description":"Do thing","assigned_agent":"backend","priority":8,"acceptance_criteria":["done"],"depends_on_titles":[]}]"#;
        let tasks = LlmDecomposer::parse_llm_output(output).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].assigned_agent, "backend");
    }

    #[test]
    fn parse_llm_output_with_json_code_block() {
        let output = "```json\n[{\"title\":\"T1\",\"description\":\"D\",\"assigned_agent\":\"backend\",\"priority\":5,\"acceptance_criteria\":[],\"depends_on_titles\":[]}]\n```";
        let tasks = LlmDecomposer::parse_llm_output(output).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "T1");
    }

    #[test]
    fn parse_llm_output_with_plain_code_block() {
        let output = "```\n[{\"title\":\"T2\",\"description\":\"D2\",\"assigned_agent\":\"frontend\",\"priority\":3,\"acceptance_criteria\":[\"done\"],\"depends_on_titles\":[]}]\n```";
        let tasks = LlmDecomposer::parse_llm_output(output).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].assigned_agent, "frontend");
    }

    #[test]
    fn parse_llm_output_invalid_json_returns_error() {
        let result = LlmDecomposer::parse_llm_output("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn decomposition_prompt_uses_contract_rendering() {
        let prompt = build_decomposition_prompt(
            "Build a weather app",
            &AppTaskPlanningContract {
                workflow_name: "default".into(),
                entry_agent: "coordinator".into(),
                worker_agents: vec![AppPlanningAgentProfile {
                    name: "backend".into(),
                    capabilities: vec!["api: implement backend endpoints".into()],
                    available: true,
                    current_load: 0,
                    max_load: 2,
                    permission_level: "system".into(),
                    model: "mock".into(),
                    allowed_tools: vec!["claude_code_execute".into()],
                }],
            },
        );
        assert!(prompt.contains("workflow `default`"));
        assert!(prompt.contains("Agent `backend`"));
        assert!(prompt.contains("api: implement backend endpoints"));
    }
}
