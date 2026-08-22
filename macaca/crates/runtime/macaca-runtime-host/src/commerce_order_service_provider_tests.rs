use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::commerce_order::COMMERCE_ORDER_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, TraceContext};
use serde_json::json;

use super::commerce_order_service_provider::{
    CommerceOrderRuntimeEventKind, CommerceOrderSystemServiceProvider,
};

fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new(format!("trace-{name}")),
    )
}

#[tokio::test]
async fn order_mock_dispatches_and_redacts_buyer_payload() {
    let provider = CommerceOrderSystemServiceProvider::mock();
    let result = provider
        .call(command(
            "order.create_order",
            json!({"buyer_email":"secret@example.invalid","raw_provider_payload":"secret"}),
        ))
        .await
        .unwrap();
    assert!(result.output.to_string().contains("order:reference"));
    assert!(!result.output.to_string().contains("secret@example"));
    assert!(!result.output.to_string().contains("raw_provider_payload"));
}

#[tokio::test]
async fn order_policy_and_conflict_gates_precede_reference_retention() {
    let provider = CommerceOrderSystemServiceProvider::mock();
    for marker in [
        "policy_denied",
        "approval_required",
        "conflict",
        "stale_data",
    ] {
        let result = provider
            .call(command(
                "order.state_transition_request",
                json!({marker: true}),
            ))
            .await;
        assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
    }
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn order_unavailable_covers_every_command_without_fake_success() {
    let provider = CommerceOrderSystemServiceProvider::unavailable("provider_not_installed");
    for name in COMMERCE_ORDER_COMMANDS {
        assert!(matches!(
            provider.call(command(name, json!({}))).await,
            Err(ServiceError::ServiceUnavailable(_))
        ));
    }
}

#[tokio::test]
async fn order_trace_and_snapshot_are_replay_safe() {
    let provider =
        CommerceOrderSystemServiceProvider::mock_with_commands(["order.inspect_provider"]);
    let mut events = provider.subscribe();
    let result = provider
        .call(command("order.inspect_provider", json!({})))
        .await
        .unwrap();
    assert!(result.output.to_string().contains("version_token_hash"));
    let snapshot = provider.snapshot().await;
    assert!(snapshot["redaction_profile"].contains("references"));
    let event = events.recv().await.unwrap();
    assert_ne!(
        event.kind,
        CommerceOrderRuntimeEventKind::ProviderCallFailed
    );
}
