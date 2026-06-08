//! Execution strategy boundary (Strategy pattern).
//!
//! The runtime coordinates commands and audit events; decomposition, goal evaluation,
//! and coordinator resume semantics are delegated to a replaceable strategy implementation.

use async_trait::async_trait;
use macaca_proto::{TodoGoal, TodoItem, TraceContext};

use crate::commands::ResumeCoordinatorCommand;
use crate::events::TaskServiceEvent;
use crate::todo_board::TaskSpace;

/// Strategy boundary for execution hooks used by the Task Service runtime.
///
/// The runtime does not own planner/reviewer/worker semantics forever. It
/// coordinates commands, state transitions, and audit events, while execution
/// hooks can later be replaced by ServiceRuntime-backed strategies.
#[async_trait]
pub trait TaskServiceExecutionStrategy: Send + Sync {
    /// Handle decomposition after a goal is created.
    async fn decompose_goal(
        &self,
        _goal: &TodoGoal,
        _space: &TaskSpace,
        _trace: Option<TraceContext>,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Handle goal evaluation when all tasks are complete.
    async fn evaluate_goal(
        &self,
        _goal: &TodoGoal,
        _tasks: &[TodoItem],
        _trace: Option<TraceContext>,
    ) -> Result<Option<TaskServiceEvent>, String> {
        Ok(None)
    }

    /// Handle resume signaling when the coordinator should continue.
    async fn resume_coordinator(&self, _command: &ResumeCoordinatorCommand) -> Result<(), String> {
        Ok(())
    }
}

/// Default no-op execution strategy used by the initial runtime skeleton.
///
/// The default strategy emits audit events and updates task state, but it does
/// not attempt to replace the existing Web planner/worker pipeline yet. That
/// makes the first implementation additive and easy to validate.
#[derive(Debug, Default, Clone)]
pub struct NoopTaskServiceExecutionStrategy;

#[async_trait]
impl TaskServiceExecutionStrategy for NoopTaskServiceExecutionStrategy {}
