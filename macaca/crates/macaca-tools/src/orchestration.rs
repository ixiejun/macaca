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

/// Type alias for the delegate callback function.
pub type DelegateCallback = Box<
    dyn Fn(String, String, String, u8, bool) -> futures::future::BoxFuture<'static, Result<String, String>>
        + Send
        + Sync,
>;

/// Tool for delegating a task to another agent.
///
/// This tool supports two modes:
/// 1. Legacy mode with OrchestrationState (just stores tasks)
/// 2. Real execution mode with callback (actually executes the delegated task)
pub struct DelegateTaskTool {
    /// Legacy state storage (optional).
    state: Option<Arc<RwLock<OrchestrationState>>>,
    /// Callback for real task execution.
    /// Arguments: (app_id, to_agent, prompt, priority, parallel) -> Result<task_id, error>
    delegate_callback: Option<DelegateCallback>,
}

impl DelegateTaskTool {
    /// Create a legacy tool that just stores tasks in memory.
    pub fn new(state: Arc<RwLock<OrchestrationState>>) -> Self {
        Self {
            state: Some(state),
            delegate_callback: None,
        }
    }

    /// Create a tool with a real execution callback.
    /// The callback receives (app_id, to_agent, prompt, priority, parallel) and returns task_id.
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(String, String, String, u8, bool) -> futures::future::BoxFuture<'static, Result<String, String>>
            + Send
            + Sync
            + 'static,
    {
        self.delegate_callback = Some(Box::new(callback));
        self
    }

    /// Create an empty tool (use with_callback to configure).
    pub fn empty() -> Self {
        Self {
            state: None,
            delegate_callback: None,
        }
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
                },
                "priority": {
                    "type": "integer",
                    "description": "Priority level (0-10, higher = more urgent)",
                    "default": 5
                },
                "parallel": {
                    "type": "boolean",
                    "description": "Whether this task can run in parallel with others",
                    "default": false
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
        let priority = input["priority"]
            .as_u64()
            .unwrap_or(5) as u8;
        let parallel = input["parallel"]
            .as_bool()
            .unwrap_or(false);
        let app_id = input["app_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // If we have a real callback, use it for actual execution
        if let Some(ref callback) = self.delegate_callback {
            match callback(app_id, agent.to_string(), prompt.to_string(), priority, parallel).await {
                Ok(task_id) => {
                    return Ok(serde_json::json!({
                        "task_id": task_id,
                        "agent": agent,
                        "status": "delegated",
                        "priority": priority,
                        "parallel": parallel
                    }));
                }
                Err(e) => {
                    return Err(MacacaError::Agent(format!("Delegation failed: {}", e)));
                }
            }
        }

        // Fall back to legacy mode (just store in memory)
        let task_id = uuid::Uuid::new_v4().to_string();

        if let Some(ref state) = self.state {
            let mut state = state.write().await;
            state.pending_tasks.insert(task_id.clone(), (agent.to_string(), prompt.to_string()));
        }

        Ok(serde_json::json!({
            "task_id": task_id,
            "agent": agent,
            "status": "delegated",
            "priority": priority,
            "parallel": parallel
        }))
    }
}

/// Type alias for the get task result callback function.
pub type GetTaskResultCallback = Box<
    dyn Fn(String, String) -> futures::future::BoxFuture<'static, Result<TaskResultData, String>>
        + Send
        + Sync,
>;

/// Result data from a delegated task.
#[derive(Debug, Clone)]
pub struct TaskResultData {
    pub status: String,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Tool for getting the result of a delegated task.
///
/// This tool supports two modes:
/// 1. Legacy mode with OrchestrationState (checks in-memory storage)
/// 2. Real execution mode with callback (checks ApplicationExecutor)
pub struct GetTaskResultTool {
    /// Legacy state storage (optional).
    state: Option<Arc<RwLock<OrchestrationState>>>,
    /// Callback for real result retrieval.
    /// Arguments: (app_id, task_id) -> Result<TaskResultData, error>
    result_callback: Option<GetTaskResultCallback>,
}

impl GetTaskResultTool {
    /// Create a legacy tool that checks in-memory storage.
    pub fn new(state: Arc<RwLock<OrchestrationState>>) -> Self {
        Self {
            state: Some(state),
            result_callback: None,
        }
    }

    /// Create a tool with a real result retrieval callback.
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(String, String) -> futures::future::BoxFuture<'static, Result<TaskResultData, String>>
            + Send
            + Sync
            + 'static,
    {
        self.result_callback = Some(Box::new(callback));
        self
    }

    /// Create an empty tool (use with_callback to configure).
    pub fn empty() -> Self {
        Self {
            state: None,
            result_callback: None,
        }
    }
}

#[async_trait]
impl Tool for GetTaskResultTool {
    fn name(&self) -> &str {
        "get_task_result"
    }

    fn description(&self) -> &str {
        "Get the result of a previously delegated task. WARNING: Do NOT poll this! The system uses Hook-based notification. Only call this ONCE after receiving a task completion notification from the system."
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
        let app_id = input["app_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // If we have a real callback, use it
        if let Some(ref callback) = self.result_callback {
            match callback(app_id, task_id.to_string()).await {
                Ok(result) => {
                    return Ok(serde_json::json!({
                        "task_id": task_id,
                        "status": result.status,
                        "output": result.output,
                        "error": result.error
                    }));
                }
                Err(e) => {
                    return Err(MacacaError::Agent(format!("Failed to get task result: {}", e)));
                }
            }
        }

        // Fall back to legacy mode
        if let Some(ref state) = self.state {
            let state = state.read().await;

            if let Some(result) = state.completed_results.get(task_id) {
                return Ok(serde_json::json!({
                    "task_id": task_id,
                    "status": "completed",
                    "output": result
                }));
            }

            if state.pending_tasks.contains_key(task_id) {
                return Ok(serde_json::json!({
                    "task_id": task_id,
                    "status": "pending"
                }));
            }
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
