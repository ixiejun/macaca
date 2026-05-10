//! Session persistence — save and restore agent state across conversations.

use crate::state::{StateError, StateModule};
use async_trait::async_trait;

/// Errors from session operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SessionError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("State error: {0}")]
    State(#[from] StateError),
}

/// Trait for persisting agent state across sessions.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Save a state module's state under a session+module key.
    async fn save(
        &self,
        session_id: &str,
        module_name: &str,
        state: serde_json::Value,
    ) -> Result<(), SessionError>;
    /// Load a state module's state from a session+module key.
    async fn load(
        &self,
        session_id: &str,
        module_name: &str,
    ) -> Result<Option<serde_json::Value>, SessionError>;
    /// Delete all state for a session.
    async fn delete_session(&self, session_id: &str) -> Result<(), SessionError>;
    /// List all session IDs.
    async fn list_sessions(&self) -> Result<Vec<String>, SessionError>;
}

/// Number of shards for the in-memory session store.
const NUM_SHARDS: usize = 8;

/// In-memory session store with sharded locks for improved concurrency.
pub struct InMemorySessionStore {
    shards: [tokio::sync::RwLock<std::collections::HashMap<String, serde_json::Value>>; NUM_SHARDS],
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            shards: std::array::from_fn(|_| {
                tokio::sync::RwLock::new(std::collections::HashMap::new())
            }),
        }
    }

    fn shard_index(key: &str) -> usize {
        // Simple hash: sum of bytes mod NUM_SHARDS
        let hash: usize = key.bytes().map(|b| b as usize).sum();
        hash % NUM_SHARDS
    }

    fn shard(
        &self,
        key: &str,
    ) -> &tokio::sync::RwLock<std::collections::HashMap<String, serde_json::Value>> {
        &self.shards[Self::shard_index(key)]
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn save(
        &self,
        session_id: &str,
        module_name: &str,
        state: serde_json::Value,
    ) -> Result<(), SessionError> {
        let key = format!("{}/{}", session_id, module_name);
        self.shard(&key).write().await.insert(key, state);
        Ok(())
    }
    async fn load(
        &self,
        session_id: &str,
        module_name: &str,
    ) -> Result<Option<serde_json::Value>, SessionError> {
        let key = format!("{}/{}", session_id, module_name);
        Ok(self.shard(&key).read().await.get(&key).cloned())
    }
    async fn delete_session(&self, session_id: &str) -> Result<(), SessionError> {
        let prefix = format!("{}/", session_id);
        // Must iterate all shards since keys for the same session may land in different shards.
        for shard in &self.shards {
            let mut data = shard.write().await;
            data.retain(|k, _| !k.starts_with(&prefix));
        }
        Ok(())
    }
    async fn list_sessions(&self) -> Result<Vec<String>, SessionError> {
        let mut session_set = std::collections::HashSet::new();
        for shard in &self.shards {
            let data = shard.read().await;
            for key in data.keys() {
                if let Some(sid) = key.split('/').next() {
                    session_set.insert(sid.to_string());
                }
            }
        }
        let mut sessions: Vec<String> = session_set.into_iter().collect();
        sessions.sort();
        Ok(sessions)
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper: save a StateModule to a session store.
pub async fn save_module_state(
    store: &dyn SessionStore,
    session_id: &str,
    module: &dyn StateModule,
) -> Result<(), SessionError> {
    let state = module.state_dict();
    store.save(session_id, module.module_name(), state).await
}

/// Helper: load state into a StateModule from a session store.
pub async fn load_module_state(
    store: &dyn SessionStore,
    session_id: &str,
    module: &mut dyn StateModule,
) -> Result<(), SessionError> {
    if let Some(state) = store.load(session_id, module.module_name()).await? {
        module.load_state_dict(state)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StateError;

    struct Counter {
        count: u64,
    }

    impl StateModule for Counter {
        fn state_dict(&self) -> serde_json::Value {
            serde_json::json!({ "count": self.count })
        }
        fn load_state_dict(&mut self, state: serde_json::Value) -> Result<(), StateError> {
            self.count = state
                .get("count")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| StateError::MissingField("count".into()))?;
            Ok(())
        }
        fn module_name(&self) -> &str {
            "counter"
        }
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let store = InMemorySessionStore::new();
        let state = serde_json::json!({ "count": 42 });
        store.save("sess1", "counter", state.clone()).await.unwrap();
        let loaded = store.load("sess1", "counter").await.unwrap();
        assert_eq!(loaded, Some(state));
    }

    #[tokio::test]
    async fn test_load_missing() {
        let store = InMemorySessionStore::new();
        let result = store.load("no-such-session", "counter").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_session() {
        let store = InMemorySessionStore::new();
        store
            .save("sess1", "counter", serde_json::json!({ "count": 1 }))
            .await
            .unwrap();
        store
            .save("sess1", "memory", serde_json::json!({ "msgs": [] }))
            .await
            .unwrap();
        store
            .save("sess2", "counter", serde_json::json!({ "count": 2 }))
            .await
            .unwrap();

        store.delete_session("sess1").await.unwrap();

        assert!(store.load("sess1", "counter").await.unwrap().is_none());
        assert!(store.load("sess1", "memory").await.unwrap().is_none());
        // sess2 still intact
        assert!(store.load("sess2", "counter").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let store = InMemorySessionStore::new();
        store
            .save("alpha", "counter", serde_json::json!({}))
            .await
            .unwrap();
        store
            .save("beta", "counter", serde_json::json!({}))
            .await
            .unwrap();
        store
            .save("alpha", "memory", serde_json::json!({}))
            .await
            .unwrap();

        let sessions = store.list_sessions().await.unwrap();
        assert_eq!(sessions, vec!["alpha", "beta"]);
    }

    #[tokio::test]
    async fn test_save_load_module() {
        let store = InMemorySessionStore::new();
        let counter = Counter { count: 99 };

        save_module_state(&store, "sess1", &counter).await.unwrap();

        let mut restored = Counter { count: 0 };
        load_module_state(&store, "sess1", &mut restored)
            .await
            .unwrap();

        assert_eq!(restored.count, 99);
    }

    #[tokio::test]
    async fn test_save_load_roundtrip() {
        let store = InMemorySessionStore::new();
        let state = serde_json::json!({
            "count": 123,
            "tags": ["a", "b", "c"],
            "nested": { "x": 1, "y": 2 }
        });
        store
            .save("sess-rt", "my_module", state.clone())
            .await
            .unwrap();
        let loaded = store.load("sess-rt", "my_module").await.unwrap();
        assert_eq!(loaded, Some(state));
    }

    #[tokio::test]
    async fn test_delete_session_then_load() {
        let store = InMemorySessionStore::new();
        store
            .save("sess-del", "counter", serde_json::json!({"count": 5}))
            .await
            .unwrap();
        assert!(store.load("sess-del", "counter").await.unwrap().is_some());

        store.delete_session("sess-del").await.unwrap();
        assert!(store.load("sess-del", "counter").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_sessions_reflects_changes() {
        let store = InMemorySessionStore::new();
        store.save("aaa", "m", serde_json::json!({})).await.unwrap();
        store.save("bbb", "m", serde_json::json!({})).await.unwrap();
        store.save("ccc", "m", serde_json::json!({})).await.unwrap();

        let list = store.list_sessions().await.unwrap();
        assert_eq!(list, vec!["aaa", "bbb", "ccc"]);

        store.delete_session("bbb").await.unwrap();
        let list = store.list_sessions().await.unwrap();
        assert_eq!(list, vec!["aaa", "ccc"]);
    }

    #[tokio::test]
    async fn test_save_empty_state() {
        let store = InMemorySessionStore::new();
        let empty = serde_json::json!({});
        store
            .save("sess-empty", "mod", empty.clone())
            .await
            .unwrap();
        let loaded = store.load("sess-empty", "mod").await.unwrap();
        assert_eq!(loaded, Some(empty));
    }

    #[tokio::test]
    async fn test_deeply_nested_state() {
        let store = InMemorySessionStore::new();
        // Build a >10 levels deep nested JSON
        let mut val = serde_json::json!("leaf");
        for i in (0..12).rev() {
            val = serde_json::json!({ format!("level_{}", i): val });
        }
        store.save("sess-deep", "mod", val.clone()).await.unwrap();
        let loaded = store.load("sess-deep", "mod").await.unwrap();
        assert_eq!(loaded, Some(val));
    }

    // -----------------------------------------------------------------------
    // Sharded concurrency tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_sharded_concurrent_writes() {
        use std::sync::Arc;

        let store = Arc::new(InMemorySessionStore::new());
        let mut handles = Vec::new();

        // Spawn 50 concurrent writes across different sessions
        for i in 0..50 {
            let store = Arc::clone(&store);
            handles.push(tokio::spawn(async move {
                let sess = format!("sess-{}", i);
                let state = serde_json::json!({ "value": i });
                store.save(&sess, "mod", state).await.unwrap();
            }));
        }

        // Wait for all writes to complete
        for h in handles {
            h.await.unwrap();
        }

        // Verify all 50 sessions were correctly saved
        for i in 0..50 {
            let sess = format!("sess-{}", i);
            let loaded = store.load(&sess, "mod").await.unwrap();
            assert_eq!(loaded, Some(serde_json::json!({ "value": i })));
        }

        let sessions = store.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 50);
    }

    #[tokio::test]
    async fn test_sharded_store_delete_session() {
        let store = InMemorySessionStore::new();

        // Save multiple modules under the same session (different keys → potentially different shards)
        for i in 0..20 {
            let module_name = format!("module_{}", i);
            store
                .save("target-sess", &module_name, serde_json::json!({ "idx": i }))
                .await
                .unwrap();
        }

        // Also save data for a different session
        store
            .save("other-sess", "mod", serde_json::json!({ "keep": true }))
            .await
            .unwrap();

        // Delete the target session
        store.delete_session("target-sess").await.unwrap();

        // All target-sess modules should be gone
        for i in 0..20 {
            let module_name = format!("module_{}", i);
            assert!(store
                .load("target-sess", &module_name)
                .await
                .unwrap()
                .is_none());
        }

        // other-sess should be intact
        assert!(store.load("other-sess", "mod").await.unwrap().is_some());

        // list_sessions should only contain other-sess
        let sessions = store.list_sessions().await.unwrap();
        assert_eq!(sessions, vec!["other-sess"]);
    }
}
