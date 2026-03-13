//! Agent Orchestrator - coordinates task delegation between agents.
//!
//! This module provides:
//! - Dynamic task routing based on LLM analysis
//! - Parallel execution of delegated tasks
//! - Result aggregation and reporting
//! - Agent capability matching

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use macaca_proto::{
    AgentId, TaskId, ApplicationId,
    orchestration::{
        DelegatedTask, DelegatedTaskResult, OrchestrationCommand,
        OrchestrationEvent, RoutingDecision, AgentRouting, AgentCapability,
    },
};

/// Maximum pending tasks in the queue.
const MAX_PENDING_TASKS: usize = 100;

/// Orchestrator that manages task delegation between agents.
pub struct AgentOrchestrator {
    /// Application ID this orchestrator manages.
    app_id: ApplicationId,
    /// Pending tasks waiting to be executed.
    pending_tasks: Arc<RwLock<HashMap<TaskId, DelegatedTask>>>,
    /// Completed task results.
    completed_tasks: Arc<RwLock<HashMap<TaskId, DelegatedTaskResult>>>,
    /// Agent capabilities registry.
    agent_capabilities: Arc<RwLock<HashMap<String, Vec<AgentCapability>>>>,
    /// Event sender for orchestration events.
    event_tx: mpsc::Sender<OrchestrationEvent>,
}

impl AgentOrchestrator {
    /// Create a new orchestrator for an application.
    pub fn new(app_id: ApplicationId, event_tx: mpsc::Sender<OrchestrationEvent>) -> Self {
        Self {
            app_id,
            pending_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(HashMap::new())),
            agent_capabilities: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    /// Register capabilities for an agent.
    pub async fn register_agent_capabilities(&self, agent_name: &str, capabilities: Vec<AgentCapability>) {
        let mut caps = self.agent_capabilities.write().await;
        caps.insert(agent_name.to_string(), capabilities);
    }

    /// Get all registered agents and their capabilities.
    pub async fn get_agent_registry(&self) -> HashMap<String, Vec<AgentCapability>> {
        self.agent_capabilities.read().await.clone()
    }

    /// Delegate a task to another agent.
    pub async fn delegate_task(
        &self,
        from_agent: AgentId,
        to_agent_name: &str,
        prompt: String,
        priority: u8,
        parallel: bool,
    ) -> TaskId {
        let task_id = TaskId::new();
        let task = DelegatedTask {
            id: task_id,
            from_agent,
            to_agent: AgentId::new(), // Will be resolved when executed
            prompt: prompt.clone(),
            priority,
            created_at: chrono::Utc::now(),
            deadline: None,
            parent_task: None,
            context: None,
        };

        // Store task
        {
            let mut pending = self.pending_tasks.write().await;
            pending.insert(task_id, task);
        }

        // Emit event
        let _ = self.event_tx.send(OrchestrationEvent::TaskDelegated {
            task_id,
            from_agent: format!("{}", from_agent.0),
            to_agent: to_agent_name.to_string(),
            prompt: prompt.clone(),
        }).await;

        task_id
    }

    /// Get the next task to execute.
    pub async fn get_next_task(&self) -> Option<DelegatedTask> {
        let pending = self.pending_tasks.read().await;
        // Find highest priority task
        pending.values()
            .max_by_key(|t| t.priority)
            .cloned()
    }

    /// Report task completion.
    pub async fn report_completion(&self, result: DelegatedTaskResult) {
        let task_id = result.task_id;

        // Remove from pending
        {
            let mut pending = self.pending_tasks.write().await;
            pending.remove(&task_id);
        }

        // Store result
        {
            let mut completed = self.completed_tasks.write().await;
            completed.insert(task_id, result.clone());
        }

        // Emit event
        let _ = self.event_tx.send(OrchestrationEvent::TaskCompleted {
            task_id,
            agent: format!("{}", result.agent_id.0),
            success: result.success,
            output_preview: result.output.chars().take(100).collect(),
        }).await;
    }

    /// Get results for specific tasks.
    pub async fn get_results(&self, task_ids: &[TaskId]) -> Vec<DelegatedTaskResult> {
        let completed = self.completed_tasks.read().await;
        task_ids.iter()
            .filter_map(|id| completed.get(id).cloned())
            .collect()
    }

    /// Check if all tasks are complete.
    pub async fn all_tasks_complete(&self) -> bool {
        let pending = self.pending_tasks.read().await;
        pending.is_empty()
    }

    /// Match a prompt to the best suited agent based on capabilities.
    pub async fn find_best_agent(&self, prompt: &str) -> Option<String> {
        let capabilities = self.agent_capabilities.read().await;
        let prompt_lower = prompt.to_lowercase();

        // Score each agent based on keyword matches
        let mut best_agent: Option<String> = None;
        let mut best_score = 0;

        for (agent_name, caps) in capabilities.iter() {
            let mut score = 0;
            for cap in caps {
                // Check keywords
                for keyword in &cap.keywords {
                    if prompt_lower.contains(&keyword.to_lowercase()) {
                        score += 2;
                    }
                }
                // Check examples (partial match)
                for example in &cap.examples {
                    if prompt_lower.contains(&example.to_lowercase()) {
                        score += 1;
                    }
                }
            }

            if score > best_score {
                best_score = score;
                best_agent = Some(agent_name.clone());
            }
        }

        // Default to coordinator if no match
        best_agent.or_else(|| Some("coordinator".to_string()))
    }

    /// Parse orchestration command from LLM tool call.
    pub fn parse_command(tool_name: &str, tool_input: &serde_json::Value) -> Option<OrchestrationCommand> {
        match tool_name {
            "delegate_task" => {
                let agent = tool_input.get("agent")?.as_str()?.to_string();
                let prompt = tool_input.get("prompt")?.as_str()?.to_string();
                let priority = tool_input.get("priority").and_then(|v| v.as_u64()).map(|v| v as u8);
                let parallel = tool_input.get("parallel").and_then(|v| v.as_bool());

                Some(OrchestrationCommand::Delegate {
                    agent,
                    prompt,
                    priority,
                    parallel,
                    context: None,
                })
            }
            "aggregate_results" => {
                use macaca_proto::orchestration::AggregationStrategy;
                let strategy = match tool_input.get("strategy")?.as_str()? {
                    "concat" => AggregationStrategy::Concat,
                    "first_success" => AggregationStrategy::FirstSuccess,
                    "all_success" => AggregationStrategy::AllSuccess,
                    "consensus" => AggregationStrategy::Consensus,
                    _ => AggregationStrategy::Concat,
                };
                Some(OrchestrationCommand::Aggregate {
                    task_ids: vec![], // Will be filled from context
                    strategy,
                })
            }
            "report_to_coordinator" => {
                let success = tool_input.get("success")?.as_bool()?;
                let output = tool_input.get("output")?.as_str()?.to_string();
                let error = tool_input.get("error").and_then(|v| v.as_str()).map(|s| s.to_string());

                Some(OrchestrationCommand::Report {
                    success,
                    output,
                    error,
                })
            }
            _ => None,
        }
    }
}

/// Builder for creating orchestrators with custom configuration.
pub struct OrchestratorBuilder {
    app_id: ApplicationId,
    event_tx: Option<mpsc::Sender<OrchestrationEvent>>,
}

impl OrchestratorBuilder {
    pub fn new(app_id: ApplicationId) -> Self {
        Self {
            app_id,
            event_tx: None,
        }
    }

