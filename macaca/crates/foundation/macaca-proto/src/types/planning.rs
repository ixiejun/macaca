//! Application planning contracts shared between kernel and application layers.
//!
//! These types describe *how* an application wants tasks planned without embedding
//! application-specific business rules in the microkernel.

use serde::{Deserialize, Serialize};

/// Structured worker profile used by planner decomposition contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationPlanningAgentProfile {
    pub name: String,
    pub capabilities: Vec<String>,
    pub available: bool,
    pub current_load: usize,
    pub max_load: usize,
    pub permission_level: String,
    pub model: String,
    pub allowed_tools: Vec<String>,
}

impl ApplicationPlanningAgentProfile {
    pub fn legacy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            capabilities: vec!["no capability metadata".into()],
            available: true,
            current_load: 0,
            max_load: 0,
            permission_level: "unknown".into(),
            model: "app default".into(),
            allowed_tools: vec![],
        }
    }
}

/// Stable application-level planning contract shared across app/task/web
/// boundaries for planner decomposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationTaskPlanningContract {
    pub workflow_name: String,
    pub entry_agent: String,
    pub worker_agents: Vec<ApplicationPlanningAgentProfile>,
}

impl ApplicationTaskPlanningContract {
    pub fn available_agent_names(&self) -> Vec<String> {
        self.worker_agents
            .iter()
            .map(|agent| agent.name.clone())
            .collect()
    }

    pub fn render_agent_profiles(&self) -> String {
        if self.worker_agents.is_empty() {
            return "(none)".to_string();
        }
        self.worker_agents
            .iter()
            .map(|agent| {
                let capabilities = if agent.capabilities.is_empty() {
                    "    - no capability metadata".to_string()
                } else {
                    agent
                        .capabilities
                        .iter()
                        .map(|capability| format!("    - {capability}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                let tools = if agent.allowed_tools.is_empty() {
                    "all registered tools (open policy)".to_string()
                } else {
                    agent.allowed_tools.join(", ")
                };
                format!(
                    "- Agent `{}`\n  available: {}\n  load: {}/{}\n  permission: {}\n  model: {}\n  tools: {}\n  capabilities:\n{}",
                    agent.name,
                    agent.available,
                    agent.current_load,
                    agent.max_load,
                    agent.permission_level,
                    agent.model,
                    tools,
                    capabilities
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
