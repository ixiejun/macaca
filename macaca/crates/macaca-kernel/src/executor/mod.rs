//! Agent Executor - Event-driven task execution system for multi-agent orchestration.
//!
//! This module provides a generic, extensible framework for executing delegated tasks
//! across any number of agents. The system is designed to be application-agnostic
//! and can handle any agent type (backend, frontend, architect, operations, design, etc.)
//! without requiring changes to the底层代码.
//!
//! # Architecture
//!
//! ```text
//! Coordinator Agent
//!        │
//!        │ delegate_task(agent, prompt)
//!        ▼
//! ┌─────────────────────────────────────────┐
//! │           ExecutionQueue                  │  Priority-based task queue
//! └────────────────┬────────────────────────┘
//!                  │
//!                  ▼
//! ┌─────────────────────────────────────────┐
//! │           TaskExecutor (Worker)          │  Executes agents
//! └────────────────┬────────────────────────┘
//!                  │
//!                  ▼  results + events
//! ┌─────────────────────────────────────────┐
//! │             EventBus                     │  Publish-subscribe
//! └────────────────┬────────────────────────┘
//!                  │
//!      ┌───────────┴───────────┐
//!      ▼                       ▼
//! Callback              Coordinator
//! Dispatcher            (receives result)
//! ```

use std::cmp::Ordering;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;


pub mod app_executor;
pub mod bus;
pub mod callback;
pub mod queue;
pub mod router;
pub mod worker;

pub use app_executor::{ApplicationExecutor, ApplicationExecutorConfig, ApplicationExecutorRegistry};
pub use bus::{EventBus, SystemEvent};
pub use callback::CallbackDispatcher;
pub use queue::ExecutionQueue;
pub use router::TaskRouter;
pub use worker::{TaskExecutor, ExecutorCommand, ExecutorEvent};

/// Unique identifier for a delegated task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a delegated task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task has been queued but not yet started
    Queued,
    /// Task is currently being executed
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was cancelled
    Cancelled,
}

/// A delegated task waiting to be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTask {
    /// Unique task identifier
    pub id: TaskId,
    /// Application ID this task belongs to (for isolation)
    pub application_id: String,
    /// Name of the agent that delegated this task
    pub from_agent: String,
    /// Name of the agent that should execute this task
    pub to_agent: String,
    /// The actual prompt/指令 to execute
    pub prompt: String,
    /// Priority level (0-10, higher = more urgent)
    pub priority: u8,
    /// Whether this task can run in parallel with others
    pub parallel: bool,
    /// When the task was created
    pub created_at: DateTime<Utc>,
    /// Optional deadline for task completion
    pub deadline: Option<DateTime<Utc>>,
    /// Parent task ID (if this is a subtask)
    pub parent_task: Option<TaskId>,
    /// Additional context for execution
    pub context: Option<TaskContext>,
}

/// Additional context for task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    /// Session ID for conversation continuity
    pub session_id: Option<String>,
    /// Files or artifacts relevant to the task
    pub artifacts: Vec<String>,
    /// Environment variables or configuration
    pub env: std::collections::HashMap<String, String>,
}

impl Default for TaskContext {
    fn default() -> Self {
        Self {
            session_id: None,
            artifacts: vec![],
            env: std::collections::HashMap::new(),
        }
    }
}

/// Result of a completed task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// The task ID this result is for
    pub task_id: TaskId,
    /// Whether the task completed successfully
    pub success: bool,
    /// The output/content from the agent
    pub output: String,
    /// Error message if failed
    pub error: Option<String>,
    /// Any artifacts/files created during execution
    pub artifacts: Vec<String>,
    /// When the task completed
    pub completed_at: DateTime<Utc>,
    /// Token usage statistics
    pub tokens_used: Option<TokenUsage>,
}

/// Token usage from LLM calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Decision from the task router about which agent should handle a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// The selected agent name
    pub agent_name: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Reasoning for this decision
    pub reasoning: String,
    /// Alternative candidates if primary fails
    pub fallback_agents: Vec<String>,
}

/// Information about an available agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique agent identifier
    pub id: String,
    /// Agent name (e.g., "backend", "frontend", "architect")
    pub name: String,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Current load (number of running tasks)
    pub current_load: u32,
    /// Maximum concurrent tasks this agent can handle
    pub max_load: u32,
    /// Whether the agent is currently available
    pub available: bool,
}

/// Trait for executing an agent with a given prompt.
/// This is the core abstraction that makes the system generic.
#[async_trait]
pub trait AgentRunner: Send + Sync {
    /// Execute an agent and return the result.
    ///
    /// # Arguments
    /// * `agent_name` - Name of the agent to execute
    /// * `prompt` - The prompt/指令 to send to the agent
    /// * `context` - Optional execution context
    ///
    /// # Returns
    /// * `Ok(TaskResult)` - The result of execution
    /// * `Err(String)` - Error message if execution failed
    async fn execute_agent(
        &self,
        agent_name: &str,
        prompt: &str,
        context: Option<TaskContext>,
    ) -> Result<TaskResult, String>;

    /// Get information about all available agents.
    async fn list_agents(&self) -> Vec<AgentInfo>;

    /// Check if a specific agent exists.
    async fn agent_exists(&self, agent_name: &str) -> bool;
}

/// Builder for creating an ExecutionQueue with custom configuration.
#[derive(Debug)]
pub struct ExecutionQueueBuilder {
    max_parallel: usize,
    max_queue_size: usize,
}

impl Default for ExecutionQueueBuilder {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            max_queue_size: 100,
        }
    }
}

impl ExecutionQueueBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_parallel(mut self, n: usize) -> Self {
        self.max_parallel = n;
        self
    }

    pub fn max_queue_size(mut self, n: usize) -> Self {
        self.max_queue_size = n;
        self
    }

    pub fn build(self) -> ExecutionQueue {
        ExecutionQueue::new(self.max_parallel, self.max_queue_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_generation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_delegated_task_defaults() {
        let task = DelegatedTask {
            id: TaskId::new(),
            application_id: "app-1".into(),
            from_agent: "coordinator".into(),
            to_agent: "backend".into(),
            prompt: "Write a smart contract".into(),
            priority: 5,
            parallel: false,
            created_at: Utc::now(),
            deadline: None,
            parent_task: None,
            context: None,
        };

        assert_eq!(task.priority, 5);
        assert_eq!(task.application_id, "app-1");
        assert!(!task.parallel);
    }
}