    pub fn with_event_channel(mut self, tx: mpsc::Sender<OrchestrationEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    pub fn build(self) -> AgentOrchestrator {
        let (tx, _) = mpsc::channel(64);
        AgentOrchestrator::new(
            self.app_id,
            self.event_tx.unwrap_or(tx),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn delegate_and_complete_task() {
        let (tx, _rx) = mpsc::channel(64);
        let app_id = ApplicationId::new();
        let orchestrator = AgentOrchestrator::new(app_id, tx);

        // Register an agent
        orchestrator.register_agent_capabilities("frontend", vec![
            AgentCapability {
                name: "frontend_development".into(),
                description: "Build UI components".into(),
                keywords: vec!["ui".into(), "component".into(), "react".into()],
                examples: vec!["create button".into(), "build form".into()],
            }
        ]).await;

        // Delegate a task
        let from = AgentId::new();
        let task_id = orchestrator.delegate_task(
            from,
            "frontend",
            "Create a login form".into(),
            5,
            false,
        ).await;

        // Get pending task
        let task = orchestrator.get_next_task().await;
        assert!(task.is_some());
        assert_eq!(task.unwrap().id, task_id);

        // Complete the task
        let result = DelegatedTaskResult {
            task_id,
            agent_id: AgentId::new(),
            success: true,
            output: "Login form created".into(),
            error: None,
            artifacts: vec![],
            completed_at: chrono::Utc::now(),
            tokens_used: None,
        };
        orchestrator.report_completion(result).await;

        // Check all tasks complete
        assert!(orchestrator.all_tasks_complete().await);
    }

    #[tokio::test]
    async fn find_best_agent_by_capability() {
        let (tx, _rx) = mpsc::channel(64);
        let app_id = ApplicationId::new();
        let orchestrator = AgentOrchestrator::new(app_id, tx);

        // Register agents
        orchestrator.register_agent_capabilities("frontend", vec![
            AgentCapability {
                name: "frontend_dev".into(),
                description: "Frontend development".into(),
                keywords: vec!["ui".into(), "react".into(), "css".into()],
                examples: vec![],
            }
        ]).await;

        orchestrator.register_agent_capabilities("backend", vec![
            AgentCapability {
                name: "backend_dev".into(),
                description: "Backend development".into(),
                keywords: vec!["api".into(), "database".into(), "server".into()],
                examples: vec![],
            }
        ]).await;

        // Test matching
        let best = orchestrator.find_best_agent("Create a new REST API endpoint").await;
        assert_eq!(best, Some("backend".to_string()));

        let best = orchestrator.find_best_agent("Build a beautiful UI component").await;
        assert_eq!(best, Some("frontend".to_string()));
    }
}
