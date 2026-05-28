//! Replayable store Strategy for `service.interaction`.
//!
//! The interaction provider depends on this trait instead of a concrete
//! database.  The default implementation stores records in `macaca-persist`
//! under stable keys, so Runtime Host can replay Thread/Turn/Item state without
//! using shell memory as the source of truth.  Future plugin, remote, or
//! tenant-specific stores can replace this Strategy without changing the
//! service command contract.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_persist::PersistBackend;
use macaca_proto::workbench::interaction::{
    InteractionItemRecord, InteractionThreadRecord, InteractionThreadStatus, InteractionTurnRecord,
};
use macaca_proto::{MacacaError, MacacaResult};

const PREFIX: &str = "interaction-ledger/";

#[async_trait]
pub trait InteractionLedgerStore: Send + Sync {
    async fn save_thread(&self, record: &InteractionThreadRecord) -> MacacaResult<()>;
    async fn get_thread(
        &self,
        session_id: &str,
        thread_id: &str,
    ) -> MacacaResult<Option<InteractionThreadRecord>>;
    async fn list_threads(
        &self,
        session_id: &str,
        include_archived: bool,
        limit: usize,
    ) -> MacacaResult<Vec<InteractionThreadRecord>>;
    async fn loaded_threads(&self, session_id: &str) -> MacacaResult<Vec<String>>;

    async fn save_turn(&self, record: &InteractionTurnRecord) -> MacacaResult<()>;
    async fn get_turn(
        &self,
        session_id: &str,
        turn_id: &str,
    ) -> MacacaResult<Option<InteractionTurnRecord>>;
    async fn list_turns(
        &self,
        session_id: &str,
        thread_id: &str,
        limit: usize,
    ) -> MacacaResult<Vec<InteractionTurnRecord>>;

    async fn append_item(&self, record: &InteractionItemRecord) -> MacacaResult<usize>;
    async fn save_item(&self, record: &InteractionItemRecord) -> MacacaResult<()>;
    async fn get_item(
        &self,
        session_id: &str,
        item_id: &str,
    ) -> MacacaResult<Option<InteractionItemRecord>>;
    async fn list_items(
        &self,
        session_id: &str,
        thread_id: &str,
        since_index: usize,
        limit: usize,
    ) -> MacacaResult<Vec<InteractionItemRecord>>;

    async fn snapshot_counts(&self, session_id: &str) -> MacacaResult<(usize, usize, usize)>;
}

/// PersistBackend-backed interaction ledger.
pub struct PersistInteractionLedgerStore {
    backend: Arc<dyn PersistBackend>,
}

impl PersistInteractionLedgerStore {
    pub fn new(backend: Arc<dyn PersistBackend>) -> Self {
        Self { backend }
    }

    fn thread_key(session_id: &str, thread_id: &str) -> String {
        format!("{PREFIX}{session_id}/threads/{thread_id}")
    }

    fn turn_key(session_id: &str, turn_id: &str) -> String {
        format!("{PREFIX}{session_id}/turns/{turn_id}")
    }

    fn item_key(session_id: &str, item_id: &str) -> String {
        format!("{PREFIX}{session_id}/items/{item_id}")
    }

    fn thread_prefix(session_id: &str) -> String {
        format!("{PREFIX}{session_id}/threads/")
    }

    fn turn_prefix(session_id: &str) -> String {
        format!("{PREFIX}{session_id}/turns/")
    }

    fn item_prefix(session_id: &str) -> String {
        format!("{PREFIX}{session_id}/items/")
    }

    fn thread_item_prefix(session_id: &str, thread_id: &str) -> String {
        format!("{PREFIX}{session_id}/thread-items/{thread_id}/")
    }

    async fn put<T: serde::Serialize>(&self, key: String, value: &T) -> MacacaResult<()> {
        let bytes = serde_json::to_vec(value).map_err(MacacaError::from)?;
        self.backend.set(&key, &bytes).await
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, key: String) -> MacacaResult<Option<T>> {
        let Some(bytes) = self.backend.get(&key).await? else {
            return Ok(None);
        };
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(MacacaError::from)
    }
}

#[async_trait]
impl InteractionLedgerStore for PersistInteractionLedgerStore {
    async fn save_thread(&self, record: &InteractionThreadRecord) -> MacacaResult<()> {
        self.put(
            Self::thread_key(&record.scope.session_id, &record.thread_id),
            record,
        )
        .await
    }

