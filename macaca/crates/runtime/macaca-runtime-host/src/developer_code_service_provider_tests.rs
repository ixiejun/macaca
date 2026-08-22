use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

use crate::developer_code_service_provider::DeveloperCodeSystemServiceProvider;

fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new("code-test"),
    )
}

#[tokio::test]
async fn denied_patch_does_not_retain_reference() {
    let service = DeveloperCodeSystemServiceProvider::mock();
    let result = service
        .call(command("code.apply_patch_request", serde_json::json!({})))
        .await;
    assert!(matches!(
        result,
        Err(macaca_proto::ServiceError::DisabledByPolicy(_))
    ));
    assert_eq!(service.snapshot().await["reference_count"], "0");
}

#[tokio::test]
async fn successful_result_contains_no_source_or_patch_content() {
    let service = DeveloperCodeSystemServiceProvider::mock();
    let result = service
        .call(command("code.parse_document", serde_json::json!({})))
        .await
        .unwrap();
    let output = result.output.to_string();
    assert!(output.contains("redacted"));
    assert!(!output.contains("fn secret"));
}

#[tokio::test]
async fn unavailable_provider_is_structured_error() {
    let service = DeveloperCodeSystemServiceProvider::unavailable("provider secret value");
    assert!(matches!(
        service
            .call(command("code.inspect_provider", serde_json::json!({})))
            .await,
        Err(macaca_proto::ServiceError::ServiceUnavailable(_))
    ));
}
