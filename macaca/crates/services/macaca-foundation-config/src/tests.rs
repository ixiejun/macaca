//! Contract tests for provider-neutral foundation config providers.

use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

use crate::{ConfigService, MockConfigProvider, UnavailableConfigProvider};

fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new(format!("trace-{name}")),
    )
}

#[tokio::test]
async fn mock_provider_returns_only_opaque_references_and_hashed_snapshot_keys() {
    let provider = MockConfigProvider::default();
    provider
        .insert_reference("ui.theme", "artifact:theme-default")
        .unwrap();
    let reply = provider
        .call(command(
            "config.get",
            serde_json::json!({
                "key":{"namespace":"app","key":"ui.theme"},
                "selector":{"profile":"default"}
            }),
        ))
        .await
        .unwrap();
    assert_eq!(reply.output["value_ref"], "artifact:theme-default");
    assert!(!reply.output.to_string().contains("raw-secret"));
    let snapshot = provider.snapshot();
    assert!(snapshot
        .source_hashes
        .keys()
        .all(|key| !key.contains("ui.theme")));
    provider.shutdown().await.unwrap();
    assert!(provider.snapshot().source_hashes.is_empty());
}

#[tokio::test]
async fn unavailable_provider_fails_closed_with_trace_evidence() {
    let provider = UnavailableConfigProvider::new("source unavailable");
    let reply = provider
        .call(command("config.get", serde_json::json!({})))
        .await
        .unwrap();
    assert_eq!(reply.status, "unavailable");
    assert_eq!(reply.trace.trace_id.as_str(), "trace-config.get");
    assert_eq!(provider.snapshot().provider_class, "unavailable");
}
