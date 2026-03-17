//! Scheduler — selects which agent should handle a given task.

use async_trait::async_trait;
use macaca_proto::{AgentId, AgentState, MacacaResult, Task};
use tracing::{info, warn};

use crate::registry::AgentRegistry;

// ── Scheduler trait ───────────────────────────────────────────────────────────

/// Decides which registered agent should execute a task.
#[async_trait]
pub trait Scheduler: Send + Sync {
    /// Return the id of an agent that can handle `task`, or `None` if no
    /// suitable agent is currently available.
    async fn select_agent(
        &self,
        registry: &AgentRegistry,
        task: &Task,
    ) -> MacacaResult<Option<AgentId>>;
}

// ── SimpleScheduler ───────────────────────────────────────────────────────────

/// Picks the first `Running` agent whose declared capabilities include at least
/// one capability whose name appears in the task description (case-insensitive).
///
/// If no capability match is found it falls back to the first `Running` agent.
pub struct SimpleScheduler;

#[async_trait]
impl Scheduler for SimpleScheduler {
    async fn select_agent(
        &self,
        registry: &AgentRegistry,
        task: &Task,
    ) -> MacacaResult<Option<AgentId>> {
        let manifests = registry.list().await;
        let description_lower = task.description.to_lowercase();

        // First pass: capability match among Running agents.
        for manifest in &manifests {
            if manifest.state != AgentState::Running {
                continue;
            }
            let matches = manifest
                .capabilities
                .iter()
                .any(|c| description_lower.contains(&c.name.to_lowercase()));
            if matches {
                info!(
                    task_id = %task.id.0,
                    agent_id = %manifest.id.0,
                    agent_name = %manifest.name,
                    matched_capability = ?manifest.capabilities.iter().find(|c| description_lower.contains(&c.name.to_lowercase())).map(|c| &c.name),
                    selection_type = "capability_match",
                    "[SCHEDULE] Agent selected by capability match"
                );
                return Ok(Some(manifest.id));
            }
        }

        // Fallback: first Running agent regardless of capability.
        for manifest in &manifests {
            if manifest.state == AgentState::Running {
                info!(
                    task_id = %task.id.0,
                    agent_id = %manifest.id.0,
                    agent_name = %manifest.name,
                    selection_type = "fallback",
                    "[SCHEDULE] Agent selected (fallback)"
                );
                return Ok(Some(manifest.id));
            }
        }

        warn!(
            task_id = %task.id.0,
            running_agents = manifests.iter().filter(|m| m.state == AgentState::Running).count(),
            total_agents = manifests.len(),
            "[SCHEDULE] No available agent for task"
        );
        Ok(None)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait as at;
    use chrono::Utc;
    use macaca_agent::AgentServices;
    use macaca_llm::LlmProvider;
    use macaca_tools::ToolSet;
    use macaca_proto::{
        AgentManifest, AgentOutput, Capability, Permission, PermissionLevel,
        TaskId, TaskPriority, TaskStatus, TokenUsage,
    };

    struct MockAgent {
        id: AgentId,
        state: AgentState,
        caps: Vec<Capability>,
    }

    #[at]
    impl macaca_agent::Agent for MockAgent {
        fn id(&self) -> AgentId { self.id }
        fn capabilities(&self) -> &[Capability] { &self.caps }
        fn state(&self) -> AgentState { self.state }
        async fn run(
            &self,
            _llm: &dyn LlmProvider,
            _tools: &dyn ToolSet,
            _services: &AgentServices,
        ) -> MacacaResult<AgentOutput> {
            Ok(AgentOutput {
                result: "ok".into(),
                artifacts: vec![],
                tokens_used: TokenUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
            })
        }
    }

    fn make_manifest(id: AgentId, state: AgentState, caps: Vec<Capability>) -> AgentManifest {
        AgentManifest {
            id,
            name: "test".into(),
            capabilities: caps,
            permission: Permission {
                level: PermissionLevel::User,
                allowed_tools: vec![],
                allowed_paths: vec![],
                network_access: false,
            },
            state,
            created_at: Utc::now(),
            model: String::new(),
        }
    }

    fn make_task(description: &str) -> Task {
        Task {
            id: TaskId::new(),
            description: description.into(),
            status: TaskStatus::Pending,
            priority: TaskPriority::Normal,
            assigned_agent: None,
            subtasks: vec![],
            parent: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn selects_running_agent_by_capability() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let caps = vec![Capability { name: "search".into(), description: "web search".into() }];
        let agent = Box::new(MockAgent { id, state: AgentState::Running, caps: caps.clone() });
        let manifest = make_manifest(id, AgentState::Running, caps);
        reg.register(agent, manifest).await.unwrap();

        let task = make_task("search the web for Rust");
        let selected = SimpleScheduler.select_agent(&reg, &task).await.unwrap();
        assert_eq!(selected, Some(id));
    }

    #[tokio::test]
    async fn skips_non_running_agent() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let caps = vec![Capability { name: "search".into(), description: "".into() }];
        let agent = Box::new(MockAgent { id, state: AgentState::Suspended, caps: caps.clone() });
        let manifest = make_manifest(id, AgentState::Suspended, caps);
        reg.register(agent, manifest).await.unwrap();

        let task = make_task("search something");
        let selected = SimpleScheduler.select_agent(&reg, &task).await.unwrap();
        assert_eq!(selected, None);
    }

    #[tokio::test]
    async fn falls_back_to_first_running_when_no_cap_match() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let caps = vec![Capability { name: "write".into(), description: "".into() }];
        let agent = Box::new(MockAgent { id, state: AgentState::Running, caps: caps.clone() });
        let manifest = make_manifest(id, AgentState::Running, caps);
        reg.register(agent, manifest).await.unwrap();

        // Task description has no matching capability name.
        let task = make_task("do something unrelated");
        let selected = SimpleScheduler.select_agent(&reg, &task).await.unwrap();
        assert_eq!(selected, Some(id));
    }

    #[tokio::test]
    async fn returns_none_when_registry_empty() {
        let reg = AgentRegistry::new(10);
        let task = make_task("any task");
        let selected = SimpleScheduler.select_agent(&reg, &task).await.unwrap();
        assert_eq!(selected, None);
    }
}
