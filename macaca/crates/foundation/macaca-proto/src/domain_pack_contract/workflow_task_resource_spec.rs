use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::workflow_task::WorkflowTaskResultStatus;

/// Named task-service resources whose reservations prevent unbounded provider state.
pub const WORKFLOW_TASK_RESOURCE_KEYS: &[&str] = &[
    "active_tasks",
    "queued_tasks",
    "active_leases",
    "attempts",
    "retries",
    "checkpoints",
    "artifacts",
    "history_entries",
    "retained_snapshots",
    "replay_metadata",
];

/// Reference-only demand and reservation counters supplied by a host-owned meter.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTaskResourceReservation {
    pub required: BTreeMap<String, u64>,
    pub reserved: BTreeMap<String, u64>,
}

/// Specification that rejects incomplete task resource reservations before dispatch.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkflowTaskResourceSpec;

impl WorkflowTaskResourceSpec {
    /// Require every known resource demand to be bounded by a matching reservation.
    pub fn evaluate(&self, value: &WorkflowTaskResourceReservation) -> WorkflowTaskResultStatus {
        for key in WORKFLOW_TASK_RESOURCE_KEYS {
            if value.required.get(*key).unwrap_or(&0) > value.reserved.get(*key).unwrap_or(&0) {
                return WorkflowTaskResultStatus::QuotaExceeded;
            }
        }
        WorkflowTaskResultStatus::Success
    }
}
