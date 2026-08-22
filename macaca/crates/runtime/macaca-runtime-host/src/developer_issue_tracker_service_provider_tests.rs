use crate::developer_issue_tracker_service_provider::DeveloperIssueTrackerSystemServiceProvider;
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new("issue-tracker-test"),
    )
}
#[tokio::test]
async fn request_requires_approval_without_retaining_reference() {
    let service = DeveloperIssueTrackerSystemServiceProvider::mock();
    let result = service
        .call(command(
            "issue_tracker.create_issue_request",
            serde_json::json!({}),
        ))
        .await;
    assert!(matches!(
        result,
        Err(macaca_proto::ServiceError::DisabledByPolicy(_))
    ));
    assert_eq!(service.snapshot().await["reference_count"], "0");
}
#[tokio::test]
async fn result_is_redacted_issue_reference() {
    let service = DeveloperIssueTrackerSystemServiceProvider::mock();
    let result = service
        .call(command("issue_tracker.get_issue", serde_json::json!({})))
        .await
        .unwrap();
    assert!(result.output.to_string().contains("redacted"));
    assert!(!result.output.to_string().contains("private_comment"));
}
#[tokio::test]
async fn unavailable_provider_returns_structured_error() {
    let service = DeveloperIssueTrackerSystemServiceProvider::unavailable("provider secret value");
    assert!(matches!(
        service
            .call(command(
                "issue_tracker.inspect_provider",
                serde_json::json!({})
            ))
            .await,
        Err(macaca_proto::ServiceError::ServiceUnavailable(_))
    ));
}
