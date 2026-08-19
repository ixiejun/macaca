//! Command-oriented operations for the durable embedded key-value provider.
//!
//! Keeping snapshot, migration, and bounded scan behavior in this module leaves the provider
//! composition and lifecycle surface small while preserving one service implementation.

use std::collections::BTreeMap;

use macaca_proto::{
    KeyValueCompactNamespaceCommand, KeyValueCompareAndSetCommand, KeyValueDeleteCommand,
    KeyValueListKeysCommand, KeyValueMigrateNamespaceCommand, KeyValuePutCommand,
    KeyValueRestoreNamespaceCommand, KeyValueSetTtlCommand, KeyValueSnapshotNamespaceCommand,
    ServiceError, ServiceResult,
};

use super::{
    entry_key, expiry, json_error, namespace_hash_ref, persist_error,
    EmbeddedKeyValueStateProvider, StoredEntry, StoredSnapshot, ENTRY_PREFIX, MAX_SCAN_PAGE_SIZE,
    SNAPSHOT_PREFIX,
};

impl EmbeddedKeyValueStateProvider {
    pub(super) async fn snapshot(
        &self,
        request: KeyValueSnapshotNamespaceCommand,
        trace_id: &str,
    ) -> ServiceResult<String> {
        if !request.namespace.is_bounded_reference() {
            return Err(ServiceError::AdapterFailure(
                "invalid namespace reference".into(),
            ));
        }
        let namespace = namespace_hash_ref(&request.namespace);
        let prefix = format!("{ENTRY_PREFIX}{namespace}/");
        let mut entries = BTreeMap::new();
        for storage_key in self.store.list_keys(&prefix).await.map_err(persist_error)? {
            if let Some(bytes) = self.store.get(&storage_key).await.map_err(persist_error)? {
                entries.insert(
                    storage_key,
                    serde_json::from_slice(&bytes).map_err(json_error)?,
                );
            }
        }
        let snapshot_id = super::hash(&format!("{namespace}:{trace_id}:{}", entries.len()));
        let snapshot = StoredSnapshot { namespace, entries };
        self.store
            .set(
                &format!("{SNAPSHOT_PREFIX}{snapshot_id}"),
                &serde_json::to_vec(&snapshot).map_err(json_error)?,
            )
            .await
            .map_err(persist_error)?;
        Ok(snapshot_id)
    }

    pub(super) async fn delete(&self, request: KeyValueDeleteCommand) -> ServiceResult<bool> {
        super::validate_key(&request.key)?;
        let _mutation = self.mutations.lock().await;
        let Some(entry) = self.load(&request.key).await? else {
            return Ok(false);
        };
        if request
            .expected_revision
            .is_some_and(|value| value != entry.revision)
        {
            return Err(ServiceError::AdapterFailure("revision conflict".into()));
        }
        self.store
            .delete(&entry_key(&request.key))
            .await
            .map_err(persist_error)?;
        Ok(true)
    }

    pub(super) async fn compare_and_set(
        &self,
        request: KeyValueCompareAndSetCommand,
    ) -> ServiceResult<StoredEntry> {
        let current = self
            .load(&request.key)
            .await?
            .ok_or_else(|| ServiceError::AdapterFailure("revision conflict".into()))?;
        if current.revision != request.expected_revision {
            return Err(ServiceError::AdapterFailure("revision conflict".into()));
        }
        self.put(KeyValuePutCommand {
            key: request.key,
            value: request.value,
            ttl: None,
            conflict_mode: macaca_proto::KeyValueConflictMode::CompareRevision,
        })
        .await
    }

    pub(super) async fn set_ttl(
        &self,
        request: KeyValueSetTtlCommand,
    ) -> ServiceResult<StoredEntry> {
        let _mutation = self.mutations.lock().await;
        let mut entry = self
            .load(&request.key)
            .await?
            .ok_or_else(|| ServiceError::AdapterFailure("key not found".into()))?;
        entry.expire_at_epoch_millis = expiry(Some(&request.ttl))?;
        self.store
            .set(
                &entry_key(&request.key),
                &serde_json::to_vec(&entry).map_err(json_error)?,
            )
            .await
            .map_err(persist_error)?;
        Ok(entry)
    }

