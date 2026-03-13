//! Agent Orchestration Types - for dynamic agent coordination.
//!
//! This module provides types for:
//! - Agent-to-agent task delegation
//! - Parallel task execution
//! - Result reporting and aggregation
//! - LLM-based intelligent routing

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AgentId, TaskId};

// ── Task Delegation ──

/// A task delegated from one agent to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTask {
    /// Unique task identifier.
    pub id: TaskId,
    /// Agent that delegated this task.
    pub from_agent: AgentId,
    /// Agent assigned to handle this task.
    pub to_agent: AgentId,
    /// Task description/prompt.
    pub prompt: String,
    /// Task priority (higher = more important).
    pub priority: u8,
    /// When this task was created.
    pub created_at: DateTime<Utc>,
    /// When this task should complete by (optional).
    pub deadline: Option<DateTime<Utc>>,
    /// Parent task ID if this is a subtask.
    pub parent_task: Option<TaskId>,
    /// Additional context to pass to the agent.
    pub context: Option<DelegatedTaskContext>,
}

/// Context passed along with a delegated task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTaskContext {
    /// Conversation history relevant to this task.
    pub conversation_snippet: Option<String>,
    /// Files or resources the agent should know about.
    pub relevant_resources: Vec<String>,
    /// Constraints or requirements.
    pub constraints: Vec<String>,
}

// ── Task Result ──

/// Result of a delegated task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTaskResult {
    /// The task this result is for.
    pub task_id: TaskId,
    /// Agent that produced this result.
    pub agent_id: AgentId,
    /// Whether the task completed successfully.
    pub success: bool,
    /// The output/result content.
    pub output: String,
    /// Error message if failed.
    pub error: Option<String>,
    /// Artifacts produced (files created, etc.).
    pub artifacts: Vec<String>,
    /// When the task was completed.
    pub completed_at: DateTime<Utc>,
    /// Tokens used during execution.
    pub tokens_used: Option<DelegatedTokenUsage>,
}

/// Token usage information for delegated tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ── Orchestration Commands ──

/// Commands that an agent can issue to the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrchestrationCommand {
    /// Delegate a task to another agent.
    Delegate {
        /// Target agent name (e.g., "frontend", "backend", "architect").
        agent: String,
        /// Task description.
        prompt: String,
        /// Priority level (0-10).
        priority: Option<u8>,
        /// Whether to run in parallel with other pending tasks.
        parallel: Option<bool>,
        /// Context to pass.
        context: Option<DelegatedTaskContext>,
    },
    /// Broadcast a message to multiple agents.
    Broadcast {
        /// Target agents (empty = all agents).
        agents: Vec<String>,
        /// Message content.
        message: String,
    },
    /// Wait for specific tasks to complete.
    WaitFor {
        /// Task IDs to wait for.
        task_ids: Vec<TaskId>,
    },
    /// Aggregate results from multiple tasks.
    Aggregate {
        /// Task IDs to aggregate.
        task_ids: Vec<TaskId>,
        /// How to combine results.
        strategy: AggregationStrategy,
    },
    /// Report completion of current task.
    Report {
        /// Whether the task succeeded.
        success: bool,
        /// Output content.
        output: String,
        /// Any error message.
        error: Option<String>,
    },
}

/// Strategy for aggregating multiple task results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationStrategy {
    /// Concatenate all outputs.
    Concat,
    /// Take the first successful result.
    FirstSuccess,
    /// Take all successful results, ignore failures.
    AllSuccess,
    /// Majority vote on output.
    Consensus,
}

// ── Routing Decision ──

/// LLM's decision on how to route a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Whether this request should be delegated.
    pub should_delegate: bool,
    /// Target agents if delegating.
    pub target_agents: Vec<AgentRouting>,
    /// Whether agents can work in parallel.
    pub parallel_execution: bool,
    /// Reasoning for this routing decision.
    pub reasoning: String,
}

/// Information about routing to a specific agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRouting {
    /// Agent name.
    pub name: String,
    /// Specific task for this agent.
    pub task: String,
    /// Expected output type.
    pub expected_output: String,
}

// ── Agent Capability ──

/// Describes what an agent can do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    /// Capability name (e.g., "frontend_development").
    pub name: String,
    /// Description of this capability.
    pub description: String,
    /// Keywords that match this capability.
    pub keywords: Vec<String>,
    /// Examples of tasks this capability handles.
    pub examples: Vec<String>,
}

// ── Orchestration Event ──

/// Events emitted during orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum OrchestrationEvent {
    /// A task was delegated to an agent.
    TaskDelegated {
        task_id: TaskId,
        from_agent: String,
        to_agent: String,
        prompt: String,
    },
    /// An agent started working on a task.
    TaskStarted {
        task_id: TaskId,
        agent: String,
    },
    /// An agent completed a task.
    TaskCompleted {
        task_id: TaskId,
        agent: String,
        success: bool,
        output_preview: String,
    },
    /// Multiple agents started working in parallel.
    ParallelExecution {
        tasks: Vec<ParallelTaskInfo>,
    },
    /// Results were aggregated.
    ResultsAggregated {
        task_count: usize,
        strategy: AggregationStrategy,
    },
}

/// Information about a parallel task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTaskInfo {
    pub task_id: TaskId,
    pub agent: String,
    pub task: String,
}

// ── Tool Definitions for LLM ──

/// Tool definition for the delegate_task tool.
pub fn delegate_task_tool_definition() -> serde_json::Value {
    serde_json::json!({
        "name": "delegate_task",
        "description": "Delegate a task to another agent. Use this to distribute work to specialized agents like frontend, backend, or architect.",
        "input_schema": {
            "type": "object",
            "properties": {
                "agent": {
                    "type": "string",
                    "description": "Name of the target agent (frontend, backend, architect, etc.)",
                    "enum": ["frontend", "backend", "architect", "coordinator"]
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
                },
                "wait_for_result": {
                    "type": "boolean",
                    "description": "Whether to wait for the result before continuing (default: true)"
                }
            },
            "required": ["agent", "prompt"]
        }
    })
}

/// Tool definition for the aggregate_results tool.
pub fn aggregate_results_tool_definition() -> serde_json::Value {
    serde_json::json!({
        "name": "aggregate_results",
        "description": "Collect and combine results from multiple delegated tasks.",
        "input_schema": {
            "type": "object",
            "properties": {
                "strategy": {
                    "type": "string",
                    "description": "How to combine results",
                    "enum": ["concat", "first_success", "all_success", "consensus"]
                }
            },
            "required": ["strategy"]
        }
    })
}

/// Tool definition for the report_to_coordinator tool.
pub fn report_to_coordinator_tool_definition() -> serde_json::Value {
    serde_json::json!({
        "name": "report_to_coordinator",
        "description": "Report task completion back to the coordinator agent.",
        "input_schema": {
            "type": "object",
            "properties": {
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
                },
                "artifacts": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Files or resources created during the task"
                }
            },
            "required": ["success", "output"]
        }
    })
}
