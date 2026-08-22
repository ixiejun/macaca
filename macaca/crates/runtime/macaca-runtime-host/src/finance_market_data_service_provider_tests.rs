use crate::finance_market_data_service_provider::FinanceMarketDataSystemServiceProvider;
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
fn command(n: &str, p: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(n),
        p,
        TraceContext::new("market-data-test"),
    )
}
#[tokio::test]
async fn denied_request_does_not_retain() {
    let s = FinanceMarketDataSystemServiceProvider::mock();
    let r = s
        .call(command(
            "market_data.get_bars",
            serde_json::json!({"range_too_large":true}),
        ))
        .await;
    assert!(matches!(
        r,
        Err(macaca_proto::ServiceError::DisabledByPolicy(_))
    ));
    assert_eq!(s.snapshot().await["reference_count"], "0");
}
#[tokio::test]
async fn result_has_freshness_attribution() {
    let s = FinanceMarketDataSystemServiceProvider::mock();
    let r = s
        .call(command("market_data.get_quote", serde_json::json!({})))
        .await
        .unwrap();
    let o = r.output.to_string();
    assert!(o.contains("freshness") && o.contains("attribution") && o.contains("redacted"));
}
#[tokio::test]
async fn unavailable_is_structured() {
    let s = FinanceMarketDataSystemServiceProvider::unavailable("provider secret");
    assert!(matches!(
        s.call(command(
            "market_data.inspect_provider",
            serde_json::json!({})
        ))
        .await,
        Err(macaca_proto::ServiceError::ServiceUnavailable(_))
    ));
}
