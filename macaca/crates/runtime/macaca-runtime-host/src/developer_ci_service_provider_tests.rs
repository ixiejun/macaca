use super::developer_ci_service_provider::DeveloperCiSystemServiceProvider;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::developer_ci::DEVELOPER_CI_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, TraceContext};
use serde_json::json;
fn command(n: &str, p: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(n),
        p,
        TraceContext::new(format!("trace-{n}")),
    )
}
#[tokio::test]
async fn ci_mock_redacts_logs_and_artifacts() {
    let p = DeveloperCiSystemServiceProvider::mock();
    let r = p
        .call(command(
            "ci.get_log",
            json!({"raw_log":"secret","artifact_bytes":"secret"}),
        ))
        .await
        .unwrap();
    assert!(r.output.to_string().contains("ci:reference"));
    assert!(!r.output.to_string().contains("secret"));
}
#[tokio::test]
async fn ci_gates_precede_reference_retention() {
    let p = DeveloperCiSystemServiceProvider::mock();
    let r = p
        .call(command(
            "ci.trigger_run_request",
            json!({"approval_required":true}),
        ))
        .await;
    assert!(matches!(r, Err(ServiceError::DisabledByPolicy(_))));
    assert_eq!(p.snapshot().await["active_reference_count"], "0");
}
#[tokio::test]
async fn ci_unavailable_covers_commands() {
    let p = DeveloperCiSystemServiceProvider::unavailable("provider_not_installed");
    for n in DEVELOPER_CI_COMMANDS {
        assert!(matches!(
            p.call(command(n, json!({}))).await,
            Err(ServiceError::ServiceUnavailable(_))
        ));
    }
}
