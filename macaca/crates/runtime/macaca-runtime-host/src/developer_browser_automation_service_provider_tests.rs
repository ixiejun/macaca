use super::developer_browser_automation_service_provider::{
    DeveloperBrowserAutomationRuntimeEventKind, DeveloperBrowserAutomationSystemServiceProvider,
};
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::developer_browser_automation::DEVELOPER_BROWSER_AUTOMATION_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, TraceContext};
use serde_json::json;

fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new(format!("trace-{name}")),
    )
}

#[tokio::test]
async fn browser_provider_redacts_dom_network_and_cookie_payloads() {
    let provider = DeveloperBrowserAutomationSystemServiceProvider::mock();
    let result = provider
        .call(command(
            "browser.inspect_dom",
            json!({"dom":"private","cookies":"secret","network_body":"secret"}),
        ))
        .await
        .unwrap();
    assert!(result.output.to_string().contains("browser:handle"));
    assert!(!result.output.to_string().contains("private"));
    assert!(!result.output.to_string().contains("secret"));
}
#[tokio::test]
async fn browser_policy_and_stale_handle_gates_precede_retention() {
    let provider = DeveloperBrowserAutomationSystemServiceProvider::mock();
    for payload in [
        json!({"origin_denied":true}),
        json!({"script_denied":true}),
        json!({"stale_handle":true}),
    ] {
        let result = provider
            .call(command("browser.action_request", payload))
            .await;
        assert!(matches!(
            result,
            Err(ServiceError::DisabledByPolicy(_)) | Err(ServiceError::InvalidArgument(_))
        ));
    }
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}
#[tokio::test]
async fn browser_unavailable_covers_all_commands() {
    let provider =
        DeveloperBrowserAutomationSystemServiceProvider::unavailable("provider_not_installed");
    for name in DEVELOPER_BROWSER_AUTOMATION_COMMANDS {
        assert!(matches!(
            provider.call(command(name, json!({}))).await,
            Err(ServiceError::ServiceUnavailable(_))
        ));
    }
}
#[tokio::test]
async fn browser_replay_snapshot_is_handle_only() {
    let provider = DeveloperBrowserAutomationSystemServiceProvider::mock_with_commands([
        "browser.inspect_provider",
    ]);
    let mut events = provider.subscribe();
    let result = provider
        .call(command("browser.inspect_provider", json!({})))
        .await
        .unwrap();
    assert!(result.output.to_string().contains("raw_dom"));
    assert!(provider.snapshot().await["redaction_profile"].contains("handles"));
    assert_ne!(
        events.recv().await.unwrap().kind,
        DeveloperBrowserAutomationRuntimeEventKind::ProviderCallFailed
    );
}
