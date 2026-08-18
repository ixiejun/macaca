//! Tests for provider-neutral key-value SDK helper construction.

use macaca_proto::{
    compose_installed_domain_pack_catalog, AppServiceContractConfig, DomainPackAvailability,
    KeyValueConflictMode, KeyValueKeyRef, KeyValueNamespaceRef, KeyValueRevision,
    KeyValueSnapshotRef, KeyValueTypedValueRef, KeyValueWatchNamespaceCommand, TraceContext,
    FOUNDATION_KEY_VALUE_STATE_PACK_ID, FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
};

use super::*;
use crate::domain_pack_client::{EmptySystemDomainPackClient, SystemDomainPackClient};
use crate::{CatalogBackedDomainPackClient, DomainPackResolveCommand};

async fn resolved() -> DomainPackResolveResult {
    let mut definition = macaca_proto::foundation_key_value_state_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    CatalogBackedDomainPackClient::new(compose_installed_domain_pack_catalog(vec![definition]))
        .resolve_declaration(&DomainPackResolveCommand {
            declaration: AppServiceContractConfig {
                optional_packs: vec![FOUNDATION_KEY_VALUE_STATE_PACK_ID.into()],
                ..Default::default()
            },
        })
        .await
        .unwrap()
}

fn namespace() -> KeyValueNamespaceRef {
    KeyValueNamespaceRef {
        namespace: "preferences".into(),
        tenant_ref: Some("tenant-ref".into()),
    }
}

fn key() -> KeyValueKeyRef {
    KeyValueKeyRef {
        namespace: namespace(),
        key: "ui.theme".into(),
    }
}

fn value() -> KeyValueTypedValueRef {
    KeyValueTypedValueRef {
        value_ref: "artifact:theme".into(),
        value_kind: "json".into(),
        schema_id: Some("preferences.v1".into()),
        secret_reference_required: false,
    }
}

#[tokio::test]
async fn cas_plan_builds_bounded_canonical_attempts() {
    assert!(KeyValueCasUpdatePlan::new(0).is_err());
    let plan = KeyValueCasUpdatePlan::new(2).unwrap();
    let command = plan
        .build_attempt(
            key(),
            KeyValueRevision {
                revision_id: "revision-1".into(),
                generation: 1,
            },
            value(),
            TraceContext::new("trace-cas"),
        )
        .unwrap()
        .build(&resolved().await)
        .unwrap();
    assert_eq!(plan.max_attempts(), 2);
    assert_eq!(command.service_id, FOUNDATION_KEY_VALUE_STATE_SERVICE_ID);
    assert_eq!(command.command_name, "kv.compare_and_set");
}

#[tokio::test]
async fn scan_and_watch_helpers_bound_streaming_intent() {
    assert!(key_value_bounded_prefix_scan_command(
        namespace(),
        None,
        0,
        None,
        TraceContext::new("trace-invalid-scan")
    )
    .is_err());
    let scan = key_value_bounded_prefix_scan_command(
        namespace(),
        Some("ui.".into()),
        100,
        None,
        TraceContext::new("trace-scan"),
    )
    .unwrap()
    .build(&resolved().await)
    .unwrap();
    assert_eq!(scan.command_name, "kv.list_keys");

    let subscription = key_value_watch_subscription(
        KeyValueWatchNamespaceCommand {
            namespace: namespace(),
            prefix: Some("ui.".into()),
            start_revision: None,
        },
        32,
        TraceContext::new("trace-watch"),
    )
    .unwrap();
    assert_eq!(subscription.cancellation().trace_id, "trace-watch");
    let watch = subscription.build(&resolved().await).unwrap();
    assert_eq!(watch.command_name, "kv.watch_namespace");
}

#[tokio::test]
async fn ttl_and_snapshot_helpers_preserve_safe_intent() {
    assert!(
        key_value_ttl_cache_entry_command(key(), value(), 0, TraceContext::new("trace-ttl"))
            .is_err()
    );
    let ttl = key_value_ttl_cache_entry_command(key(), value(), 60, TraceContext::new("trace-ttl"))
        .unwrap()
        .build(&resolved().await)
        .unwrap();
    assert_eq!(ttl.command_name, "kv.put");

    let restore = key_value_restore_snapshot_dry_run_command(
        KeyValueSnapshotRef {
            snapshot_id: "snapshot-ref".into(),
            namespace: namespace(),
            state_hash: "state-hash".into(),
        },
        KeyValueConflictMode::Fail,
        TraceContext::new("trace-restore"),
    )
    .unwrap()
    .build(&resolved().await)
    .unwrap();
    assert_eq!(restore.command_name, "kv.restore_namespace");
    assert_eq!(restore.payload["dry_run"], true);
}

#[tokio::test]
async fn unavailable_diagnostics_are_scoped_to_the_key_value_pack() {
    let result = EmptySystemDomainPackClient
        .resolve_declaration(&DomainPackResolveCommand {
            declaration: AppServiceContractConfig {
                optional_packs: vec![FOUNDATION_KEY_VALUE_STATE_PACK_ID.into()],
                ..Default::default()
            },
        })
        .await
        .unwrap();
    let diagnostics = key_value_unavailable_diagnostics(&result);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].pack_id, FOUNDATION_KEY_VALUE_STATE_PACK_ID);
}
