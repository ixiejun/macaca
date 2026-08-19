//! Durable embedded Strategy backed by the provider-neutral persistence port.
//!
//! Storage names are derived from hashes of logical namespaces and keys. The
//! backing store therefore never receives raw namespace/key identifiers, while
//! values remain opaque references rather than application data or secrets.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use macaca_persist::PersistStore;
use macaca_proto::{
    CapabilityId, CleanupPolicy, DomainPackProviderCapabilityState, KernelServiceId,
    KeyValueBatchDeleteCommand, KeyValueBatchGetCommand, KeyValueBatchPutCommand,
    KeyValueCompactNamespaceCommand, KeyValueCompareAndSetCommand, KeyValueDeleteCommand,
    KeyValueExistsCommand, KeyValueGetCommand, KeyValueGetTtlCommand, KeyValueKeyRef,
    KeyValueListKeysCommand, KeyValueMigrateNamespaceCommand, KeyValuePutCommand,
    KeyValueRestoreNamespaceCommand, KeyValueRevision, KeyValueSetTtlCommand,
    KeyValueSnapshotNamespaceCommand, KeyValueTtlPolicy, KeyValueTypedValueRef,
    KeyValueWatchNamespaceCommand, ServiceCallResult, ServiceCapability, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
    FOUNDATION_KEY_VALUE_STATE_COMMANDS, FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::KeyValueStateService;

const ENTRY_PREFIX: &str = "kv.entry/";
const SNAPSHOT_PREFIX: &str = "kv.snapshot/";
const MAX_SCAN_PAGE_SIZE: u32 = 500;

#[path = "embedded_provider_commands.rs"]
mod embedded_provider_commands;

/// Durable, namespace-sandboxed provider composed by a runtime host.
pub struct EmbeddedKeyValueStateProvider {
    store: Arc<dyn PersistStore>,
    mutations: Mutex<()>,
    namespaces: Mutex<BTreeSet<String>>,
    active_watches: Mutex<u32>,
    stopped: Mutex<bool>,
}

impl EmbeddedKeyValueStateProvider {
    /// Inject a host-owned persistence Strategy; the provider never opens files itself.
    pub fn new(store: Arc<dyn PersistStore>) -> Self {
        Self {
            store,
            mutations: Mutex::new(()),
            namespaces: Mutex::new(BTreeSet::new()),
            active_watches: Mutex::new(0),
            stopped: Mutex::new(false),
        }
    }

    async fn load(&self, key: &KeyValueKeyRef) -> ServiceResult<Option<StoredEntry>> {
        validate_key(key)?;
        let storage_key = entry_key(key);
        let Some(bytes) = self.store.get(&storage_key).await.map_err(persist_error)? else {
            return Ok(None);
        };
        let entry: StoredEntry = serde_json::from_slice(&bytes).map_err(json_error)?;
        if entry.is_expired() {
            self.store
                .delete(&storage_key)
                .await
                .map_err(persist_error)?;
            return Ok(None);
        }
        Ok(Some(entry))
    }

    async fn put(&self, request: KeyValuePutCommand) -> ServiceResult<StoredEntry> {
        validate_key(&request.key)?;
        if !request.value.is_admissible_reference() {
            return Err(ServiceError::AdapterFailure(
                "invalid opaque value reference".into(),
            ));
        }
        let _mutation = self.mutations.lock().await;
        let prior = self.load(&request.key).await?;
        let generation = prior.map_or(1, |entry| entry.revision.generation.saturating_add(1));
        let entry = StoredEntry {
            revision: KeyValueRevision {
                revision_id: hash(&format!("{}:{generation}", entry_key(&request.key))),
                generation,
            },
            value: request.value,
            expire_at_epoch_millis: expiry(request.ttl.as_ref())?,
        };
        self.store
            .set(
                &entry_key(&request.key),
                &serde_json::to_vec(&entry).map_err(json_error)?,
            )
            .await
            .map_err(persist_error)?;
        self.namespaces
            .lock()
            .await
            .insert(namespace_hash(&request.key));
        Ok(entry)
    }
}

#[async_trait]
impl KeyValueStateService for EmbeddedKeyValueStateProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(FOUNDATION_KEY_VALUE_STATE_SERVICE_ID),
            ServiceType::new("foundation.key_value_state"),
            TraceSchemaRef::new("macaca.trace.foundation.key_value_state.v1"),
        );
        descriptor.health = ServiceHealth::Healthy;
        descriptor.cleanup_policy = CleanupPolicy::OnStop;
        descriptor.capabilities = supported_commands()
            .iter()
            .map(|name| ServiceCapability::new(CapabilityId::new(*name), "key-value state command"))
            .collect();
        descriptor
            .metadata
            .insert("provider_class".into(), "embedded-durable".into());
        descriptor
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        if *self.stopped.lock().await {
            return unavailable(trace, "provider_stopped");
        }
        let operation = command.name.as_str();
        if !FOUNDATION_KEY_VALUE_STATE_COMMANDS.contains(&operation) {
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        let (output, status) = match operation {
            "kv.get" => {
                let request = decode::<KeyValueGetCommand>(&command.payload)?;
                match self.load(&request.key).await? {
                    Some(entry) => (
                        serde_json::json!({"status":"success","revision":entry.revision,"value_present":true}),
                        "ok",
                    ),
                    None => (serde_json::json!({"status":"not_found"}), "not_found"),
                }
            }
            "kv.put" => {
                let entry = self
                    .put(decode::<KeyValuePutCommand>(&command.payload)?)
                    .await?;
                (
                    serde_json::json!({"status":"success","revision":entry.revision}),
                    "ok",
                )
            }
            "kv.delete" => {
                let deleted = self
                    .delete(decode::<KeyValueDeleteCommand>(&command.payload)?)
                    .await?;
                (
                    serde_json::json!({"status":if deleted {"success"} else {"not_found"}}),
                    if deleted { "ok" } else { "not_found" },
                )
            }
            "kv.exists" => {
                let request = decode::<KeyValueExistsCommand>(&command.payload)?;
                (
                    serde_json::json!({"status":"success","exists":self.load(&request.key).await?.is_some()}),
                    "ok",
                )
            }
            "kv.batch_get" => {
                let request = decode::<KeyValueBatchGetCommand>(&command.payload)?;
                if request.keys.len() > 500 {
                    return Err(ServiceError::AdapterFailure("batch limit exceeded".into()));
                }
                let mut found = 0_u32;
                for key in &request.keys {
                    found += u32::from(self.load(key).await?.is_some());
                }
                (
                    serde_json::json!({"status":"success","requested":request.keys.len(),"found":found}),
                    "ok",
                )
            }
            "kv.batch_put" => {
                let request = decode::<KeyValueBatchPutCommand>(&command.payload)?;
                if request.entries.is_empty() || request.entries.len() > 500 {
                    return Err(ServiceError::AdapterFailure("invalid bounded batch".into()));
                }
                for entry in request.entries {
                    self.put(entry).await?;
                }
                (serde_json::json!({"status":"success"}), "ok")
            }
            "kv.batch_delete" => {
                let request = decode::<KeyValueBatchDeleteCommand>(&command.payload)?;
                if request.keys.is_empty() || request.keys.len() > 500 {
                    return Err(ServiceError::AdapterFailure("invalid bounded batch".into()));
                }
                let mut deleted = 0_u32;
                for key in request.keys {
                    deleted += u32::from(
                        self.delete(KeyValueDeleteCommand {
                            key,
                            expected_revision: request.expected_revision.clone(),
                        })
                        .await?,
                    );
                }
                (
                    serde_json::json!({"status":"success","deleted":deleted}),
                    "ok",
                )
            }
            "kv.compare_and_set" => {
                let entry = self
                    .compare_and_set(decode::<KeyValueCompareAndSetCommand>(&command.payload)?)
                    .await?;
                (
                    serde_json::json!({"status":"success","revision":entry.revision}),
                    "ok",
                )
            }
            "kv.set_ttl" => {
                let entry = self
                    .set_ttl(decode::<KeyValueSetTtlCommand>(&command.payload)?)
                    .await?;
                (
                    serde_json::json!({"status":"success","revision":entry.revision}),
                    "ok",
                )
            }
            "kv.get_ttl" => {
                let request = decode::<KeyValueGetTtlCommand>(&command.payload)?;
                let ttl = self.load(&request.key).await?.and_then(|entry| {
                    entry
                        .expire_at_epoch_millis
                        .map(|expiry| expiry.saturating_sub(now()))
                });
                (
                    serde_json::json!({"status":"success","ttl_millis":ttl}),
                    "ok",
                )
            }
            "kv.list_keys" => (
                self.list(decode::<KeyValueListKeysCommand>(&command.payload)?)
                    .await?,
                "ok",
            ),
            "kv.snapshot_namespace" => {
                let snapshot_id = self
                    .snapshot(
                        decode::<KeyValueSnapshotNamespaceCommand>(&command.payload)?,
                        &trace.trace_id,
                    )
                    .await?;
                (
                    serde_json::json!({"status":"success","snapshot_ref":snapshot_id}),
                    "ok",
                )
            }
            "kv.restore_namespace" => (
                self.restore(decode::<KeyValueRestoreNamespaceCommand>(&command.payload)?)
                    .await?,
                "ok",
            ),
            "kv.migrate_namespace" => (
                self.migrate(decode::<KeyValueMigrateNamespaceCommand>(&command.payload)?)
                    .await?,
                "ok",
            ),
            "kv.compact_namespace" => (
                self.compact(decode::<KeyValueCompactNamespaceCommand>(&command.payload)?)
                    .await?,
                "ok",
            ),
            "kv.watch_namespace" => {
                let request = decode::<KeyValueWatchNamespaceCommand>(&command.payload)?;
                if !request.namespace.is_bounded_reference() {
                    return Err(ServiceError::AdapterFailure(
                        "invalid namespace reference".into(),
                    ));
                }
                let mut watches = self.active_watches.lock().await;
                if *watches >= 32 {
                    return Err(ServiceError::AdapterFailure("watch quota exceeded".into()));
                }
                *watches += 1;
                (
                    serde_json::json!({"status":"watch_checkpoint","checkpoint":hash(&trace.trace_id),"active_watch_count":*watches}),
                    "ok",
                )
            }
            _ => (serde_json::json!({"status":"unsupported"}), "unsupported"),
        };
        tracing::info!(service_id = FOUNDATION_KEY_VALUE_STATE_SERVICE_ID, command = operation,
            trace_id = %trace.trace_id, "embedded key-value provider completed command");
        Ok(ServiceCallResult {
            output,
            trace,
            status: status.into(),
            metadata: BTreeMap::from([
                ("replay.provider_class".into(), "embedded-durable".into()),
                ("replay.key_value_state_command".into(), operation.into()),
                (
                    "key_value_state.redaction".into(),
                    "namespaces_keys_and_values_redacted".into(),
                ),
            ]),
            cleanup_hint: Some(CleanupPolicy::OnStop),
        })
    }

    fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }

    fn snapshot(&self) -> macaca_proto::KeyValueStateProviderSnapshot {
        macaca_proto::KeyValueStateProviderSnapshot {
            descriptor_hash: "foundation-key-value-state-embedded-v1".into(),
            provider_class: "embedded-durable".into(),
            namespace_hashes: BTreeMap::new(),
            active_watch_count: self
                .active_watches
                .try_lock()
                .map(|count| *count)
                .unwrap_or(0),
        }
    }

    fn provider_capabilities(&self) -> macaca_proto::KeyValueStateProviderCapability {
        macaca_proto::KeyValueStateProviderCapability {
            provider_class: "embedded-durable".into(),
            supported_commands: supported_commands()
                .into_iter()
                .map(str::to_string)
                .collect(),
            supports_ttl: true,
            supports_watch: true,
            supports_snapshot: true,
            supports_compaction: true,
            max_value_bytes: 1_048_576,
            max_batch_entries: 500,
            availability: DomainPackProviderCapabilityState::Available,
        }
    }

    async fn shutdown(&self) -> ServiceResult<()> {
        *self.stopped.lock().await = true;
        *self.active_watches.lock().await = 0;
        Ok(())
    }

    async fn cancel_watch(&self, trace_id: &str) -> ServiceResult<()> {
        let mut watches = self.active_watches.lock().await;
        if *watches > 0 {
            *watches -= 1;
        }
        tracing::info!(
            service_id = FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
            trace_id,
            active_watch_count = *watches,
            "embedded key-value watch cancelled"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredEntry {
    revision: KeyValueRevision,
    value: KeyValueTypedValueRef,
    expire_at_epoch_millis: Option<u64>,
}
impl StoredEntry {
    fn is_expired(&self) -> bool {
        self.expire_at_epoch_millis
            .is_some_and(|expiry| expiry <= now())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSnapshot {
    namespace: String,
    entries: BTreeMap<String, StoredEntry>,
}

fn decode<T: serde::de::DeserializeOwned>(payload: &serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(payload.clone()).map_err(json_error)
}
fn supported_commands() -> [&'static str; 16] {
    [
        "kv.batch_get",
        "kv.batch_put",
        "kv.batch_delete",
        "kv.get",
        "kv.put",
        "kv.delete",
        "kv.exists",
        "kv.list_keys",
        "kv.compare_and_set",
        "kv.set_ttl",
        "kv.get_ttl",
        "kv.snapshot_namespace",
        "kv.restore_namespace",
        "kv.migrate_namespace",
        "kv.compact_namespace",
        "kv.watch_namespace",
    ]
}
fn validate_key(key: &KeyValueKeyRef) -> ServiceResult<()> {
    if key.is_bounded_reference() {
        Ok(())
    } else {
        Err(ServiceError::AdapterFailure("invalid key reference".into()))
    }
}
fn entry_key(key: &KeyValueKeyRef) -> String {
    format!("{ENTRY_PREFIX}{}/{}", namespace_hash(key), hash(&key.key))
}
fn namespace_hash(key: &KeyValueKeyRef) -> String {
    namespace_hash_ref(&key.namespace)
}
fn namespace_hash_ref(namespace: &macaca_proto::KeyValueNamespaceRef) -> String {
    hash(&format!(
        "{}:{}",
        namespace.tenant_ref.as_deref().unwrap_or_default(),
        namespace.namespace
    ))
}
fn hash(value: &str) -> String {
    format!(
        "{:016x}",
        value.bytes().fold(0_u64, |state, byte| state
            .wrapping_mul(1099511628211)
            .wrapping_add(byte as u64))
    )
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn expiry(ttl: Option<&KeyValueTtlPolicy>) -> ServiceResult<Option<u64>> {
    match ttl {
        None => Ok(None),
        Some(policy) if policy.is_bounded(u64::MAX, now()) => {
            Ok(policy.expire_at_epoch_millis.or_else(|| {
                policy
                    .ttl_seconds
                    .map(|seconds| now().saturating_add(seconds.saturating_mul(1000)))
            }))
        }
        _ => Err(ServiceError::AdapterFailure("invalid TTL policy".into())),
    }
}
fn persist_error(error: macaca_proto::MacacaError) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
fn json_error(error: serde_json::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
fn unavailable(
    trace: macaca_proto::TraceContext,
    reason: &str,
) -> ServiceResult<ServiceCallResult> {
    Ok(ServiceCallResult {
        output: serde_json::json!({"status":"unavailable","reason":reason}),
        trace,
        status: "unavailable".into(),
        metadata: BTreeMap::new(),
        cleanup_hint: Some(CleanupPolicy::None),
    })
}
