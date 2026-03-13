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

use crate::tool::{Tool, ToolSet};

// ============================================================================
// Agent Executor Trait - abstracts how agents are actually executed
// This allows the orchestration tools to be generic across different applications
// ============================================================================

/// Result of executing an agent.
#[derive(Debug, Clone)]
pub struct AgentExecutionResult {
    pub success: bool,
    pub output: String,
    pub artifacts: Vec<String>,
}

/// Trait for executing agents - implemented by the application layer.
/// This is the key abstraction that makes orchestration application-agnostic.
pub trait AgentExecutor: Send + Sync {
    /// Execute an agent with a given prompt.
    fn execute_agent(
        &self,
        agent_name: &str,
        prompt: &str,
    ) -> futures::future::BoxFuture<'static, Result<AgentExecutionResult, String>>;
}

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

// ── DelegateTaskTool ─────────────────────────────────────────────────────────

/// Tool for delegating a task to another agent.
///
/// This allows the coordinator to distribute work to specialized agents.
pub struct DelegateTaskTool {
    state: Arc<RwLock<OrchestrationState>>,
    executor: Option<Arc<dyn AgentExecutor>>,
}

impl DelegateTaskTool {
    pub fn new(state: Arc<RwLock<OrchestrationState>>) -> Self {
        Self {
            state,
            executor: None,
        }
    }

    /// Set the agent executor that actually runs the agents.
    pub fn with_executor(mut self, executor: Arc<dyn AgentExecutor>) -> Self {
        self.executor = Some(executor);
        self
    }
}

#[async_trait]
impl Tool for DelegateTaskTool {
    fn name(&self) -> &str {
        "delegate_task"
    }

    fn description(&self) -> &str {
        "Delegate a task to another agent. Use this to distribute work to specialized agents like frontend, backend, or architect."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "agent": {
                    "type": "string",
                    "description": "Name of the target agent. Use list_agents tool to see available agents."
                },
                "prompt": {
                    "type": "string",
                    "description": "Clear description of what the agent should do"
                },
                "priority": {
                    "type": "integer",
                    "description": "Priority level 0-10 (default: 5)",
                    "minimum": 0,
                    "maximum": 10
                },
                "parallel": {
                    "type": "boolean",
                    "description": "Whether this task can run in parallel with others (default: false)"
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
        let agent_str = agent.to_string();
        let prompt_str = prompt.to_string();
        let task_id_for_log = task_id.clone();

        // If we have an executor, actually run the agent
        if let Some(ref executor) = self.executor {
            let agent_name = agent_str.clone();
            let task_id_clone = task_id.clone();
            let executor = Arc::clone(executor);
            let state = Arc::clone(&self.state);

            // Execute the agent asynchronously in background
            tokio::spawn(async move {
                let result = executor.execute_agent(&agent_name, &prompt_str).await;

                // Store the result
                let mut state_guard = state.write().await;
                state_guard.pending_tasks.remove(&task_id_clone);

                match result {
                    Ok(exec_result) => {
                        state_guard.completed_results.insert(
                            task_id_clone,
                            serde_json::json!({
                                "success": exec_result.success,
                                "output": exec_result.output,
                                "artifacts": exec_result.artifacts
                            }).to_string()
                        );
                    }
                    Err(e) => {
                        state_guard.completed_results.insert(
                            task_id_clone,
                            serde_json::json!({
                                "success": false,
                                "output": "",
                                "error": e
                            }).to_string()
                        );
                    }
                }
            });
        } else {
            // No executor - just store the task
            let mut state = self.state.write().await;
            state.pending_tasks.insert(task_id.clone(), (agent_str, prompt_str));
        }

        tracing::info!(
            task_id = %task_id_for_log,
            target_agent = %agent,
            prompt_preview = %prompt.chars().take(50).collect::<String>(),
            "task delegated"
        );

        Ok(serde_json::json!({
            "task_id": task_id,
            "agent": agent,
            "status": "delegated",
            "message": format!("Task delegated to {} agent. Task ID: {}", agent, task_id)
        }))
    }
}

// ── GetTaskResultTool ─────────────────────────────────────────────────────────

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
        "Get the result of a previously delegated task. Returns pending if not complete."
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

