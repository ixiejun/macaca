use chrono::Utc;

use crate::{
    workbench_service_descriptors, ApplicationId, ArtifactRef, BoundedSummary, MacacaError,
    ServiceHealth, TraceContext, WorkbenchCommandResult, WorkbenchCommandScope,
    WorkbenchCommandStatus,
};

#[test]
fn descriptors_cover_every_foundation_service() {
    let descriptors = workbench_service_descriptors();

    assert_eq!(descriptors.len(), 15);
    assert!(descriptors
        .iter()
        .all(|descriptor| !descriptor.commands.is_empty()));
    assert!(descriptors
        .iter()
        .all(|descriptor| matches!(descriptor.base.health, ServiceHealth::Unavailable { .. })));
}

#[test]
fn command_result_round_trips_with_artifact_backed_status() {
    let artifact = ArtifactRef {
        id: "artifact-output".into(),
        media_type: "text/plain".into(),
        sha256: Some("abc123".into()),
        byte_len: 42,
        redaction_profile: "default".into(),
    };
    let result: WorkbenchCommandResult<serde_json::Value> = WorkbenchCommandResult {
        trace: TraceContext::new("trace-workbench-result"),
        status: WorkbenchCommandStatus::ArtifactBacked {
            artifact_ref: artifact,
        },
        output: None,
        summary: Some(BoundedSummary {
            text: "stored as artifact".into(),
            truncated: true,
            original_byte_len: Some(42),
            artifact_ref: None,
        }),
        audit_ref: Some("audit-workbench".into()),
        completed_at: Utc::now(),
    };

    let encoded = serde_json::to_string(&result).unwrap();
    let decoded: WorkbenchCommandResult<serde_json::Value> =
        serde_json::from_str(&encoded).unwrap();

    assert!(matches!(
        decoded.status,
        WorkbenchCommandStatus::ArtifactBacked { .. }
    ));
    assert!(decoded.summary.unwrap().truncated);
}

#[test]
fn command_scope_rejects_missing_session() {
    let err = WorkbenchCommandScope::new(ApplicationId::from_name("generic-test-app"), " ")
        .expect_err("empty session must be rejected before service dispatch");

    assert!(matches!(err, MacacaError::Config(_)));
}
