//! Contract tests for deterministic and unavailable key-value state providers.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_persist::PersistStore;
use macaca_proto::{
    KeyValueConflictMode, KeyValueConsistencyLevel, KeyValueGetCommand, KeyValueKeyRef,
    KeyValueNamespaceRef, KeyValuePutCommand, KeyValueTypedValueRef, MacacaResult, ServiceCommand,
    ServiceCommandName, TraceContext,
};
use tokio::sync::Mutex;

use crate::{
    EmbeddedKeyValueStateProvider, KeyValueStateProviderFactory, KeyValueStateService,
    MockKeyValueStateProvider, UnavailableKeyValueStateProvider,
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

#[derive(Default)]
struct MemoryPersistStore(Mutex<BTreeMap<String, Vec<u8>>>);

#[async_trait]
impl PersistStore for MemoryPersistStore {
    async fn get(&self, key: &str) -> MacacaResult<Option<Vec<u8>>> {
        Ok(self.0.lock().await.get(key).cloned())
    }
    async fn set(&self, key: &str, value: &[u8]) -> MacacaResult<()> {
        self.0.lock().await.insert(key.into(), value.into());
        Ok(())
    }
    async fn delete(&self, key: &str) -> MacacaResult<()> {
        self.0.lock().await.remove(key);
        Ok(())
    }
    async fn list_keys(&self, prefix: &str) -> MacacaResult<Vec<String>> {
        Ok(self
            .0
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect())
    }
}

#[tokio::test]
async fn embedded_provider_persists_opaque_references_without_leaking_them() {
    let provider = EmbeddedKeyValueStateProvider::new(Arc::new(MemoryPersistStore::default()));
    let key = KeyValueKeyRef {
        namespace: KeyValueNamespaceRef {
            namespace: "preferences".into(),
            tenant_ref: Some("tenant".into()),
        },
        key: "theme".into(),
    };
    let value = KeyValueTypedValueRef {
        value_ref: "artifact:theme".into(),
        value_kind: "json".into(),
        schema_id: None,
        secret_reference_required: false,
    };
    let put = ServiceCommand::with_trace(
        ServiceCommandName::new("kv.put"),
        serde_json::to_value(KeyValuePutCommand {
            key: key.clone(),
            value,
            ttl: None,
            conflict_mode: KeyValueConflictMode::Fail,
        })
        .unwrap(),
        TraceContext::new("trace-put"),
    );
    assert_eq!(provider.call(put).await.unwrap().status, "ok");
    let get = ServiceCommand::with_trace(
        ServiceCommandName::new("kv.get"),
        serde_json::to_value(KeyValueGetCommand {
            key,
            consistency: KeyValueConsistencyLevel::Local,
        })
        .unwrap(),
        TraceContext::new("trace-get"),
    );
    let reply = provider.call(get).await.unwrap();
    assert_eq!(reply.status, "ok");
    assert!(reply.output["value_present"].as_bool().unwrap());
    assert!(!serde_json::to_string(&reply)
        .unwrap()
        .contains("artifact:theme"));
}

#[tokio::test]
async fn embedded_provider_snapshots_restores_and_bounds_watch_slots() {
    let provider = EmbeddedKeyValueStateProvider::new(Arc::new(MemoryPersistStore::default()));
    let namespace = KeyValueNamespaceRef {
        namespace: "preferences".into(),
        tenant_ref: Some("tenant".into()),
    };
    let key = KeyValueKeyRef {
        namespace: namespace.clone(),
        key: "theme".into(),
    };
    let value = KeyValueTypedValueRef {
        value_ref: "artifact:theme".into(),
        value_kind: "json".into(),
        schema_id: None,
        secret_reference_required: false,
    };
    let trace = |name| TraceContext::new(format!("trace-{name}"));
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("kv.put"),
            serde_json::to_value(KeyValuePutCommand {
                key: key.clone(),
                value,
                ttl: None,
                conflict_mode: KeyValueConflictMode::Fail,
            })
            .unwrap(),
            trace("put"),
        ))
        .await
        .unwrap();
    let snapshot = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("kv.snapshot_namespace"),
            serde_json::json!({"namespace":namespace,"include_prefix":null}),
            trace("snapshot"),
        ))
        .await
        .unwrap();
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("kv.delete"),
            serde_json::json!({"key":key,"expected_revision":null}),
            trace("delete"),
        ))
        .await
        .unwrap();
    let snapshot_id = snapshot.output["snapshot_ref"].as_str().unwrap();
    let restore = serde_json::json!({"snapshot":{"snapshot_id":snapshot_id,"namespace":namespace,"state_hash":"redacted"},"conflict_mode":"fail","dry_run":false});
    assert_eq!(
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("kv.restore_namespace"),
                restore,
                trace("restore")
            ))
            .await
            .unwrap()
            .status,
        "ok"
    );
    for index in 0..32 {
        assert_eq!(
            provider
                .call(ServiceCommand::with_trace(
                    ServiceCommandName::new("kv.watch_namespace"),
                    serde_json::json!({"namespace":namespace,"prefix":null,"start_revision":null}),
                    TraceContext::new(format!("trace-watch-{index}"))
                ))
                .await
                .unwrap()
                .status,
            "ok"
        );
    }
    assert!(provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("kv.watch_namespace"),
            serde_json::json!({"namespace":namespace,"prefix":null,"start_revision":null}),
            trace("watch-overflow")
        ))
        .await
        .is_err());
    provider.cancel_watch("trace-watch-0").await.unwrap();
    assert_eq!(provider.snapshot().active_watch_count, 31);
}
