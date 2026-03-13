use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use redb::{Database, ReadableTable, TableDefinition};
use macaca_proto::error::{MacacaError, MacacaResult};

use crate::store::PersistStore;

const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("macaca_data");

/// A [`PersistStore`] backed by a redb embedded database.
pub struct RedbStore {
    db: Arc<Database>,
}

impl RedbStore {
    /// Open (or create) a redb database at `path`.
    pub fn open(path: impl AsRef<Path>) -> MacacaResult<Self> {
        let db = Database::create(path.as_ref())
            .map_err(|e| MacacaError::Persist(e.to_string()))?;

        // Ensure the table exists.
        let write_txn = db.begin_write()
            .map_err(|e| MacacaError::Persist(e.to_string()))?;
        {
            write_txn.open_table(TABLE)
                .map_err(|e| MacacaError::Persist(e.to_string()))?;
        }
        write_txn.commit()
            .map_err(|e| MacacaError::Persist(e.to_string()))?;

        Ok(Self { db: Arc::new(db) })
    }
}

#[async_trait]
impl PersistStore for RedbStore {
    async fn get(&self, key: &str) -> MacacaResult<Option<Vec<u8>>> {
        let db = Arc::clone(&self.db);
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || {
            let read_txn = db.begin_read()
                .map_err(|e| MacacaError::Persist(e.to_string()))?;
            let table = read_txn.open_table(TABLE)
                .map_err(|e| MacacaError::Persist(e.to_string()))?;
            let result = table.get(key.as_str())
                .map_err(|e| MacacaError::Persist(e.to_string()))?;
            Ok(result.map(|v| v.value().to_vec()))
        })
        .await
        .map_err(|e| MacacaError::Persist(e.to_string()))?
    }

    async fn set(&self, key: &str, value: &[u8]) -> MacacaResult<()> {
        let db = Arc::clone(&self.db);
        let key = key.to_owned();
        let value = value.to_vec();
        tokio::task::spawn_blocking(move || {
            let write_txn = db.begin_write()
                .map_err(|e| MacacaError::Persist(e.to_string()))?;
            {
                let mut table = write_txn.open_table(TABLE)
                    .map_err(|e| MacacaError::Persist(e.to_string()))?;
                table.insert(key.as_str(), value.as_slice())
                    .map_err(|e| MacacaError::Persist(e.to_string()))?;
            }
            write_txn.commit()
                .map_err(|e| MacacaError::Persist(e.to_string()))
        })
        .await
        .map_err(|e| MacacaError::Persist(e.to_string()))?
    }

    async fn delete(&self, key: &str) -> MacacaResult<()> {
        let db = Arc::clone(&self.db);
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || {
            let write_txn = db.begin_write()
                .map_err(|e| MacacaError::Persist(e.to_string()))?;
            {
                let mut table = write_txn.open_table(TABLE)
                    .map_err(|e| MacacaError::Persist(e.to_string()))?;
                table.remove(key.as_str())
                    .map_err(|e| MacacaError::Persist(e.to_string()))?;
            }
            write_txn.commit()
                .map_err(|e| MacacaError::Persist(e.to_string()))
        })
        .await
        .map_err(|e| MacacaError::Persist(e.to_string()))?
    }

    async fn list_keys(&self, prefix: &str) -> MacacaResult<Vec<String>> {
        let db = Arc::clone(&self.db);
        let prefix = prefix.to_owned();
        tokio::task::spawn_blocking(move || {
            let read_txn = db.begin_read()
                .map_err(|e| MacacaError::Persist(e.to_string()))?;
            let table = read_txn.open_table(TABLE)
                .map_err(|e| MacacaError::Persist(e.to_string()))?;
            let mut keys = Vec::new();
            for entry in table.iter().map_err(|e| MacacaError::Persist(e.to_string()))? {
                let (k, _) = entry.map_err(|e| MacacaError::Persist(e.to_string()))?;
                let k_str = k.value().to_owned();
                if k_str.starts_with(&prefix) {
                    keys.push(k_str);
                }
            }
            Ok(keys)
        })
        .await
        .map_err(|e| MacacaError::Persist(e.to_string()))?
    }
}
