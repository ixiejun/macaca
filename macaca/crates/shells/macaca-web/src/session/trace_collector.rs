//! In-memory agent trace collector used during SSE streaming (Observer pattern).
//!
//! `AgentTraceCollector` accumulates delegated-agent traces keyed by agent id until a
//! terminal session snapshot persists the aggregated trace map with the assistant turn.

use std::sync::Arc;

use tokio::sync::RwLock;

use super::trace_mapping::trace_step_from_agent_event;
use super::types::AgentTrace;

/// Collects agent traces during SSE stream processing.
/// Shared between SSE stream and session saving.
pub(crate) struct AgentTraceCollector {
    traces: RwLock<std::collections::HashMap<String, Vec<AgentTrace>>>,
    /// Maps task_id to agent name for looking up agent when TaskCompleted/TaskFailed is received
    task_to_agent: RwLock<std::collections::HashMap<String, String>>,
}

impl AgentTraceCollector {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            traces: RwLock::new(std::collections::HashMap::new()),
            task_to_agent: RwLock::new(std::collections::HashMap::new()),
        })
    }

    /// Called when executor emits TaskStarted - creates new trace
    pub async fn on_task_started(&self, task_id: &str, agent: &str) {
        tracing::debug!(task_id = %task_id, agent = %agent, "AgentTraceCollector: on_task_started called");
        let mut traces = self.traces.write().await;
        let mut task_to_agent = self.task_to_agent.write().await;

        // Store task_id -> agent mapping
        task_to_agent.insert(task_id.to_string(), agent.to_string());

        let agent_traces = traces.entry(agent.to_string()).or_insert_with(Vec::new);
        agent_traces.push(AgentTrace {
            task_id: task_id.to_string(),
            agent: agent.to_string(),
            status: "running".to_string(),
            steps: Vec::new(),
            output: None,
            error: None,
        });
        tracing::debug!(task_id = %task_id, agent = %agent, trace_count = %agent_traces.len(), "AgentTraceCollector: trace created");
    }

    /// Called when executor emits AgentEvent - adds step to trace
    pub async fn on_agent_event(
        &self,
        task_id: &str,
        agent: &str,
        event: &macaca_proto::AgentExecutionEvent,
    ) {
        tracing::debug!(task_id = %task_id, agent = %agent, event_type = ?std::mem::discriminant(event), "AgentTraceCollector: on_agent_event called");
        let mut traces = self.traces.write().await;
        if let Some(agent_traces) = traces.get_mut(agent) {
            if let Some(trace) = agent_traces.iter_mut().find(|t| t.task_id == task_id) {
                tracing::debug!(task_id = %task_id, agent = %agent, "AgentTraceCollector: found trace, adding step");
                let step = trace_step_from_agent_event(event);
                trace.steps.push(step);
            }
        }
    }

    /// Called when executor emits TaskCompleted/TaskFailed - update trace status
    /// Note: TaskCompleted/TaskFailed don't have agent field, so we look it up from task_to_agent mapping
    pub async fn on_task_completed(
        &self,
        task_id: &str,
        success: bool,
        output: Option<String>,
        error: Option<String>,
    ) {
        let agent = {
            let task_to_agent = self.task_to_agent.read().await;
            task_to_agent.get(task_id).cloned()
        };

        if let Some(agent) = agent {
            let mut traces = self.traces.write().await;
            if let Some(agent_traces) = traces.get_mut(&agent) {
                if let Some(trace) = agent_traces.iter_mut().find(|t| t.task_id == task_id) {
                    trace.status = if success {
                        "completed".to_string()
                    } else {
                        "error".to_string()
                    };
                    trace.output = output;
                    trace.error = error;
                }
            }
        }
    }

    /// Get all collected traces for session storage
    pub async fn get_all(&self) -> std::collections::HashMap<String, Vec<AgentTrace>> {
        self.traces.read().await.clone()
    }
}