    pub(super) async fn list(
        &self,
        request: KeyValueListKeysCommand,
    ) -> ServiceResult<serde_json::Value> {
        if !request.namespace.is_bounded_reference()
            || !(1..=MAX_SCAN_PAGE_SIZE).contains(&request.page_size)
        {
            return Err(ServiceError::AdapterFailure(
                "invalid bounded scan request".into(),
            ));
        }
        let prefix = format!("{ENTRY_PREFIX}{}/", namespace_hash_ref(&request.namespace));
        let mut keys = self.store.list_keys(&prefix).await.map_err(persist_error)?;
        keys.sort();
        let offset = request
            .cursor
            .as_deref()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let end = offset
            .saturating_add(request.page_size as usize)
            .min(keys.len());
        let key_hashes = keys[offset.min(keys.len())..end]
            .iter()
            .filter_map(|key| key.rsplit('/').next())
            .collect::<Vec<_>>();
        Ok(serde_json::json!({
            "status": if end < keys.len() { "partial_page" } else { "success" },
            "key_hashes": key_hashes,
            "next_cursor": (end < keys.len()).then_some(end.to_string())
        }))
    }

    pub(super) async fn restore(
        &self,
        request: KeyValueRestoreNamespaceCommand,
    ) -> ServiceResult<serde_json::Value> {
        let key = format!("{SNAPSHOT_PREFIX}{}", request.snapshot.snapshot_id);
        let bytes = self
            .store
            .get(&key)
            .await
            .map_err(persist_error)?
            .ok_or_else(|| ServiceError::AdapterFailure("snapshot not found".into()))?;
        let snapshot: StoredSnapshot = serde_json::from_slice(&bytes).map_err(json_error)?;
        if request.dry_run {
            return Ok(serde_json::json!({
                "status": "success",
                "dry_run": true,
                "entry_count": snapshot.entries.len()
            }));
        }
        let _mutation = self.mutations.lock().await;
        for (storage_key, entry) in &snapshot.entries {
            self.store
                .set(storage_key, &serde_json::to_vec(entry).map_err(json_error)?)
                .await
                .map_err(persist_error)?;
        }
        Ok(serde_json::json!({
            "status": "success",
            "entry_count": snapshot.entries.len()
        }))
    }

    pub(super) async fn migrate(
        &self,
        request: KeyValueMigrateNamespaceCommand,
    ) -> ServiceResult<serde_json::Value> {
        if !request.source.is_bounded_reference() || !request.target.is_bounded_reference() {
            return Err(ServiceError::AdapterFailure(
                "invalid namespace reference".into(),
            ));
        }
        let source = namespace_hash_ref(&request.source);
        let target = namespace_hash_ref(&request.target);
        let keys = self
            .store
            .list_keys(&format!("{ENTRY_PREFIX}{source}/"))
            .await
            .map_err(persist_error)?;
        if request.dry_run {
            return Ok(serde_json::json!({
                "status": "success",
                "dry_run": true,
                "entry_count": keys.len()
            }));
        }
        let _mutation = self.mutations.lock().await;
        for old_key in &keys {
            if let Some(bytes) = self.store.get(old_key).await.map_err(persist_error)? {
                let suffix = old_key
                    .strip_prefix(&format!("{ENTRY_PREFIX}{source}/"))
                    .unwrap_or_default();
                self.store
                    .set(&format!("{ENTRY_PREFIX}{target}/{suffix}"), &bytes)
                    .await
                    .map_err(persist_error)?;
            }
        }
        Ok(serde_json::json!({
            "status": "success",
            "entry_count": keys.len()
        }))
    }

    pub(super) async fn compact(
        &self,
        request: KeyValueCompactNamespaceCommand,
    ) -> ServiceResult<serde_json::Value> {
        if !request.namespace.is_bounded_reference() {
            return Err(ServiceError::AdapterFailure(
                "invalid namespace reference".into(),
            ));
        }
        let keys = self
            .store
            .list_keys(&format!(
                "{ENTRY_PREFIX}{}/",
                namespace_hash_ref(&request.namespace)
            ))
            .await
            .map_err(persist_error)?;
        Ok(serde_json::json!({
            "status": "success",
            "dry_run": request.dry_run,
            "retained_entries": keys.len(),
            "before_revision": request.before_revision
        }))
    }
}
