use std::collections::{BTreeMap, BTreeSet};

use super::*;

// Key-value state tests validate the provider-neutral contract surface. They do
// not construct stores, remote clients, watchers, snapshots, or migrations.

#[test]
fn foundation_key_value_state_descriptor_is_discoverable_and_not_callable() {
    let definition = foundation_key_value_state_pack_definition();

    assert_eq!(definition.pack_id, FOUNDATION_KEY_VALUE_STATE_PACK_ID);
    assert!(!definition.is_callable());
    assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
    assert_eq!(
        definition.metadata.parent_pack_id.as_deref(),
        Some("pack.foundation.v1")
    );
    assert_eq!(
        definition.metadata.diagnostics.unavailable_reason,
        "key_value_state_provider_not_installed"
    );
    assert!(definition
        .metadata
        .sdk
        .docs_url
        .contains("developer-packs/foundation/key-value-state"));

    let commands = definition
        .metadata
        .service_command_schemas
        .get(FOUNDATION_KEY_VALUE_STATE_SERVICE_ID)
        .expect("key-value state descriptor exposes command schemas");
    for command in FOUNDATION_KEY_VALUE_STATE_COMMANDS {
        assert!(commands.contains(*command), "missing command {command}");
    }

    for scope in [
        "state.read",
        "state.write",
        "state.delete",
        "state.list",
        "state.watch",
        "state.ttl",
        "state.counter",
        "state.snapshot",
        "state.restore",
        "state.migrate",
        "state.compact",
    ] {
        assert!(
            definition.metadata.permission_scopes.contains(scope),
            "missing permission scope {scope}"
        );
    }

    for provider_class in [
        "embedded-durable",
        "remote-kv",
        "lease-consensus",
        "mock",
        "unavailable",
    ] {
        assert!(
            definition
                .metadata
                .provider_descriptors
                .contains_key(provider_class),
            "missing provider descriptor {provider_class}"
        );
    }
}

