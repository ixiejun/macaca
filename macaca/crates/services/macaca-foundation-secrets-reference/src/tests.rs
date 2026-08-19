//! Contract tests for unavailable and deterministic metadata-only providers.

use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

use crate::{
    MockSecretsReferenceProvider, SecretsReferenceAdapterBridge, SecretsReferenceProviderFactory,
    SecretsReferenceService, UnavailableSecretsReferenceProvider,
};
use std::sync::Arc;

fn command(name: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({
            "reference":{"reference_id":"secret-ref","provider_class":"mock","version_hint":"current"},
            "purpose":{"purpose":"test-purpose","service_id":"service.test","expires_at_epoch_millis":null},
            "policy":{"allowed_service_ids":["service.test"],"requires_approval":false,"max_lease_ttl_seconds":3600},
            "locator":{"provider_class":"mock","redacted_locator_hash":"locator-hash"},
            "purpose_name":"test-purpose",
            "service_id":"service.test",
            "ttl_seconds":60,
            "lease":{"lease_id":"lease:test","reference_id":"secret-ref","expires_at_epoch_millis":999999999i64},
            "raw_secret":"must-never-escape",
            "credential":"must-never-escape",
            "provider_locator":"must-never-escape"
        }),
        TraceContext::new(format!("trace-{name}")),
    )
}

#[tokio::test]
async fn mock_provider_covers_commands_without_raw_secret_results() {
    let provider = MockSecretsReferenceProvider::default();
    for operation in macaca_proto::FOUNDATION_SECRETS_REFERENCE_COMMANDS {
        let result = provider.call(command(operation)).await.unwrap();
        let serialized = serde_json::to_string(&result).unwrap();
        assert!(!serialized.contains("must-never-escape"));
        assert_eq!(
            result.metadata.get("replay.secrets_reference_command"),
            Some(&operation.to_string())
        );
        assert!(result
            .output
            .get("raw_value")
            .is_none_or(|value| value.is_null()));
    }
    assert!(
        provider
            .provider_capabilities()
            .raw_value_app_results_forbidden
    );
}

#[tokio::test]
async fn unavailable_provider_is_traceable_and_fail_closed() {
    let provider = UnavailableSecretsReferenceProvider::default();
    let result = provider
        .call(command("secrets.resolve_for_provider"))
        .await
        .unwrap();
    assert_eq!(result.status, "unavailable");
    assert_eq!(result.trace.trace_id, "trace-secrets.resolve_for_provider");
    assert!(result.output["reason"]
        .as_str()
        .unwrap()
        .contains("not installed"));
    assert_eq!(provider.snapshot().provider_class, "unavailable");
}

struct Factory;
impl SecretsReferenceProviderFactory for Factory {
    fn provider_class(&self) -> &str {
        "test-adapter"
    }
    fn create(&self) -> Arc<dyn SecretsReferenceService> {
        Arc::new(MockSecretsReferenceProvider::default())
    }
}

#[test]
fn adapter_bridge_exposes_only_abstract_factory_and_class_label() {
    let bridge = SecretsReferenceAdapterBridge::new("test-adapter", Arc::new(Factory));
    assert_eq!(bridge.provider_class(), "test-adapter");
    assert_eq!(
        bridge.create().provider_capabilities().provider_class,
        "mock"
    );
}
