//! aos-persist: redb-backed persistence layer for Agent OS.

pub mod checkpoint;
pub mod entitlement_store;
pub mod event_index;
pub mod event_log;
pub mod lineage;
pub mod redb_store;
pub mod store;

#[cfg(test)]
mod event_log_tests;

pub use checkpoint::{CheckpointBuilder, CheckpointManager, CheckpointRecord, SessionSnapshot};
pub use entitlement_store::{EntitlementStore, InMemoryEntitlementStore};
pub use event_index::EventLogQuery;
pub use event_log::{AppendEventCommand, EventLog, EventReplayIterator};
pub use lineage::SessionLineageStore;
pub use redb_store::RedbStore;
pub use store::{PersistBackend, PersistStore};

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

        let loaded = mgr
            .load_snapshot(&id)
            .await
            .unwrap()
            .expect("snapshot present");
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
        mgr.save_snapshot(&id1, &sample_manifest(id1))
            .await
            .unwrap();
        mgr.save_snapshot(&id2, &sample_manifest(id2))
            .await
            .unwrap();

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

    #[tokio::test]
    async fn checkpoint_builder_preserves_snapshot_save_behavior() {
        let (store, _dir) = temp_store();
        let store: Arc<dyn PersistStore> = Arc::new(store);
        let mgr = CheckpointManager::new(Arc::clone(&store));

        let id = AgentId::new();
        let manifest = sample_manifest(id);
        let record = CheckpointBuilder::new()
            .agent_id(id)
            .state(manifest.clone())
            .build()
            .unwrap();
        mgr.save_record(record).await.unwrap();

        let loaded = mgr.load_snapshot(&id).await.unwrap().unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.name, manifest.name);
    }

    #[test]
    fn session_snapshot_memento_captures_restore_metadata() {
        let snapshot = SessionSnapshot {
            session_id: "sess-1".into(),
            last_event_cursor: 42,
            checkpoint_key: Some("snapshot/agent-1".into()),
            coordinator_resume_key: Some("resume/coordinator".into()),
        };

        assert_eq!(snapshot.session_id, "sess-1");
        assert_eq!(snapshot.last_event_cursor, 42);
        assert_eq!(snapshot.checkpoint_key.as_deref(), Some("snapshot/agent-1"));
        assert_eq!(
            snapshot.coordinator_resume_key.as_deref(),
            Some("resume/coordinator")
        );
    }

    // ── Session Storage Tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn session_create_and_retrieve() {
        let (store, _dir) = temp_store();
        let session_id = "test-session-001";
        let key = format!("session/{}", session_id);

        let session_data = serde_json::json!({
            "meta": {
                "session_id": session_id,
                "app_id": "app-001",
                "created_at": "2026-03-18T00:00:00Z",
                "updated_at": "2026-03-18T00:00:00Z",
                "message_count": 1,
                "title": "Test session",
                "status": "running"
            },
            "messages": [{"role": "user", "content": "hello"}],
            "turns": []
        });

        let data = serde_json::to_vec(&session_data).unwrap();
        store.set(&key, &data).await.unwrap();

        let retrieved = store.get(&key).await.unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&retrieved).unwrap();
        assert_eq!(parsed["meta"]["status"], "running");
        assert_eq!(parsed["meta"]["session_id"], session_id);
    }

    #[tokio::test]
    async fn session_status_update() {
        let (store, _dir) = temp_store();
        let key = "session/test-002";

        let session = serde_json::json!({
            "meta": {
                "session_id": "test-002",
                "app_id": "app-001",
                "created_at": "2026-03-18T00:00:00Z",
                "updated_at": "2026-03-18T00:00:00Z",
                "message_count": 0,
                "title": "Test",
                "status": "running"
            },
            "messages": [],
            "turns": []
        });
        store
            .set(key, &serde_json::to_vec(&session).unwrap())
            .await
            .unwrap();

        // Update status to completed
        let mut updated: serde_json::Value =
            serde_json::from_slice(&store.get(key).await.unwrap().unwrap()).unwrap();
        updated["meta"]["status"] = serde_json::json!("completed");
        store
            .set(key, &serde_json::to_vec(&updated).unwrap())
            .await
            .unwrap();

        let final_data = store.get(key).await.unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&final_data).unwrap();
        assert_eq!(parsed["meta"]["status"], "completed");
    }

    #[tokio::test]
    async fn session_list_by_prefix() {
        let (store, _dir) = temp_store();

        store.set("session/s1", b"{}").await.unwrap();
        store.set("session/s2", b"{}").await.unwrap();
        store.set("app_sessions/app1/s1", b"s1").await.unwrap();
        store.set("other/key", b"x").await.unwrap();

        let session_keys = store.list_keys("session/").await.unwrap();
        assert_eq!(session_keys.len(), 2);

        let app_keys = store.list_keys("app_sessions/app1/").await.unwrap();
        assert_eq!(app_keys.len(), 1);
    }

    #[tokio::test]
    async fn session_immediate_visibility() {
        let (store, _dir) = temp_store();
        let key = "session/immediate-test";

        // Simulate: user sends message -> session created immediately with running status
        let session = serde_json::json!({
            "meta": {
                "session_id": "immediate-test",
                "app_id": "app-001",
                "created_at": "2026-03-18T00:00:00Z",
                "updated_at": "2026-03-18T00:00:00Z",
                "message_count": 0,
                "title": "User prompt",
                "status": "running"
            },
            "messages": [{"role": "user", "content": "test prompt"}],
            "turns": []
        });
        store
            .set(key, &serde_json::to_vec(&session).unwrap())
            .await
            .unwrap();

        // Verify: session is immediately visible in list
        let keys = store.list_keys("session/").await.unwrap();
        assert!(keys.contains(&key.to_string()));

        // Verify: session has running status
        let data = store.get(key).await.unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&data).unwrap();
        assert_eq!(parsed["meta"]["status"], "running");
        assert_eq!(parsed["messages"][0]["content"], "test prompt");
    }
}
