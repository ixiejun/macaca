use crate::developer_design_tools_service_provider::DeveloperDesignToolsSystemServiceProvider;
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new("design-tools-test"),
    )
}

#[tokio::test]
async fn request_requires_approval_and_does_not_retain_reference() {
    let service = DeveloperDesignToolsSystemServiceProvider::mock();
    let result = service
        .call(command(
            "design_tools.token_sync_request",
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
async fn result_is_redacted_reference_metadata() {
    let service = DeveloperDesignToolsSystemServiceProvider::mock();
    let result = service
        .call(command("design_tools.inspect_node", serde_json::json!({})))
        .await
        .unwrap();
    let output = result.output.to_string();
    assert!(output.contains("redacted"));
    assert!(!output.contains("private_comment"));
}

#[tokio::test]
async fn unavailable_provider_returns_structured_error() {
    let service = DeveloperDesignToolsSystemServiceProvider::unavailable("provider secret value");
    assert!(matches!(
        service
            .call(command(
                "design_tools.inspect_provider",
                serde_json::json!({})
            ))
            .await,
        Err(macaca_proto::ServiceError::ServiceUnavailable(_))
    ));
}
