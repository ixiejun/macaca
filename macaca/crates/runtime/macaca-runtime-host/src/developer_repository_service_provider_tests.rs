use crate::developer_repository_service_provider::DeveloperRepositorySystemServiceProvider;
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
fn command(n: &str, p: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(n),
        p,
        TraceContext::new("repository-test"),
    )
}
#[tokio::test]
async fn mutation_requires_approval() {
    let s = DeveloperRepositorySystemServiceProvider::mock();
    let r = s
        .call(command("repository.push_request", serde_json::json!({})))
        .await;
    assert!(matches!(
        r,
        Err(macaca_proto::ServiceError::DisabledByPolicy(_))
    ));
    assert_eq!(s.snapshot().await["reference_count"], "0");
}
#[tokio::test]
async fn result_is_redacted() {
    let s = DeveloperRepositorySystemServiceProvider::mock();
    let r = s
        .call(command("repository.status", serde_json::json!({})))
        .await
        .unwrap();
    assert!(r.output.to_string().contains("redacted"));
    assert!(!r.output.to_string().contains("private_url"));
}
#[tokio::test]
async fn unavailable_is_structured() {
    let s = DeveloperRepositorySystemServiceProvider::unavailable("provider secret");
    assert!(matches!(
        s.call(command(
            "repository.inspect_provider",
            serde_json::json!({})
        ))
        .await,
        Err(macaca_proto::ServiceError::ServiceUnavailable(_))
    ));
}
