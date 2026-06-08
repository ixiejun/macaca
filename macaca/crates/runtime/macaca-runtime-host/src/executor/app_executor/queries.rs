//! Read-only queries, subscriptions, and event broadcast helpers.

use std::sync::Arc;

use super::executor::ApplicationExecutor;
use crate::executor::{
    AgentInfo, AgentRunner, ApplicationId, CallbackDispatcher, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkManager,
    RoutingDecision, SystemEvent, TaskContext, TaskId, TaskResult, TaskRouter, TaskStatus,
};

impl ApplicationExecutor {
    /// Get the result of a delegated task.
    pub async fn get_task_result(&self, task_id: TaskId) -> Option<TaskResult> {
        self.queue.get_result(&task_id).await
    }

    /// Get the status of a task.
    pub async fn get_task_status(&self, task_id: &TaskId) -> Option<TaskStatus> {
        self.queue.get_status(task_id).await
    }

    /// List all agents in this application.
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        self.agents.read().await.clone()
    }

    /// Update agent availability.
    pub async fn update_agent_availability(&self, agent_name: &str, available: bool) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.iter_mut().find(|a| a.name == agent_name) {
            agent.available = available;
        }
    }

    /// Subscribe to system events.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<crate::executor::bus::Event> {
        self.event_bus.subscribe_to_broadcast()
    }

    /// Subscribe to executor events (for external subscribers like SSE).
    ///
    /// This returns a broadcast receiver that will receive all executor events
    /// including task lifecycle events and agent execution events.
    pub fn subscribe_to_events(&self) -> tokio::sync::broadcast::Receiver<ExecutorEvent> {
        self.event_broadcast.subscribe()
    }

    /// Broadcast an executor event to all subscribers.
    ///
    /// Used by external execution paths (e.g. WorkerLoop via FrameworkRunner)
    /// to emit events into the same broadcast channel that AppExecutor uses.
    pub fn broadcast_event(&self, event: ExecutorEvent) {
        let _ = self.event_broadcast.send(event);
    }

    /// Get the Fork Manager for Fork-Join workflow.
    pub fn fork_manager(&self) -> Arc<ForkManager> {
        Arc::clone(&self.fork_manager)
    }
}
