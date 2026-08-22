use crate::developer_terminal_service_provider::DeveloperTerminalSystemServiceProvider;
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
fn command(n: &str, p: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(n),
        p,
        TraceContext::new("terminal-test"),
    )
}
#[tokio::test]
async fn spawn_requires_approval() {
    let s = DeveloperTerminalSystemServiceProvider::mock();
    let r = s
        .call(command("terminal.spawn_request", serde_json::json!({})))
        .await;
    assert!(matches!(
        r,
        Err(macaca_proto::ServiceError::DisabledByPolicy(_))
    ));
    assert_eq!(s.snapshot().await["reference_count"], "0");
}
#[tokio::test]
async fn result_is_redacted() {
    let s = DeveloperTerminalSystemServiceProvider::mock();
    let r = s
        .call(command("terminal.inspect_provider", serde_json::json!({})))
        .await
        .unwrap();
    assert!(r.output.to_string().contains("redacted"));
    assert!(!r.output.to_string().contains("secret_env"));
}
#[tokio::test]
async fn unavailable_is_structured() {
    let s = DeveloperTerminalSystemServiceProvider::unavailable("provider secret");
    assert!(matches!(
        s.call(command("terminal.inspect_provider", serde_json::json!({})))
            .await,
        Err(macaca_proto::ServiceError::ServiceUnavailable(_))
    ));
}
