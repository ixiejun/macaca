use serde::{Deserialize, Serialize};

use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};
use super::workflow_task::{workflow_task_pack_definition, WorkflowTaskResultStatus};
use super::workflow_task_transition::{
    WorkflowTaskTransitionDecision, WorkflowTaskTransitionRequest, WorkflowTaskTransitionSpec,
};

/// Sanitized reason returned when a task command cannot cross the provider boundary.
///
/// The gate keeps only an outcome category and stable reason code. It deliberately
/// excludes task inputs, credentials, provider responses, and worker diagnostics
/// so callers may attach this value to trace and audit records without disclosure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTaskDispatchRejection {
    pub status: WorkflowTaskResultStatus,
    pub reason_code: String,
}

/// Provider-neutral command gate for workflow task side effects.
///
/// This is a Specification/Decorator boundary: descriptor admission validates
/// service commands, scopes, policy, entitlement, approval, and reservation;
/// the transition specification validates task state. A concrete provider is
/// represented only by the final closure, which cannot be invoked on rejection.
#[derive(Debug, Clone)]
pub struct WorkflowTaskDispatchGate {
    admission: DomainPackCommandPreflightSpec,
    transition: WorkflowTaskTransitionSpec,
}

impl WorkflowTaskDispatchGate {
    /// Build the gate from the descriptor, keeping command and scope ownership declarative.
    pub fn new(approval_required_commands: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            admission: DomainPackCommandPreflightSpec::from_definition(
                &workflow_task_pack_definition(),
                approval_required_commands,
            ),
            transition: WorkflowTaskTransitionSpec,
        }
    }

    /// Validate admission and transition facts before a provider can allocate or mutate state.
    pub fn evaluate(
        &self,
        preflight: &DomainPackCommandPreflight,
        transition: &WorkflowTaskTransitionRequest,
    ) -> Result<(), WorkflowTaskDispatchRejection> {
        self.admission
            .evaluate(preflight)
            .map_err(map_admission_rejection)?;
        let decision = self.transition.evaluate(transition);
        if decision.allowed {
            Ok(())
        } else {
            Err(map_transition_rejection(decision))
        }
    }

    /// Invoke the provider closure only after all generic and task-specific gates accept.
    pub fn dispatch_after_validation<T>(
        &self,
        preflight: &DomainPackCommandPreflight,
        transition: &WorkflowTaskTransitionRequest,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, WorkflowTaskDispatchRejection> {
        self.evaluate(preflight, transition)?;
        Ok(dispatch())
    }
}

fn map_admission_rejection(
    rejection: DomainPackPreflightRejection,
) -> WorkflowTaskDispatchRejection {
    let status = match rejection.status {
        DomainPackPreflightStatus::Denied => WorkflowTaskResultStatus::Denied,
        DomainPackPreflightStatus::Unavailable => WorkflowTaskResultStatus::Unavailable,
        DomainPackPreflightStatus::Unsupported => WorkflowTaskResultStatus::Unsupported,
        DomainPackPreflightStatus::Conflict => WorkflowTaskResultStatus::Conflict,
        DomainPackPreflightStatus::QuotaExceeded => WorkflowTaskResultStatus::QuotaExceeded,
    };
    WorkflowTaskDispatchRejection {
        status,
        reason_code: rejection.reason_code,
    }
}

fn map_transition_rejection(
    decision: WorkflowTaskTransitionDecision,
) -> WorkflowTaskDispatchRejection {
    WorkflowTaskDispatchRejection {
        status: decision.status,
        reason_code: "workflow_task_transition_rejected".into(),
    }
}
