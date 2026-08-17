//! Contract tests for deterministic and unavailable key-value state providers.

use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

use std::sync::Arc;

use crate::{
    KeyValueStateProviderFactory, KeyValueStateService, MockKeyValueStateProvider,
    UnavailableKeyValueStateProvider,
};

fn command(name: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({"raw_value":"private-marker"}),
        TraceContext::new(format!("trace-{name}")),
    )
}

#[tokio::test]
async fn mock_provider_replays_all_declared_commands_without_raw_values() {
    let provider = MockKeyValueStateProvider::default();
    for operation in macaca_proto::FOUNDATION_KEY_VALUE_STATE_COMMANDS {
        let reply = provider.call(command(operation)).await.unwrap();
        assert_eq!(
            reply.metadata.get("replay.key_value_state_command"),
            Some(&operation.to_string())
        );
        assert!(!serde_json::to_string(&reply.metadata)
            .unwrap()
            .contains("private-marker"));
    }
    assert!(provider.provider_capabilities().supports_watch);
    assert_eq!(provider.snapshot().provider_class, "mock");
}

#[tokio::test]
async fn unavailable_provider_returns_structured_traceable_diagnostics() {
    let provider = UnavailableKeyValueStateProvider::default();
    let reply = provider.call(command("kv.get")).await.unwrap();
    assert_eq!(reply.status, "unavailable");
    assert_eq!(
        reply.metadata.get("key_value_state.audit_event"),
        Some(&"key_value_state_pack_unavailable".into())
    );
    assert_eq!(provider.snapshot().provider_class, "unavailable");
}

struct MockFactory;

impl KeyValueStateProviderFactory for MockFactory {
    fn provider_class(&self) -> &str {
        "mock"
    }
    fn create(&self) -> Arc<dyn KeyValueStateService> {
        Arc::new(MockKeyValueStateProvider::default())
    }
}

#[test]
fn provider_factory_keeps_adapter_selection_outside_sdk_contracts() {
    let factory = MockFactory;
    assert_eq!(factory.provider_class(), "mock");
    assert_eq!(
        factory.create().descriptor().metadata.get("provider_class"),
        Some(&"mock".into())
    );
}
