use crate::finance_crypto_service_provider::FinanceCryptoSystemServiceProvider;
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
fn command(n: &str, p: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(n),
        p,
        TraceContext::new("crypto-test"),
    )
}
#[tokio::test]
async fn address_requires_approval() {
    let s = FinanceCryptoSystemServiceProvider::mock();
    let r = s
        .call(command(
            "crypto.inspect_public_address_balance",
            serde_json::json!({}),
        ))
        .await;
    assert!(matches!(
        r,
        Err(macaca_proto::ServiceError::DisabledByPolicy(_))
    ));
    assert_eq!(s.snapshot().await["reference_count"], "0");
}
#[tokio::test]
async fn result_has_freshness_and_redaction() {
    let s = FinanceCryptoSystemServiceProvider::mock();
    let r = s
        .call(command("crypto.get_quote", serde_json::json!({})))
        .await
        .unwrap();
    let o = r.output.to_string();
    assert!(o.contains("freshness") && o.contains("redacted"));
    assert!(!o.contains("private_key"));
}
#[tokio::test]
async fn unavailable_is_structured() {
    let s = FinanceCryptoSystemServiceProvider::unavailable("provider secret");
    assert!(matches!(
        s.call(command("crypto.inspect_provider", serde_json::json!({})))
            .await,
        Err(macaca_proto::ServiceError::ServiceUnavailable(_))
    ));
}
