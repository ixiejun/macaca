use super::super::workflow_task::WorkflowTaskResultStatus;
use super::super::workflow_task_resource_spec::{
    WorkflowTaskResourceReservation, WorkflowTaskResourceSpec, WORKFLOW_TASK_RESOURCE_KEYS,
};
use std::collections::BTreeMap;

#[test]
fn task_resource_spec_requires_each_named_reservation() {
    let required = BTreeMap::from_iter(
        WORKFLOW_TASK_RESOURCE_KEYS
            .iter()
            .map(|key| ((*key).into(), 1)),
    );
    let accepted = WorkflowTaskResourceReservation {
        required: required.clone(),
        reserved: required.clone(),
    };
    assert_eq!(
        WorkflowTaskResourceSpec.evaluate(&accepted),
        WorkflowTaskResultStatus::Success
    );
    for key in WORKFLOW_TASK_RESOURCE_KEYS {
        let mut reserved = required.clone();
        reserved.insert((*key).into(), 0);
        assert_eq!(
            WorkflowTaskResourceSpec.evaluate(&WorkflowTaskResourceReservation {
                required: required.clone(),
                reserved
            }),
            WorkflowTaskResultStatus::QuotaExceeded
        );
    }
}
