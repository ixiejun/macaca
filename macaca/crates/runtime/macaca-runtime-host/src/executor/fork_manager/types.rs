//! Fork domain types — value objects and hook event taxonomy.
//!
//! Pure data structures with no I/O. [`ForkContext`] is the State object
//! carried through create/suspend/resume/merge transitions.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use macaca_proto::{
    AcceptanceCriteria, ApplicationId, ForkId, ForkState, LlmMessage, TaskId, ValidationResult,
};

/// Maximum number of concurrent forks per application.
pub(crate) const MAX_PARALLEL_FORKS: usize = 10;

/// Maximum time to wait for a delegate task to complete.
pub(crate) const DEFAULT_DELEGATE_TIMEOUT_SECS: u64 = 300;

/// Maximum number of messages to inherit from parent.
pub(crate) const MAX_INHERITED_MESSAGES: usize = 10;

/// Callback for hook events.
pub type HookCallback =
    Box<dyn Fn(HookEvent) -> futures::future::BoxFuture<'static, ()> + Send + Sync>;

/// Event emitted when a fork's state changes.
#[derive(Debug, Clone)]
pub enum HookEvent {
    /// Fork created.
    ForkCreated {
        fork_id: ForkId,
        application_id: ApplicationId,
        agent_name: String,
    },
    /// Fork suspended waiting for delegate task.
    ForkWaiting {
        fork_id: ForkId,
        delegate_task_id: TaskId,
    },
    /// Fork resumed after delegate task completed.
    ForkResumed {
        fork_id: ForkId,
        delegate_result: DelegateResult,
    },
    /// Fork's delegate task completed.
    DelegateCompleted {
        fork_id: ForkId,
        task_id: TaskId,
        success: bool,
        output: String,
    },
    /// Fork's delegate task failed.
    DelegateFailed {
        fork_id: ForkId,
        task_id: TaskId,
        error: String,
    },
    /// Fork validated and ready to merge.
    ForkValidated {
        fork_id: ForkId,
        result: ValidationResult,
    },
    /// Fork merged to parent.
    ForkMerged { fork_id: ForkId },
}

/// Result from a delegated task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateResult {
    pub task_id: TaskId,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub artifacts: Vec<String>,
}

/// Context for a forked agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkContext {
    /// Unique fork identifier.
    pub id: ForkId,
    /// Parent fork ID (None for root/main agent).
    pub parent_fork_id: Option<ForkId>,
    /// Application ID this fork belongs to.
    pub application_id: ApplicationId,
    /// Name of the agent running this fork.
    pub agent_name: String,
    /// Current lifecycle state.
    pub state: ForkState,
    /// Inherited conversation history from parent (last N messages).
    pub inherited_messages: Vec<LlmMessage>,
    /// Conversation history generated during this fork's execution.
    pub own_messages: Vec<LlmMessage>,
    /// System prompt (inherited from parent).
    pub system_prompt: String,
    /// Original task prompt.
    pub task_prompt: String,
    /// Acceptance criteria for validation.
    pub acceptance_criteria: AcceptanceCriteria,
    /// Task ID this fork is waiting on (if any).
    pub waiting_on_task: Option<TaskId>,
    /// Artifacts produced by this fork.
    pub artifacts: Vec<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Completion timestamp (if completed).
    pub completed_at: Option<DateTime<Utc>>,
    /// Timeout for delegate tasks (stored as seconds for serialization).
    #[serde(with = "duration_secs")]
    pub delegate_timeout: Duration,
}

impl ForkContext {
    /// Create a new fork context.
    pub fn new(
        parent_fork_id: Option<ForkId>,
        application_id: ApplicationId,
        agent_name: String,
        task_prompt: String,
        inherited_messages: Vec<LlmMessage>,
        system_prompt: String,
        acceptance_criteria: AcceptanceCriteria,
    ) -> Self {
        // Keep only the last N messages to limit context size
        let inherited_messages = inherited_messages
            .into_iter()
            .rev()
            .take(MAX_INHERITED_MESSAGES)
            .rev()
            .collect();

        Self {
            id: ForkId::new(),
            parent_fork_id,
            application_id,
            agent_name,
            state: ForkState::Pending,
            inherited_messages,
            own_messages: vec![],
            system_prompt,
            task_prompt,
            acceptance_criteria,
            waiting_on_task: None,
            artifacts: vec![],
            created_at: Utc::now(),
            completed_at: None,
            delegate_timeout: Duration::from_secs(DEFAULT_DELEGATE_TIMEOUT_SECS),
        }
    }

    /// Check if the fork is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            ForkState::Completed
                | ForkState::Failed { .. }
                | ForkState::Merged
                | ForkState::Cancelled
        )
    }
}

/// Result from merging a fork back to parent.
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub fork_id: ForkId,
    /// Summary message to append to parent's conversation.
    pub summary_message: String,
    pub artifacts: Vec<String>,
}

/// Serde helper for serializing `Duration` as seconds (u64).
mod duration_secs {
    use serde::{Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_secs())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(d)?;
        Ok(Duration::from_secs(secs))
    }

    use serde::Deserialize;
}
