use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::commerce_payment_intent::COMMERCE_PAYMENT_INTENT_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, TraceContext};
use serde_json::json;

use super::commerce_payment_intent_service_provider::{
    CommercePaymentIntentRuntimeEventKind, CommercePaymentIntentSystemServiceProvider,
};

fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new(format!("trace-{name}")),
    )
}

#[tokio::test]
async fn payment_intent_rejects_raw_credentials_before_provider_reference() {
    let provider = CommercePaymentIntentSystemServiceProvider::mock();
    let result = provider
        .call(command(
            "payment_intent.create_intent",
            json!({"card_number":"4111111111111111"}),
        ))
        .await;
    assert!(matches!(result, Err(ServiceError::InvalidArgument(_))));
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn payment_intent_confirm_returns_handle_only_action_required() {
    let provider = CommercePaymentIntentSystemServiceProvider::mock();
    let result = provider
        .call(command(
            "payment_intent.confirm",
            json!({"token_ref":"pm_ref"}),
        ))
        .await
        .unwrap();
    assert_eq!(result.output["status"], "action_required");
    assert_eq!(result.output["state"], "requires_action");
    assert!(result.output["client_secret"].is_null());
}

#[tokio::test]
async fn payment_intent_policy_gates_precede_retention() {
    let provider = CommercePaymentIntentSystemServiceProvider::mock();
    let result = provider
        .call(command(
            "payment_intent.capture",
            json!({"approval_required":true}),
        ))
        .await;
    assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn payment_intent_unavailable_and_replay_paths_are_explicit() {
    let unavailable =
        CommercePaymentIntentSystemServiceProvider::unavailable("provider_not_installed");
    for name in COMMERCE_PAYMENT_INTENT_COMMANDS {
        assert!(matches!(
            unavailable.call(command(name, json!({}))).await,
            Err(ServiceError::ServiceUnavailable(_))
        ));
    }
    let provider = CommercePaymentIntentSystemServiceProvider::mock_with_commands([
        "payment_intent.inspect_provider",
    ]);
    let mut events = provider.subscribe();
    let result = provider
        .call(command("payment_intent.inspect_provider", json!({})))
        .await
        .unwrap();
    assert!(result.output.to_string().contains("idempotency_key_hash"));
    assert!(provider.snapshot().await["redaction_profile"].contains("token_hashes"));
    assert_ne!(
        events.recv().await.unwrap().kind,
        CommercePaymentIntentRuntimeEventKind::ProviderCallFailed
    );
}