    async fn get_thread(
        &self,
        session_id: &str,
        thread_id: &str,
    ) -> MacacaResult<Option<InteractionThreadRecord>> {
        self.get(Self::thread_key(session_id, thread_id)).await
    }

    async fn list_threads(
        &self,
        session_id: &str,
        include_archived: bool,
        limit: usize,
    ) -> MacacaResult<Vec<InteractionThreadRecord>> {
        let mut records = Vec::new();
        for key in self
            .backend
            .list_keys(&Self::thread_prefix(session_id))
            .await?
        {
            if records.len() >= limit {
                break;
            }
            if let Some(record) = self.get::<InteractionThreadRecord>(key).await? {
                if include_archived || record.status == InteractionThreadStatus::Active {
                    records.push(record);
                }
            }
        }
        records.sort_by_key(|record| record.created_at);
        Ok(records)
    }

    async fn loaded_threads(&self, session_id: &str) -> MacacaResult<Vec<String>> {
        Ok(self
            .list_threads(session_id, false, usize::MAX)
            .await?
            .into_iter()
            .map(|record| record.thread_id)
            .collect())
    }

    async fn save_turn(&self, record: &InteractionTurnRecord) -> MacacaResult<()> {
        self.put(Self::turn_key(&record.session_id, &record.turn_id), record)
            .await
    }

    async fn get_turn(
        &self,
        session_id: &str,
        turn_id: &str,
    ) -> MacacaResult<Option<InteractionTurnRecord>> {
        self.get(Self::turn_key(session_id, turn_id)).await
    }

    async fn list_turns(
        &self,
        session_id: &str,
        thread_id: &str,
        limit: usize,
    ) -> MacacaResult<Vec<InteractionTurnRecord>> {
        let mut records = Vec::new();
        for key in self
            .backend
            .list_keys(&Self::turn_prefix(session_id))
            .await?
        {
            if records.len() >= limit {
                break;
            }
            if let Some(record) = self.get::<InteractionTurnRecord>(key).await? {
                if record.thread_id == thread_id {
                    records.push(record);
                }
            }
        }
        records.sort_by_key(|record| record.started_at);
        Ok(records)
    }

    async fn append_item(&self, record: &InteractionItemRecord) -> MacacaResult<usize> {
        self.save_item(record).await?;
        let prefix = Self::thread_item_prefix(session_id_from_item(record), &record.thread_id);
        let index = self.backend.list_keys(&prefix).await?.len();
        self.backend
            .set(&format!("{prefix}{index:08}"), record.item_id.as_bytes())
            .await?;
        Ok(index)
    }

    async fn save_item(&self, record: &InteractionItemRecord) -> MacacaResult<()> {
        self.put(Self::item_key(&record.session_id, &record.item_id), record)
            .await
    }

    async fn get_item(
        &self,
        session_id: &str,
        item_id: &str,
    ) -> MacacaResult<Option<InteractionItemRecord>> {
        self.get(Self::item_key(session_id, item_id)).await
    }

    async fn list_items(
        &self,
        session_id: &str,
        thread_id: &str,
        since_index: usize,
        limit: usize,
    ) -> MacacaResult<Vec<InteractionItemRecord>> {
        let prefix = Self::thread_item_prefix(session_id, thread_id);
        let mut keys = self.backend.list_keys(&prefix).await?;
        keys.sort();
        let mut records = Vec::new();
        for key in keys.into_iter().skip(since_index).take(limit) {
            let Some(bytes) = self.backend.get(&key).await? else {
                continue;
            };
            let item_id = String::from_utf8(bytes)
                .map_err(|error| MacacaError::Persist(error.to_string()))?;
            if let Some(record) = self.get_item(session_id, &item_id).await? {
                records.push(record);
            }
        }
        Ok(records)
    }

    async fn snapshot_counts(&self, session_id: &str) -> MacacaResult<(usize, usize, usize)> {
        Ok((
            self.backend
                .list_keys(&Self::thread_prefix(session_id))
                .await?
                .len(),
            self.backend
                .list_keys(&Self::turn_prefix(session_id))
                .await?
                .len(),
            self.backend
                .list_keys(&Self::item_prefix(session_id))
                .await?
                .len(),
        ))
    }
}

fn session_id_from_item(record: &InteractionItemRecord) -> &str {
    &record.session_id
}
