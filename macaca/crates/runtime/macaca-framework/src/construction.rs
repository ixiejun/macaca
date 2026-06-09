//! Framework-level traced agent construction contracts.
//!
//! These contracts let higher layers express build intent and runtime wiring
//! through `macaca-agent` abstractions instead of re-defining parallel
//! capability, service, and lifecycle structures in each crate.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_agent::{AgentCapabilitySet, AgentLifecyclePolicy, AgentServices};
use macaca_proto::{AgentState, ApplicationId, TaskId};

use crate::agent::Agent;
use crate::tool::Toolkit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentBuildIntent {
    RuntimeAgent,
    CoordinatorChat,
    PlannerDecomposition { goal_id: Option<TaskId> },
    PlannerReview { task_id: TaskId },
    PlannerFollowUp { goal_id: Option<TaskId> },
    GoalEvaluation { goal_id: Option<TaskId> },
    WorkerTask { task_id: TaskId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentity {
    pub app_id: ApplicationId,
    pub agent_name: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTraceContext {
    pub session_id: Option<String>,
    pub task_id: Option<TaskId>,
    pub source_agent: String,
}

pub struct AgentLifecycleConfig {
    pub initial_state: AgentState,
    pub policy: Option<Arc<dyn AgentLifecyclePolicy>>,
}

impl Default for AgentLifecycleConfig {
    fn default() -> Self {
        Self {
            initial_state: AgentState::Created,
            policy: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentToolConfig {
    pub goal_id: Option<TaskId>,
    pub suppress_worker_lifecycle_tools: bool,
    pub allowed_tool_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationPromptParts {
    pub role: String,
    pub constraints: String,
    pub tools: String,
    pub handoff: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplicationToolPolicy {
    pub allowed_tool_names: Vec<String>,
    pub execution_tool_names: Vec<String>,
    pub is_entry_agent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationSemantics {
    pub workflow_name: Option<String>,
    pub entry_agent: Option<String>,
    pub prompt_parts: Option<ApplicationPromptParts>,
    pub tool_policy: ApplicationToolPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn default_profile(name: impl Into<String>) -> Self {
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub struct AgentBuildRequest {
    pub identity: AgentIdentity,
    pub intent: AgentBuildIntent,
    pub system_prompt: String,
    pub services: AgentServices,
    pub capabilities: AgentCapabilitySet,
    pub lifecycle: AgentLifecycleConfig,
    pub trace: AgentTraceContext,
    pub tools: AgentToolConfig,
    pub application: Option<ApplicationSemantics>,
}

pub struct AgentBuildRequestBuilder {
    identity: AgentIdentity,
    intent: AgentBuildIntent,
    system_prompt: String,
    services: AgentServices,
    capabilities: AgentCapabilitySet,
    lifecycle: AgentLifecycleConfig,
    trace: Option<AgentTraceContext>,
    tools: AgentToolConfig,
    application: Option<ApplicationSemantics>,
}

impl AgentBuildRequestBuilder {
    pub fn new(identity: AgentIdentity, intent: AgentBuildIntent) -> Self {
        Self {
            identity,
            intent,
            system_prompt: String::new(),
            services: AgentServices::default(),
            capabilities: AgentCapabilitySet::default(),
            lifecycle: AgentLifecycleConfig::default(),
            trace: None,
            tools: AgentToolConfig::default(),
            application: None,
        }
    }

    pub fn system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = system_prompt.into();
        self
    }

    pub fn services(mut self, services: AgentServices) -> Self {
        self.services = services;
        self
    }

    pub fn capabilities(mut self, capabilities: AgentCapabilitySet) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn lifecycle(mut self, lifecycle: AgentLifecycleConfig) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    pub fn trace(mut self, trace: AgentTraceContext) -> Self {
        self.trace = Some(trace);
        self
    }

    pub fn tools(mut self, tools: AgentToolConfig) -> Self {
        self.tools = tools;
        self
    }

    pub fn application(mut self, application: ApplicationSemantics) -> Self {
        self.application = Some(application);
        self
    }

    pub fn application_opt(mut self, application: Option<ApplicationSemantics>) -> Self {
        self.application = application;
        self
    }

    pub fn build(self) -> Result<AgentBuildRequest, String> {
        Ok(AgentBuildRequest {
            identity: self.identity,
            intent: self.intent,
            system_prompt: self.system_prompt,
            services: self.services,
            capabilities: self.capabilities,
            lifecycle: self.lifecycle,
            trace: self
                .trace
                .ok_or_else(|| "missing AgentTraceContext".to_string())?,
            tools: self.tools,
            application: self.application,
        })
    }
}

#[async_trait]
pub trait ToolkitContributor: Send + Sync {
    async fn contribute(
        &self,
        request: &AgentBuildRequest,
        toolkit: &mut Toolkit,
    ) -> Result<(), String>;
}

#[async_trait]
pub trait TracedAgentFactory: Send + Sync {
    type Output: Agent;

    async fn build(&self, request: AgentBuildRequest) -> Result<Self::Output, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionInput {
    pub identity: AgentIdentity,
    pub session_id: Option<String>,
    pub task_id: Option<TaskId>,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionOutput {
    pub agent_name: String,
    pub session_id: Option<String>,
    pub task_id: Option<TaskId>,
}

#[async_trait]
pub trait AgentExecutionLauncher: Send + Sync {
    async fn launch(
        &self,
        intent: AgentBuildIntent,
        input: AgentExecutionInput,
    ) -> Result<AgentExecutionOutput, String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_agent::AgentCapabilitySet;

    #[test]
    fn lifecycle_config_defaults_to_created() {
        let lifecycle = AgentLifecycleConfig::default();
        assert_eq!(lifecycle.initial_state, AgentState::Created);
        assert!(lifecycle.policy.is_none());
    }

    #[test]
    fn build_request_accepts_agent_core_abstractions() {
        let identity = AgentIdentity {
            app_id: ApplicationId::new(),
            agent_name: "planner".into(),
            session_id: Some("s1".into()),
        };
        let request = AgentBuildRequestBuilder::new(
            identity.clone(),
            AgentBuildIntent::PlannerDecomposition { goal_id: None },
        )
        .system_prompt("You are planner")
        .services(AgentServices::default())
        .capabilities(AgentCapabilitySet::default())
        .lifecycle(AgentLifecycleConfig::default())
        .trace(AgentTraceContext {
            session_id: Some("s1".into()),
            task_id: None,
            source_agent: "planner".into(),
        })
        .tools(AgentToolConfig::default())
        .application(ApplicationSemantics {
            workflow_name: Some("default".into()),
            entry_agent: Some("planner".into()),
            prompt_parts: Some(ApplicationPromptParts {
                role: "role".into(),
                constraints: "constraints".into(),
                tools: "tools".into(),
                handoff: "handoff".into(),
            }),
            tool_policy: ApplicationToolPolicy {
                allowed_tool_names: vec!["claude_code_execute".into()],
                execution_tool_names: vec!["claude_code_execute".into()],
                is_entry_agent: true,
            },
        })
        .build()
        .expect("request should build");

        assert_eq!(request.identity.agent_name, "planner");
        assert_eq!(request.identity, identity);
        assert_eq!(request.capabilities.len(), 0);
        assert!(request.trace.session_id.is_some());
        assert_eq!(
            request
                .application
                .as_ref()
                .and_then(|app| app.entry_agent.as_deref()),
            Some("planner")
        );
        let _ = request.services.memory_service();
        let _ = request.services.ipc_service();
        let _ = request.services.persist_service();
    }

    #[test]
    fn build_request_builder_requires_trace() {
        let result = AgentBuildRequestBuilder::new(
            AgentIdentity {
                app_id: ApplicationId::new(),
                agent_name: "planner".into(),
                session_id: Some("s1".into()),
            },
            AgentBuildIntent::RuntimeAgent,
        )
        .system_prompt("missing trace")
        .build();

        assert!(result.is_err());
    }
}
