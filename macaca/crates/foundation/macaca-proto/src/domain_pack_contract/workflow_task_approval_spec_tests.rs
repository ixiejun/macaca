use super::super::workflow_task::WorkflowTaskResultStatus;
use super::super::workflow_task_approval_spec::{
    WorkflowTaskApprovalEvidence, WorkflowTaskApprovalSpec, WorkflowTaskSensitiveAction,
};

#[test]
fn sensitive_task_actions_require_approval_before_provider_dispatch() {
    for action in actions() {
        let allowed = WorkflowTaskApprovalEvidence {
            action,
            approval_ref: "approval:granted".into(),
            approved: true,
        };
        assert_eq!(
            WorkflowTaskApprovalSpec.dispatch_after_approval(&allowed, || "dispatched"),
            Ok("dispatched")
        );
        let denied = WorkflowTaskApprovalEvidence {
            approved: false,
            ..allowed
        };
        let mut dispatched = false;
        assert_eq!(
            WorkflowTaskApprovalSpec.dispatch_after_approval(&denied, || dispatched = true),
            Err(WorkflowTaskResultStatus::Denied)
        );
        assert!(!dispatched);
    }
}

fn actions() -> [WorkflowTaskSensitiveAction; 5] {
    [
        WorkflowTaskSensitiveAction::ForceTransition,
        WorkflowTaskSensitiveAction::PropagateCancellation,
        WorkflowTaskSensitiveAction::AdministrativeRepair,
        WorkflowTaskSensitiveAction::HighPriorityQueue,
        WorkflowTaskSensitiveAction::AttachExternalArtifact,
    ]
}