        // Check if task is complete
        if let Some(result) = state.completed_results.get(task_id) {
            return Ok(serde_json::json!({
                "task_id": task_id,
                "status": "completed",
                "result": result
            }));
        }

        // Check if task is still pending
        if state.pending_tasks.contains_key(task_id) {
            return Ok(serde_json::json!({
                "task_id": task_id,
                "status": "pending",
                "message": "Task is still in progress"
            }));
        }

        Err(MacacaError::Agent(format!("Task {} not found", task_id)))
    }
}

// ── ReportResultTool ──────────────────────────────────────────────────────────

/// Tool for an agent to report its task result back to coordinator.
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
                "task_id": {
                    "type": "string",
                    "description": "The task ID that was assigned"
                },
                "success": {
                    "type": "boolean",
                    "description": "Whether the task completed successfully"
                },
                "output": {
                    "type": "string",
                    "description": "The result or output of the task"
                },
                "error": {
                    "type": "string",
                    "description": "Error message if the task failed"
                }
            },
            "required": ["task_id", "success", "output"]
        })
    }

    #[instrument(name = "report_result", skip(self))]
    async fn execute(&self, input: Value) -> MacacaResult<Value> {
        let task_id = input["task_id"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("report_result requires 'task_id' field".into()))?;
        let success = input["success"]
            .as_bool()
            .ok_or_else(|| MacacaError::Agent("report_result requires 'success' field".into()))?;
        let output = input["output"]
            .as_str()
            .ok_or_else(|| MacacaError::Agent("report_result requires 'output' field".into()))?;

        let mut state = self.state.write().await;

        // Remove from pending and add to completed
        state.pending_tasks.remove(task_id);
        state.completed_results.insert(task_id.to_string(), output.to_string());

        tracing::info!(
            task_id = %task_id,
            success = success,
            "task result reported"
        );

        Ok(serde_json::json!({
            "task_id": task_id,
            "status": "recorded",
            "message": if success {
                "Task result recorded successfully"
            } else {
                "Task failure recorded"
            }
        }))
    }
}

// ── ListAgentsTool ────────────────────────────────────────────────────────────

/// Tool for listing available agents and their capabilities.
/// NOTE: The agent list should be dynamically provided via set_agents_callback.
/// If no callback is set, returns an empty list.
pub struct ListAgentsTool {
    agents_callback: Option<Box<dyn Fn() -> futures::future::BoxFuture<'static, Vec<serde_json::Value>> + Send + Sync>>,
}

impl ListAgentsTool {
    pub fn new() -> Self {
        Self {
            agents_callback: None,
        }
    }

    /// Set an async callback to dynamically fetch agents.
    /// This allows the tool to query the actual registered agents at runtime.
    pub fn with_agents_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> futures::future::BoxFuture<'static, Vec<serde_json::Value>> + Send + Sync + 'static,
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
        "List all available agents and their capabilities for task delegation."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    #[instrument(name = "list_agents", skip(self))]
    async fn execute(&self, _input: Value) -> MacacaResult<Value> {
        let agents = if let Some(ref callback) = self.agents_callback {
            callback().await
        } else {
            vec![]
        };

        Ok(serde_json::json!({
            "agents": agents
        }))
    }
}

/// Create the default orchestration tool set.
pub fn orchestration_tool_set(state: Arc<RwLock<OrchestrationState>>) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(DelegateTaskTool::new(Arc::clone(&state))),
        Box::new(GetTaskResultTool::new(Arc::clone(&state))),
        Box::new(ReportResultTool::new(Arc::clone(&state))),
        Box::new(ListAgentsTool::new()),
    ]
}