#[test]
fn industrial_catalog_uses_foundation_key_value_state_contract_descriptor() {
    let definition = industrial_reference_domain_pack_definitions()
        .into_iter()
        .find(|definition| definition.pack_id == FOUNDATION_KEY_VALUE_STATE_PACK_ID)
        .expect("industrial catalog includes foundation key-value state");

    assert!(!definition.is_callable());
    assert!(definition
        .metadata
        .service_command_schemas
        .contains_key(FOUNDATION_KEY_VALUE_STATE_SERVICE_ID));
    assert_eq!(
        definition
            .metadata
            .provider_descriptors
            .get("embedded-durable")
            .and_then(|descriptor| descriptor.metadata.get("compaction"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn foundation_key_value_state_command_dtos_are_serde_compatible() {
    let namespace = KeyValueNamespaceRef {
        namespace: "preferences".into(),
        tenant_ref: Some("tenant-ref".into()),
    };
    let key = KeyValueKeyRef {
        namespace: namespace.clone(),
        key: "ui.theme".into(),
    };
    let value = KeyValueTypedValueRef {
        value_ref: "artifact:bounded-state-value".into(),
        value_kind: "json".into(),
        schema_id: Some("preference.schema.v1".into()),
        secret_reference_required: false,
    };
    let ttl = KeyValueTtlPolicy {
        ttl_seconds: Some(3_600),
        expire_at_epoch_millis: None,
    };
    let revision = KeyValueRevision {
        revision_id: "rev-1".into(),
        generation: 1,
    };
    let snapshot = KeyValueSnapshotRef {
        snapshot_id: "snapshot-1".into(),
        namespace: namespace.clone(),
        state_hash: "state-hash".into(),
    };
    let put = KeyValuePutCommand {
        key: key.clone(),
        value: value.clone(),
        ttl: Some(ttl.clone()),
        conflict_mode: KeyValueConflictMode::CompareRevision,
    };

    let commands = vec![
        serde_json::to_value(KeyValueGetCommand {
            key: key.clone(),
            consistency: KeyValueConsistencyLevel::Session,
        })
        .unwrap(),
        serde_json::to_value(put.clone()).unwrap(),
        serde_json::to_value(KeyValueDeleteCommand {
            key: key.clone(),
            expected_revision: Some(revision.clone()),
        })
        .unwrap(),
        serde_json::to_value(KeyValueExistsCommand { key: key.clone() }).unwrap(),
        serde_json::to_value(KeyValueBatchGetCommand {
            keys: vec![key.clone()],
            consistency: KeyValueConsistencyLevel::Strong,
        })
        .unwrap(),
        serde_json::to_value(KeyValueBatchPutCommand { entries: vec![put] }).unwrap(),
        serde_json::to_value(KeyValueBatchDeleteCommand {
            keys: vec![key.clone()],
            expected_revision: Some(revision.clone()),
        })
        .unwrap(),
        serde_json::to_value(KeyValueListKeysCommand {
            namespace: namespace.clone(),
            prefix: Some("ui.".into()),
            page_size: 100,
            cursor: None,
        })
        .unwrap(),
        serde_json::to_value(KeyValueCompareAndSetCommand {
            key: key.clone(),
            expected_revision: revision.clone(),
            value,
        })
        .unwrap(),
        serde_json::to_value(KeyValueIncrementCommand {
            key: key.clone(),
            delta: 1,
            initialize: true,
        })
        .unwrap(),
        serde_json::to_value(KeyValueSetTtlCommand {
            key: key.clone(),
            ttl,
        })
        .unwrap(),
        serde_json::to_value(KeyValueGetTtlCommand { key: key.clone() }).unwrap(),
        serde_json::to_value(KeyValueWatchNamespaceCommand {
            namespace: namespace.clone(),
            prefix: Some("ui.".into()),
            start_revision: Some(revision.clone()),
        })
        .unwrap(),
        serde_json::to_value(KeyValueSnapshotNamespaceCommand {
            namespace: namespace.clone(),
            include_prefix: Some("ui.".into()),
        })
        .unwrap(),
        serde_json::to_value(KeyValueRestoreNamespaceCommand {
            snapshot,
            conflict_mode: KeyValueConflictMode::Fail,
            dry_run: true,
        })
        .unwrap(),
        serde_json::to_value(KeyValueMigrateNamespaceCommand {
            source: namespace.clone(),
            target: namespace.clone(),
            dry_run: true,
        })
        .unwrap(),
        serde_json::to_value(KeyValueCompactNamespaceCommand {
            namespace,
            before_revision: revision,
            dry_run: true,
        })
        .unwrap(),
    ];

    assert_eq!(commands.len(), FOUNDATION_KEY_VALUE_STATE_COMMANDS.len());
    assert!(commands.iter().all(|value| value.is_object()));
}

#[test]
fn foundation_key_value_state_hashes_change_with_contract_content() {
    let request = KeyValueListKeysCommand {
        namespace: KeyValueNamespaceRef {
            namespace: "preferences".into(),
            tenant_ref: None,
        },
        prefix: Some("ui.".into()),
        page_size: 100,
        cursor: None,
    };
    let mut changed = request.clone();
    changed.page_size = 200;

    assert_eq!(
        key_value_state_stable_hash(&request),
        key_value_state_stable_hash(&request)
    );
    assert_ne!(
        key_value_state_stable_hash(&request),
        key_value_state_stable_hash(&changed)
    );

    let hashes = foundation_key_value_state_descriptor_hashes();
    let unique = BTreeSet::from([
        hashes.command_schema_hash,
        hashes.result_schema_hash,
        hashes.snapshot_schema_hash,
        hashes.provider_capability_schema_hash,
        hashes.unavailable_schema_hash,
    ]);
    assert_eq!(unique.len(), 5);
    assert!(unique.iter().all(|hash| !hash.is_empty()));
}

#[test]
fn foundation_key_value_state_result_and_snapshot_dtos_are_bounded() {
    let snapshot = KeyValueStateProviderSnapshot {
        descriptor_hash: "descriptor-hash".into(),
        provider_class: "unavailable".into(),
        namespace_hashes: BTreeMap::from([("preferences".into(), "namespace-hash".into())]),
        active_watch_count: 0,
    };
    let unavailable: KeyValueStateResultEnvelope<KeyValueWatchEvent> =
        KeyValueStateResultEnvelope {
            status: KeyValueStateResultStatus::Unavailable,
            data: None,
            error: Some(KeyValueStateError {
                code: KeyValueStateResultStatus::Unavailable,
                message: "key-value state provider is not installed".into(),
                retryable: false,
            }),
            trace_id: "trace-kv-unavailable".into(),
            descriptor_hash: key_value_state_stable_hash(&snapshot),
        };

    let serialized = serde_json::to_string(&unavailable).unwrap();
    assert!(serialized.contains("trace-kv-unavailable"));
    assert!(!serialized.contains("raw-state-value"));
    assert!(!serialized.contains("raw-secret-value"));
    assert!(!serialized.contains("provider-native-transaction"));
}
