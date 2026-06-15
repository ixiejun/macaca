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

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

pub mod app_executor;
pub mod bus;
pub mod callback;
pub mod event_factory;
pub mod fork_manager;
pub mod queue;
pub mod router;
pub mod worker;

pub use app_executor::{
    ApplicationExecutor, ApplicationExecutorConfig, ApplicationExecutorRegistry, WorkerHealth,
    WorkerState, WorkerSupervisorConfig,
};
pub use bus::{EventBus, SystemEvent};
pub use callback::CallbackDispatcher;
pub use event_factory::ExecutorEventFactory;
pub use fork_manager::{DelegateResult, ForkContext, ForkManager, HookEvent, MergeResult};
pub use macaca_proto::{AgentInfo, ApplicationId, TaskResult, TokenUsage};
pub use queue::ExecutionQueue;
pub use router::TaskRouter;
pub use worker::{ExecutorCommand, ExecutorEvent, TaskExecutor};

/// Re-export TaskId from the shared protocol crate (single source of truth).
pub use macaca_proto::TaskId;

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

/// Re-export DelegatedTask and TaskContext from the shared protocol crate.
pub use macaca_proto::orchestration::DelegatedTask;
pub use macaca_proto::TaskContext;

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

/// Trait for executing an agent with a given prompt.
/// This is the core abstraction that makes the system generic.
#[async_trait]
pub trait AgentRunner: Send + Sync {
    /// Execute an agent and return the result.
    ///
    /// # Arguments
    /// * `application_id` - ID of the application this agent belongs to
    /// * `agent_name` - Name of the agent to execute
    /// * `prompt` - The prompt/指令 to send to the agent
    /// * `context` - Optional execution context
    ///
    /// # Returns
    /// * `Ok(TaskResult)` - The result of execution
    /// * `Err(String)` - Error message if execution failed
    async fn execute_agent(
        &self,
        application_id: &ApplicationId,
        agent_name: &str,
        prompt: &str,
        context: Option<TaskContext>,
    ) -> Result<TaskResult, String>;

    /// Execute an agent with event callback for progress tracking.
    ///
    /// This method is similar to `execute_agent` but accepts an event callback
    /// that will be called during execution to report progress (thinking, tool calls, etc.)
    ///
    /// # Arguments
    /// * `application_id` - ID of the application this agent belongs to
    /// * `agent_name` - Name of the agent to execute
    /// * `prompt` - The prompt to send to the agent
    /// * `context` - Optional execution context
    /// * `event_tx` - Optional channel to send execution events
    ///
    /// Default implementation delegates to `execute_agent` without events.
    async fn execute_agent_with_events(
        &self,
        application_id: &ApplicationId,
        agent_name: &str,
        prompt: &str,
        context: Option<TaskContext>,
        event_tx: Option<mpsc::Sender<macaca_proto::AgentExecutionEvent>>,
    ) -> Result<TaskResult, String> {
        // Default implementation ignores event_tx and calls execute_agent
        let _ = event_tx; // suppress unused warning
        self.execute_agent(application_id, agent_name, prompt, context)
            .await
    }

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
    use chrono::Utc;

    #[test]
    fn test_task_id_generation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_delegated_task_defaults() {
        let app_id = ApplicationId::new();
        let task = DelegatedTask {
            id: TaskId::new(),
            application_id: app_id.clone(),
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
        assert_eq!(task.application_id, app_id);
        assert!(!task.parallel);
    }
}
