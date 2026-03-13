//! Orchestration tools for agent-to-agent coordination.
//!
//! These tools allow the coordinator agent to delegate tasks to other agents.

use async_trait::async_trait;
use futures::FutureExt;
use macaca_proto::{MacacaError, MacacaResult};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;

use crate::tool::Tool;

/// Shared state for orchestration between agents.
pub struct OrchestrationState {
    /// Pending delegated tasks (task_id -> (target_agent, prompt)).
    pub pending_tasks: std::collections::HashMap<String, (String, String)>,
    /// Completed task results.
    pub completed_results: std::collections::HashMap<String, String>,
}

impl Default for OrchestrationState {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchestrationState {
    pub fn new() -> Self {
        Self {
            pending_tasks: std::collections::HashMap::new(),
            completed_results: std::collections::HashMap::new(),
        }
    }
}

/// Tool for delegating a task to another agent.
pub struct DelegateTaskTool {
    state: Arc<RwLock<OrchestrationState>>,
}

impl DelegateTaskTool {
    pub fn new(state: Arc<RwLock<OrchestrationState>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for DelegateTaskTool {
    fn name(&self) -> &str {
        "delegate_task"
    }

    fn description(&self) -> &str {
        "Delegate a task to another agent. Use this to distribute work to specialized agents."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "agent": {
                    "type": "string",
                    "description": "Name of the target agent"
                },
                "prompt": {
                    "type": "string",
                    "description": "Clear description of what the agent should do"
                }
            },
            "required": ["agent", "prompt"]
        })
    }

    #[instrument(name = "delegate_task", skip(self))]
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let agent = input["agent"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("delegate_task requires 'agent' field".into()))?;
        let prompt = input["prompt"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("delegate_task requires 'prompt' field".into()))?;

        let task_id = uuid::Uuid::new_v4().to_string();

        let mut state = self.state.write().await;
        state.pending_tasks.insert(task_id.clone(), (agent.to_string(), prompt.to_string()));

        Ok(serde_json::json!({
            "task_id": task_id,
            "agent": agent,
            "status": "delegated"
        }))
    }
}

/// Tool for getting the result of a delegated task.
pub struct GetTaskResultTool {
    state: Arc<RwLock<OrchestrationState>>,
}

impl GetTaskResultTool {
    pub fn new(state: Arc<RwLock<OrchestrationState>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for GetTaskResultTool {
    fn name(&self) -> &str {
        "get_task_result"
    }

    fn description(&self) -> &str {
        "Get the result of a previously delegated task."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "The task ID returned by delegate_task"
                }
            },
            "required": ["task_id"]
        })
    }

    #[instrument(name = "get_task_result", skip(self))]
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id = input["task_id"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("get_task_result requires 'task_id' field".into()))?;

        let state = self.state.read().await;

        if let Some(result) = state.completed_results.get(task_id) {
            return Ok(serde_json::json!({
                "task_id": task_id,
                "status": "completed",
                "result": result
            }));
        }

        if state.pending_tasks.contains_key(task_id) {
            return Ok(serde_json::json!({
                "task_id": task_id,
                "status": "pending"
            }));
        }

        Err(MacacaError::Agent(format!("Task {} not found", task_id)))
    }
}

/// Tool for an agent to report its task result.
pub struct ReportResultTool {
    state: Arc<RwLock<OrchestrationState>>,
}

impl ReportResultTool {
    pub fn new(state: Arc<RwLock<OrchestrationState>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for ReportResultTool {
    fn name(&self) -> &str {
        "report_result"
    }

    fn description(&self) -> &str {
        "Report task completion result back to the coordinator."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": {"type": "string"},
                "success": {"type": "boolean"},
                "output": {"type": "string"}
            },
            "required": ["task_id", "success", "output"]
        })
    }

    #[instrument(name = "report_result", skip(self))]
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id = input["task_id"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("report_result requires 'task_id' field".into()))?;
        let output = input["output"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("report_result requires 'output' field".into()))?;

        let mut state = self.state.write().await;
        state.pending_tasks.remove(task_id);
        state.completed_results.insert(task_id.to_string(), output.to_string());

        Ok(serde_json::json!({
            "task_id": task_id,
            "status": "recorded"
        }))
    }
}

/// Tool for listing available agents.
pub struct ListAgentsTool {
    agents_callback: Option<Box<dyn Fn() -> futures::future::BoxFuture<'static, Vec<Value>> + Send + Sync>>,
}

impl ListAgentsTool {
    pub fn new() -> Self {
        Self { agents_callback: None }
    }

    pub fn with_agents_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> futures::future::BoxFuture<'static, Vec<Value>> + Send + Sync + 'static,
    {
        self.agents_callback = Some(Box::new(callback));
        self
    }
}

impl Default for ListAgentsTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ListAgentsTool {
    fn name(&self) -> &str {
        "list_agents"
    }

    fn description(&self) -> &str {
        "List all available agents and their capabilities."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }

    async fn execute(&self, _input: Value) -> MacacaResult<Value> {
        let agents = if let Some(ref callback) = self.agents_callback {
            callback().await
        } else {
            vec![]
        };
        Ok(serde_json::json!({"agents": agents}))
    }
}
