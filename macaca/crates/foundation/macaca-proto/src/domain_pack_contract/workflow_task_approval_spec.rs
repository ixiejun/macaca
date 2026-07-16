use serde::{Deserialize, Serialize};

use super::workflow_task::WorkflowTaskResultStatus;

/// Sensitive task operations requiring a host-issued approval reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowTaskSensitiveAction {
    ForceTransition,
    PropagateCancellation,
    AdministrativeRepair,
    HighPriorityQueue,
    AttachExternalArtifact,
}

/// Opaque approval evidence supplied by a policy or approval service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTaskApprovalEvidence {
    pub action: WorkflowTaskSensitiveAction,
    pub approval_ref: String,
    pub approved: bool,
}

/// Specification that admits sensitive task mutations only with bounded approval evidence.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkflowTaskApprovalSpec;

impl WorkflowTaskApprovalSpec {
    /// Validate a stable approval reference without retaining its confidential decision content.
    pub fn evaluate(&self, evidence: &WorkflowTaskApprovalEvidence) -> WorkflowTaskResultStatus {
        if evidence.approved
            && !evidence.approval_ref.is_empty()
            && evidence.approval_ref.len() <= 128
            && !evidence.approval_ref.chars().any(char::is_control)
        {
            WorkflowTaskResultStatus::Success
        } else {
            WorkflowTaskResultStatus::Denied
        }
    }

    /// Invoke a provider side effect only after sensitive-action approval succeeds.
    pub fn dispatch_after_approval<T>(
        &self,
        evidence: &WorkflowTaskApprovalEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, WorkflowTaskResultStatus> {
        match self.evaluate(evidence) {
            WorkflowTaskResultStatus::Success => Ok(dispatch()),
            status => Err(status),
        }
    }
}
