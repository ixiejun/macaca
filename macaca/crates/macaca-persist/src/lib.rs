//! aos-persist: redb-backed persistence layer for Agent OS.

pub mod checkpoint;
pub mod redb_store;
pub mod store;

pub use checkpoint::CheckpointManager;
pub use redb_store::RedbStore;
pub use store::PersistStore;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use tempfile::tempdir;

    use macaca_proto::types::{
        AgentId, AgentManifest, AgentState, Capability, Permission, PermissionLevel,
    };

    use super::*;

    fn temp_store() -> (RedbStore, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        let store = RedbStore::open(path).expect("open store");
        (store, dir)
    }

    fn sample_manifest(id: AgentId) -> AgentManifest {
        AgentManifest {
            id,
            name: "test-agent".into(),
            capabilities: vec![Capability {
                name: "read".into(),
                description: "read files".into(),
            }],
            permission: Permission {
                level: PermissionLevel::User,
                allowed_tools: vec!["read_file".into()],
                allowed_paths: vec!["/tmp".into()],
                network_access: false,
            },
            state: AgentState::Running,
            created_at: Utc::now(),
            model: String::new(),
        }
    }

    // ── RedbStore CRUD ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn redb_set_and_get() {
        let (store, _dir) = temp_store();
        store.set("hello", b"world").await.unwrap();
        let val = store.get("hello").await.unwrap();
        assert_eq!(val, Some(b"world".to_vec()));
    }

    #[tokio::test]
    async fn redb_get_missing_returns_none() {
        let (store, _dir) = temp_store();
        let val = store.get("nonexistent").await.unwrap();
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn redb_delete_removes_key() {
        let (store, _dir) = temp_store();
        store.set("k", b"v").await.unwrap();
        store.delete("k").await.unwrap();
        let val = store.get("k").await.unwrap();
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn redb_delete_nonexistent_is_ok() {
        let (store, _dir) = temp_store();
        store.delete("ghost").await.unwrap();
    }

    #[tokio::test]
    async fn redb_list_keys_prefix() {
        let (store, _dir) = temp_store();
        store.set("foo/a", b"1").await.unwrap();
        store.set("foo/b", b"2").await.unwrap();
        store.set("bar/c", b"3").await.unwrap();

        let mut keys = store.list_keys("foo/").await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["foo/a", "foo/b"]);
    }

    #[tokio::test]
    async fn redb_list_keys_empty_prefix_returns_all() {
        let (store, _dir) = temp_store();
        store.set("x", b"1").await.unwrap();
        store.set("y", b"2").await.unwrap();

        let mut keys = store.list_keys("").await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["x", "y"]);
    }

    // ── CheckpointManager ──────────────────────────────────────────────────

    #[tokio::test]
    async fn checkpoint_save_and_load() {
        let (store, _dir) = temp_store();
        let store: Arc<dyn PersistStore> = Arc::new(store);
        let mgr = CheckpointManager::new(Arc::clone(&store));

        let id = AgentId::new();
        let manifest = sample_manifest(id);

        mgr.save_snapshot(&id, &manifest).await.unwrap();

        let loaded = mgr.load_snapshot(&id).await.unwrap().expect("snapshot present");
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.name, "test-agent");
        assert_eq!(loaded.state, AgentState::Running);
    }

    #[tokio::test]
    async fn checkpoint_load_missing_returns_none() {
        let (store, _dir) = temp_store();
        let store: Arc<dyn PersistStore> = Arc::new(store);
        let mgr = CheckpointManager::new(store);

        let result = mgr.load_snapshot(&AgentId::new()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn checkpoint_list_snapshots() {
        let (store, _dir) = temp_store();
        let store: Arc<dyn PersistStore> = Arc::new(store);
        let mgr = CheckpointManager::new(Arc::clone(&store));

        let id1 = AgentId::new();
        let id2 = AgentId::new();
        mgr.save_snapshot(&id1, &sample_manifest(id1)).await.unwrap();
        mgr.save_snapshot(&id2, &sample_manifest(id2)).await.unwrap();

        let mut ids = mgr.list_snapshots().await.unwrap();
        ids.sort_by_key(|a| a.0);
        let mut expected = vec![id1, id2];
        expected.sort_by_key(|a| a.0);
        assert_eq!(ids, expected);
    }

    #[tokio::test]
    async fn checkpoint_delete_snapshot() {
        let (store, _dir) = temp_store();
        let store: Arc<dyn PersistStore> = Arc::new(store);
        let mgr = CheckpointManager::new(Arc::clone(&store));

        let id = AgentId::new();
        mgr.save_snapshot(&id, &sample_manifest(id)).await.unwrap();
        mgr.delete_snapshot(&id).await.unwrap();

        let result = mgr.load_snapshot(&id).await.unwrap();
        assert!(result.is_none());

        let ids = mgr.list_snapshots().await.unwrap();
        assert!(ids.is_empty());
    }
}
