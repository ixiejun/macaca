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
///
/// Production callers should obtain this strategy through [`SchedulerFactory::build`]
/// so scheduler selection stays centralized and traceable.
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
    use chrono::Utc;
    use macaca_proto::{
        AgentManifest, Capability, Permission, PermissionLevel, TaskId, TaskPriority, TaskStatus,
    };

    use crate::{SchedulerFactory, SchedulerKind};

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
        let caps = vec![Capability {
            name: "search".into(),
            description: "web search".into(),
        }];
        let manifest = make_manifest(id, AgentState::Running, caps);
        reg.register(manifest).await.unwrap();

        let task = make_task("search the web for Rust");
        let scheduler = SchedulerFactory::build(SchedulerKind::Simple);
        let selected = scheduler.select_agent(&reg, &task).await.unwrap();
        assert_eq!(selected, Some(id));
    }

    #[tokio::test]
    async fn skips_non_running_agent() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let caps = vec![Capability {
            name: "search".into(),
            description: "".into(),
        }];
        let manifest = make_manifest(id, AgentState::Suspended, caps);
        reg.register(manifest).await.unwrap();

        let task = make_task("search something");
        let scheduler = SchedulerFactory::build(SchedulerKind::Simple);
        let selected = scheduler.select_agent(&reg, &task).await.unwrap();
        assert_eq!(selected, None);
    }

    #[tokio::test]
    async fn falls_back_to_first_running_when_no_cap_match() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let caps = vec![Capability {
            name: "write".into(),
            description: "".into(),
        }];
        let manifest = make_manifest(id, AgentState::Running, caps);
        reg.register(manifest).await.unwrap();

        // Task description has no matching capability name.
        let task = make_task("do something unrelated");
        let scheduler = SchedulerFactory::build(SchedulerKind::Simple);
        let selected = scheduler.select_agent(&reg, &task).await.unwrap();
        assert_eq!(selected, Some(id));
    }

    #[tokio::test]
    async fn returns_none_when_registry_empty() {
        let reg = AgentRegistry::new(10);
        let task = make_task("any task");
        let scheduler = SchedulerFactory::build(SchedulerKind::Simple);
        let selected = scheduler.select_agent(&reg, &task).await.unwrap();
        assert_eq!(selected, None);
    }
}
