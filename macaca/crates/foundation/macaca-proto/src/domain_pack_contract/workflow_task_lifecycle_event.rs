use serde::{Deserialize, Serialize};

use super::workflow_task::WorkflowTaskState;

/// Stable task lifecycle operations emitted by conforming task service providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowTaskLifecycleEventKind {
    PackDeclared,
    AdmissionValidated,
    Created,
    Enqueued,
    Claimed,
    HeartbeatRecorded,
    ProgressRecorded,
    CheckpointRecorded,
    ArtifactAttached,
    Completed,
    Failed,
    RetryScheduled,
    Cancelled,
    Skipped,
    LeaseRevoked,
    SnapshotRecorded,
}

/// Sanitized event envelope suitable for trace, audit, and deterministic replay indexes.
///
/// All fields are stable references, hashes, counters, or bounded enum values. Providers
/// must keep raw prompts, inputs, artifact contents, credentials, worker diagnostics,
/// provider payloads, and unbounded history outside this envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTaskLifecycleEvent {
    pub kind: WorkflowTaskLifecycleEventKind,
    pub trace_id: String,
    pub task_ref: String,
    pub state: WorkflowTaskState,
    pub version_ref: String,
    pub queue_ref_hash: String,
    pub attempt_index: u32,
    pub replay_ref: String,
}

impl WorkflowTaskLifecycleEvent {
    /// Verify reference-only bounds before providers publish an event to observers.
    pub fn is_trace_safe(&self) -> bool {
        [
            self.trace_id.as_str(),
            self.task_ref.as_str(),
            self.version_ref.as_str(),
            self.queue_ref_hash.as_str(),
            self.replay_ref.as_str(),
        ]
        .into_iter()
        .all(bounded_reference)
    }
}

fn bounded_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.chars().any(char::is_control)
        && !value.contains("raw_")
        && !value.contains("secret")
}
