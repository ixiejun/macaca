//! Capability-driven tool policy resolution for per-agent toolkit assembly.
//!
//! Policy derives from kernel agent manifests and application persona allowlists.
//! No application-specific agent names are hardcoded; capabilities such as
//! `todo_goal_management` drive which todo tools an agent receives.

use std::collections::HashSet;
use std::sync::Arc;

use macaca_host_composition::app::{app_agent_manifest_view, discovered_app_agent_names};
use macaca_host_composition::framework::tool::Toolkit;
use macaca_proto::ApplicationId;

use crate::state::AppState;

/// Todo tool tier assigned from agent capabilities (Strategy pattern).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TodoToolPolicy {
    GoalManager,
    Planner,
    Worker,
}

/// Resolved policy snapshot consumed by the toolkit builder and agent_tools registrar.
#[derive(Debug, Clone)]
pub(crate) struct AgentToolPolicy {
    pub(crate) base_allowed_tools: Option<HashSet<String>>,
    pub(crate) todo_policy: TodoToolPolicy,
    pub(crate) disallowed_task_assignees: HashSet<String>,
    pub(crate) can_create_scheduled_agent_tasks: bool,
}

impl AgentToolPolicy {
    /// Returns whether a base tool name is permitted by manifest allowlist policy.
    pub(crate) fn allows_base_tool(&self, tool_name: &str) -> bool {
        self.base_allowed_tools
            .as_ref()
            .map(|allowlist| allowlist.contains(tool_name))
            .unwrap_or(true)
    }
}

/// Remove toolkit entries not present in the manifest allowlist (final enforcement pass).
pub(crate) fn enforce_base_tool_allowlist(
    toolkit: &mut Toolkit,
    allowlist: Option<&HashSet<String>>,
) {
    let Some(allowlist) = allowlist else {
        return;
    };
    let names = toolkit
        .get_definitions()
        .into_iter()
        .filter_map(|tool| {
            tool.get("name")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    for name in names {
        if !allowlist.contains(&name) {
            toolkit.unregister(&name);
        }
    }
}
pub(super) async fn app_agent_names(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
) -> Option<HashSet<String>> {
    let app = {
        let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
        registry.get_app(app_id).cloned()
    }?;

    match discovered_app_agent_names(&app) {
        Ok(agent_names) => Some(agent_names.into_iter().collect()),
        Err(error) => {
            tracing::warn!(
                app_id = %app_id,
                error = %error,
                "Failed to resolve app-scoped agent names; falling back to global agents"
            );
            None
        }
    }
}

pub(super) async fn resolve_tool_policy(
    state: &Arc<AppState>,
    app_id: &ApplicationId,
    agent_name: &str,
) -> AgentToolPolicy {
    let manifest = state.kernel.get_agent_by_name(agent_name).await;
    let capabilities: HashSet<String> = manifest
        .as_ref()
        .map(|m| m.capabilities.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default();
    let (manifest_allowed_tools, is_entry_agent) = {
        let registry = crate::application_shell_adapter::registry_read_guard(&state).await;
        registry
            .get_app(app_id)
            .and_then(|app| app_agent_manifest_view(&app.manifest, agent_name))
            .map(|agent| (Some(agent.allowed_tools().to_vec()), agent.is_entry_agent()))
            .unwrap_or((None, false))
    };
    let base_allowed_tools = manifest_allowed_tools
        .and_then(|allowed_tools| (!allowed_tools.is_empty()).then_some(allowed_tools))
        .map(|allowed_tools| allowed_tools.into_iter().collect())
        .or_else(|| {
            manifest.as_ref().and_then(|m| {
                (!m.permission.allowed_tools.is_empty())
                    .then_some(m.permission.allowed_tools.iter().cloned().collect())
            })
        });
    // Capability-driven policy (preferred):
    // - todo_goal_management: can create/check goals
    // - task_planning / todo_planning: can create/review/reassign todos
    // - todo_execution: worker task-board operations
    //
    // Capability-neutral defaults:
    // - entry agent defaults to GoalManager
    // - non-planner/non-entry agents default to Worker
    let todo_policy = if capabilities.contains("todo_goal_management") {
        TodoToolPolicy::GoalManager
    } else if capabilities.contains("task_planning") || capabilities.contains("todo_planning") {
        TodoToolPolicy::Planner
    } else if capabilities.contains("todo_execution") {
        TodoToolPolicy::Worker
    } else if is_entry_agent {
        TodoToolPolicy::GoalManager
    } else {
        TodoToolPolicy::Worker
    };

    // Any supervisor-like agent should not receive executable TaskBoard todos.
    // Keep this capability-driven first, with entry-agent defaulting only when
    // the manifest does not expose a more precise capability.
    let app_agent_names = app_agent_names(state, app_id).await;
    let mut disallowed_task_assignees: HashSet<String> = state
        .kernel
        .list_agents()
        .await
        .into_iter()
        .filter(|m| {
            app_agent_names
                .as_ref()
                .is_none_or(|names| names.contains(&m.name))
        })
        .filter_map(|m| {
            let caps: HashSet<String> = m.capabilities.into_iter().map(|c| c.name).collect();
            let is_supervisor = caps.contains("todo_goal_management")
                || caps.contains("task_planning")
                || caps.contains("todo_planning");
            if is_supervisor {
                Some(m.name)
            } else {
                None
            }
        })
        .collect();
    if is_entry_agent {
        disallowed_task_assignees.insert(agent_name.to_string());
    }

    AgentToolPolicy {
        base_allowed_tools,
        todo_policy,
        disallowed_task_assignees,
        can_create_scheduled_agent_tasks: is_entry_agent
            || capabilities.contains("scheduled_agent_task_management"),
    }
}
