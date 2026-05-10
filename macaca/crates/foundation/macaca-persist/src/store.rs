use async_trait::async_trait;
use macaca_proto::error::MacacaResult;

/// A generic key-value persistence store.
#[async_trait]
pub trait PersistStore: Send + Sync {
    async fn get(&self, key: &str) -> MacacaResult<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8]) -> MacacaResult<()>;
    async fn delete(&self, key: &str) -> MacacaResult<()>;
    async fn list_keys(&self, prefix: &str) -> MacacaResult<Vec<String>>;
}

/// Strategy boundary for concrete persistence backends.
///
/// Existing consumers may continue depending on `PersistStore`. This trait
/// makes the backend identity explicit without forcing a breaking change.
pub trait PersistBackend: PersistStore {
    fn backend_name(&self) -> &'static str;
}
