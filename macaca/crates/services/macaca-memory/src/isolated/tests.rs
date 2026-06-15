use super::*;
use crate::embedding::MockEmbedding;
use crate::vector::InMemoryVectorStore;
use chrono::Utc;
use macaca_proto::{MemoryId, MemoryLayer};
use tempfile::TempDir;

fn make_isolated(dir: &TempDir) -> IsolatedMemoryManager<InMemoryVectorStore, MockEmbedding> {
    let app_id = ApplicationId::new();
    let agent_id = AgentId::new();
    IsolatedMemoryManager::new(
        app_id,
        agent_id,
        dir.path().to_path_buf(),
        Duration::from_secs(60),
        Some(InMemoryVectorStore::new()),
        Some(MockEmbedding::default()),
    )
}

fn make_entry(content: &str) -> MemoryEntry {
    MemoryEntry {
        id: MemoryId::new(),
        layer: MemoryLayer::Session,
        content: content.to_string(),
        metadata: serde_json::Value::Null,
        agent_id: None,
        created_at: Utc::now(),
        expires_at: None,
    }
}

#[tokio::test]
async fn store_forces_agent_id() {
    let dir = TempDir::new().unwrap();
    let mgr = make_isolated(&dir);
    let entry = make_entry("test content");
    assert!(entry.agent_id.is_none()); // No agent_id initially.

    let id = mgr.store_entry(entry).await.unwrap();
    let got = mgr.get_entry(&id).await.unwrap().unwrap();
    assert_eq!(got.agent_id, Some(mgr.agent_id()));
}

#[tokio::test]
async fn store_and_retrieve() {
    let dir = TempDir::new().unwrap();
    let mgr = make_isolated(&dir);
    mgr.store_entry(make_entry("isolated memory content"))
        .await
        .unwrap();
    let results = mgr.retrieve_entries("isolated", 10).await.unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|e| e.content.contains("isolated")));
}

#[tokio::test]
async fn isolated_facade_forces_agent_scope() {
    let dir = TempDir::new().unwrap();
    let mgr = make_isolated(&dir);
    let id = mgr
        .remember_text(crate::facade::RememberText::new("isolated facade memory"))
        .await
        .unwrap();
    let entry = mgr.get_memory(&id).await.unwrap().unwrap();

    assert_eq!(entry.agent_id, Some(mgr.agent_id()));
}

#[tokio::test]
async fn different_agents_are_isolated() {
    let dir = TempDir::new().unwrap();
    let app_id = ApplicationId::new();

    // Agent A
    let agent_a = AgentId::new();
    let mgr_a: IsolatedMemoryManager<InMemoryVectorStore, MockEmbedding> =
        IsolatedMemoryManager::new(
            app_id,
            agent_a,
            dir.path().to_path_buf(),
            Duration::from_secs(60),
            None,
            None,
        );

    // Agent B
    let agent_b = AgentId::new();
    let mgr_b: IsolatedMemoryManager<InMemoryVectorStore, MockEmbedding> =
        IsolatedMemoryManager::new(
            app_id,
            agent_b,
            dir.path().to_path_buf(),
            Duration::from_secs(60),
            None,
            None,
        );

    // Agent A stores a memory.
    mgr_a
        .store_entry(make_entry("secret A data"))
        .await
        .unwrap();

    // Agent B stores a memory.
    mgr_b
        .store_entry(make_entry("secret B data"))
        .await
        .unwrap();

    // Agent A can only see its own memories.
    let a_results = mgr_a.retrieve_entries("secret", 10).await.unwrap();
    assert_eq!(a_results.len(), 1);
    assert!(a_results[0].content.contains("A data"));

    // Agent B can only see its own memories.
    let b_results = mgr_b.retrieve_entries("secret", 10).await.unwrap();
    assert_eq!(b_results.len(), 1);
    assert!(b_results[0].content.contains("B data"));
}

#[tokio::test]
async fn delete_entry() {
    let dir = TempDir::new().unwrap();
    let mgr = make_isolated(&dir);
    let id = mgr.store_entry(make_entry("to delete")).await.unwrap();
    mgr.delete_entry(&id).await.unwrap();
    assert!(mgr.get_entry(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn list_entries() {
    let dir = TempDir::new().unwrap();
    let mgr = make_isolated(&dir);
    mgr.store_entry(make_entry("entry 1")).await.unwrap();
    mgr.store_entry(make_entry("entry 2")).await.unwrap();
    let results = mgr.list_entries(10).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn naming_conventions() {
    let app_id = ApplicationId::new();
    let agent_id = AgentId::new();

    let db = naming::app_database_name(&app_id);
    assert!(db.starts_with("app_"));
    assert!(db.len() <= 64);

    let coll = naming::agent_collection_name(&agent_id);
    assert!(coll.starts_with("agent_"));

    let shared = naming::shared_collection_name();
    assert_eq!(shared, "shared_memory");
}

#[tokio::test]
async fn file_directory_scoped_by_app_and_agent() {
    let dir = TempDir::new().unwrap();
    let app_id = ApplicationId::new();
    let agent_id = AgentId::new();

    let mgr: IsolatedMemoryManager<InMemoryVectorStore, MockEmbedding> = IsolatedMemoryManager::new(
        app_id,
        agent_id,
        dir.path().to_path_buf(),
        Duration::from_secs(60),
        None,
        None,
    );

    mgr.store_entry(make_entry("scoped file")).await.unwrap();

    // Verify the file was created in the scoped directory.
    let scoped_dir = dir
        .path()
        .join(app_id.0.to_string())
        .join(agent_id.0.to_string());
    assert!(scoped_dir.exists());
    let files: Vec<_> = std::fs::read_dir(&scoped_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 1);
}
