//! Orchestration tools for agent-to-agent coordination.
//!
//! These tools allow the coordinator agent to delegate tasks to other agents.

use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;

use crate::tool::{Tool, ToolCommand};

/// Type alias for the delegate callback function.
/// Arguments: (app_id, to_agent, prompt, priority, parallel, session_id) -> Result<task_id, error>
pub type DelegateCallback = Box<
    dyn Fn(
            String,
            String,
            String,
            u8,
            bool,
            Option<String>,
        ) -> futures::future::BoxFuture<'static, Result<String, String>>
        + Send
        + Sync,
>;

/// Tool for delegating a task to another agent.
pub struct DelegateTaskTool {
    /// Callback for service-backed task execution.
    /// Arguments: (app_id, to_agent, prompt, priority, parallel, session_id) -> Result<task_id, error>
    delegate_callback: Option<DelegateCallback>,
    /// Current session ID, shared with AppState so routes can set it before execution.
    pub session_id: Arc<RwLock<Option<String>>>,
}

impl DelegateTaskTool {
    /// Create a tool with a service-backed execution callback.
    /// The callback receives (app_id, to_agent, prompt, priority, parallel, session_id) and returns task_id.
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(
                String,
                String,
                String,
                u8,
                bool,
                Option<String>,
            ) -> futures::future::BoxFuture<'static, Result<String, String>>
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
            delegate_callback: None,
            session_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Create an empty tool with a pre-shared session_id Arc.
    pub fn empty_with_session_id(session_id: Arc<RwLock<Option<String>>>) -> Self {
        Self {
            delegate_callback: None,
            session_id,
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

    fn tool_schema(&self) -> serde_json::Value {
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

    #[instrument(name = "delegate_task", skip(self, command))]
    async fn invoke(&self, command: ToolCommand) -> MacacaResult<Value> {
        let input = command.input;
        let agent = input["agent"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("delegate_task requires 'agent' field".into()))?;
        let prompt = input["prompt"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("delegate_task requires 'prompt' field".into()))?;
        let priority = input["priority"].as_u64().unwrap_or(5) as u8;
        let parallel = input["parallel"].as_bool().unwrap_or(false);
        let app_id = input["app_id"].as_str().unwrap_or("").to_string();

        if let Some(ref callback) = self.delegate_callback {
            let session_id = self.session_id.read().await.clone();
            match callback(
                app_id,
                agent.to_string(),
                prompt.to_string(),
                priority,
                parallel,
                session_id,
            )
            .await
            {
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

        Err(MacacaError::Agent(
            "delegate_task requires a service-backed delegation callback".into(),
        ))
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
pub struct GetTaskResultTool {
    /// Callback for service-backed result retrieval.
    /// Arguments: (app_id, task_id) -> Result<TaskResultData, error>
    result_callback: Option<GetTaskResultCallback>,
}

impl GetTaskResultTool {
    /// Create a tool with a service-backed result retrieval callback.
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(
                String,
                String,
            ) -> futures::future::BoxFuture<'static, Result<TaskResultData, String>>
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

    fn tool_schema(&self) -> serde_json::Value {
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

    #[instrument(name = "get_task_result", skip(self, command))]
    async fn invoke(&self, command: ToolCommand) -> MacacaResult<Value> {
        let input = command.input;
        let task_id = input["task_id"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("get_task_result requires 'task_id' field".into()))?;
        let app_id = input["app_id"].as_str().unwrap_or("").to_string();

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
                    return Err(MacacaError::Agent(format!(
                        "Failed to get task result: {}",
                        e
                    )));
                }
            }
        }

        Err(MacacaError::Agent(
            "get_task_result requires a service-backed result callback".into(),
        ))
    }
}

/// Tool for listing available agents.
pub struct ListAgentsTool {
    agents_callback:
        Option<Box<dyn Fn() -> futures::future::BoxFuture<'static, Vec<Value>> + Send + Sync>>,
}

impl ListAgentsTool {
    pub fn new() -> Self {
        Self {
            agents_callback: None,
        }
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

    fn tool_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }

    async fn invoke(&self, _command: ToolCommand) -> MacacaResult<Value> {
        // Structured unavailable (2026-07-08 audit S14): when no agents provider
        // is wired, the previous code returned `{"agents": []}`, which is
        // indistinguishable from a real, legitimately empty agent registry. That
        // hides a wiring gap as a normal empty result. We now return a structured
        // error so "no provider" is explicit and distinct from "zero agents".
        let Some(ref callback) = self.agents_callback else {
            return Err(MacacaError::Agent(
                "agent listing unavailable: no agents provider is wired to this tool".into(),
            ));
        };
        let agents = callback().await;
        Ok(serde_json::json!({"agents": agents}))
    }
}
