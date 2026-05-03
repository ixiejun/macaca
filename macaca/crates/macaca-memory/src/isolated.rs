//! `IsolatedMemoryManager` — per-agent scoped memory across all three layers.
//!
//! Enforces strict isolation: an agent can only access its own memories.
//! Directory structure for files: `{base_path}/{app_id}/{agent_id}/{memory_id}.json`
//! Milvus multi-tenancy: Application→Database, Agent→Collection.

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use tracing::debug;

use crate::store::MemoryQueryContext;
use macaca_proto::{AgentId, ApplicationId, MacacaResult, MemoryEntry, MemoryId};

use crate::file::FileMemory;
use crate::session::SessionMemory;
use crate::store::{EmbeddingProvider, MemoryRetriever, MemoryStore, VectorStore};

/// Per-agent isolated memory manager.
///
/// All operations are scoped to a specific `(app_id, agent_id)` pair.
/// The agent cannot read or write another agent's memories.
pub struct IsolatedMemoryManager<V: VectorStore, E: EmbeddingProvider> {
    agent_id: AgentId,
    app_id: ApplicationId,
    session: SessionMemory,
    file: FileMemory,
    vector: Option<V>,
    embedding: Option<E>,
}

impl<V: VectorStore, E: EmbeddingProvider> IsolatedMemoryManager<V, E> {
    /// Create an isolated memory manager for a specific agent.
    ///
    /// - `base_path`: root directory for file storage
    /// - Files stored at `{base_path}/{app_id}/{agent_id}/{memory_id}.json`
    /// - Session memory scoped by tagging entries with `agent_id`
    /// - Vector store should already be scoped to the agent's collection
    pub fn new(
        app_id: ApplicationId,
        agent_id: AgentId,
        base_path: PathBuf,
        session_ttl: Duration,
        vector: Option<V>,
        embedding: Option<E>,
    ) -> Self {
        // Scoped file path: {base}/{app_id}/{agent_id}/
        let scoped_path = base_path
            .join(app_id.0.to_string())
            .join(agent_id.0.to_string());

        Self {
            agent_id,
            app_id,
            session: SessionMemory::new(session_ttl),
            file: FileMemory::new(scoped_path),
            vector,
            embedding,
        }
    }

    /// The agent this manager is scoped to.
    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    /// The application this manager belongs to.
    pub fn app_id(&self) -> ApplicationId {
        self.app_id
    }

    async fn store_entry(&self, mut entry: MemoryEntry) -> MacacaResult<MemoryId> {
        // Force agent_id — prevent cross-agent contamination.
        entry.agent_id = Some(self.agent_id);

        let id = entry.id;

        // Store in session and file concurrently.
        let (s_res, f_res) = tokio::join!(
            self.session.store(entry.clone()),
            self.file.store(entry.clone())
        );
        s_res?;
        f_res?;

        // Optionally embed and store in vector layer.
        if let (Some(vec_store), Some(embed)) = (&self.vector, &self.embedding) {
            match embed.embed(vec![entry.content.clone()]).await {
                Ok(vectors) if !vectors.is_empty() => {
                    let vector = vectors.into_iter().next().unwrap();
                    let payload = serde_json::json!({
                        "memory_id": id.0.to_string(),
                        "agent_id": self.agent_id.0.to_string(),
                        "content": entry.content,
                        "layer": format!("{:?}", entry.layer),
                    });
                    if let Err(e) = vec_store.upsert(&id.0.to_string(), vector, payload).await {
                        debug!("isolated_memory: vector upsert failed: {e}");
                    }
                }
                Ok(_) => {}
                Err(e) => debug!("isolated_memory: embed failed: {e}"),
            }
        }

        Ok(id)
    }

    async fn retrieve_entries(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        let agent_id = self.agent_id;

        // Session: filter by agent_id via list
        let (session_res, file_res) = tokio::join!(
            self.session.list(Some(&agent_id), limit),
            self.file.retrieve(query, limit)
        );

        let mut seen = std::collections::HashMap::new();

        // Session results (already agent-scoped via list filter)
        for entry in session_res? {
            if entry.content.to_lowercase().contains(&query.to_lowercase()) {
                seen.entry(entry.id).or_insert(entry);
            }
        }

        // File results (already agent-scoped by directory)
        for entry in file_res? {
            seen.entry(entry.id).or_insert(entry);
        }

        // Vector layer (already agent-scoped by collection)
        if let (Some(vec_store), Some(embed)) = (&self.vector, &self.embedding) {
            match embed.embed(vec![query.to_string()]).await {
                Ok(vectors) if !vectors.is_empty() => {
                    let qvec = vectors.into_iter().next().unwrap();
                    match vec_store.search(qvec, limit).await {
                        Ok(hits) => {
                            for hit in hits {
                                if let Some(mid_str) =
                                    hit.payload.get("memory_id").and_then(|v| v.as_str())
                                {
                                    if let Ok(uuid) = uuid::Uuid::parse_str(mid_str) {
                                        let mid = MemoryId(uuid);
                                        if !seen.contains_key(&mid) {
                                            if let Ok(Some(entry)) = self.file.get(&mid).await {
                                                seen.insert(mid, entry);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => debug!("isolated_memory: vector search failed: {e}"),
                    }
                }
                Ok(_) => {}
                Err(e) => debug!("isolated_memory: embed for retrieve failed: {e}"),
            }
        }

        let mut results: Vec<MemoryEntry> = seen.into_values().collect();
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        results.truncate(limit);
        Ok(results)
    }

    async fn list_entries(&self, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        // File layer is already directory-scoped, so list all.
        self.file.list(None, limit).await
    }

    async fn get_entry(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>> {
        // Check file layer (directory-scoped to this agent).
        if let Some(entry) = self.file.get(id).await? {
            return Ok(Some(entry));
        }
        // Fall back to session layer, but verify agent_id.
        if let Some(entry) = self.session.get(id).await? {
            if entry.agent_id.as_ref() == Some(&self.agent_id) {
                return Ok(Some(entry));
            }
        }
        Ok(None)
    }

    async fn delete_entry(&self, id: &MemoryId) -> MacacaResult<()> {
        let (s_res, f_res) = tokio::join!(self.session.delete(id), self.file.delete(id));
        s_res?;
        f_res?;

        if let Some(vec_store) = &self.vector {
            if let Err(e) = vec_store.delete(&id.0.to_string()).await {
                debug!("isolated_memory: vector delete failed: {e}");
            }
        }
        Ok(())
    }

    pub async fn remember_text(
        &self,
        input: crate::facade::RememberText,
    ) -> MacacaResult<MemoryId> {
        let entry = MemoryEntry {
            id: MemoryId::new(),
            layer: input.layer,
            content: input.content,
            metadata: input.metadata,
            agent_id: input.agent_id,
            created_at: chrono::Utc::now(),
            expires_at: None,
        };
        self.store_entry(entry).await
    }

    pub async fn recall(
        &self,
        query: crate::facade::RecallQuery,
    ) -> MacacaResult<crate::facade::RecallResult> {
        self.retrieve_entries(&query.query, query.limit)
            .await
            .map(crate::facade::RecallResult::new)
    }

    pub async fn list_memories(&self, limit: usize) -> MacacaResult<crate::facade::RecallResult> {
        self.list_entries(limit)
            .await
            .map(crate::facade::RecallResult::new)
    }

    pub async fn get_memory(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>> {
        self.get_entry(id).await
    }

    pub async fn forget(&self, input: crate::facade::ForgetMemory) -> MacacaResult<()> {
        self.delete_entry(&input.id).await
    }

    /// Store a memory entry through the legacy direct manager API.
    #[deprecated(note = "use IsolatedMemoryManager::remember_text for new text memories")]
    pub async fn store(&self, entry: MemoryEntry) -> MacacaResult<MemoryId> {
        self.store_entry(entry).await
    }

    /// Retrieve scoped entries through the legacy direct manager API.
    #[deprecated(note = "use IsolatedMemoryManager::recall with RecallQuery")]
    pub async fn retrieve(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        self.retrieve_entries(query, limit).await
    }

    /// List scoped entries through the legacy direct manager API.
    #[deprecated(note = "use IsolatedMemoryManager::list_memories")]
    pub async fn list(&self, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        self.list_entries(limit).await
    }

    /// Get a scoped entry through the legacy direct manager API.
    #[deprecated(note = "use IsolatedMemoryManager::get_memory")]
    pub async fn get(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>> {
        self.get_entry(id).await
    }

    /// Delete a scoped entry through the legacy direct manager API.
    #[deprecated(note = "use IsolatedMemoryManager::forget")]
    pub async fn delete(&self, id: &MemoryId) -> MacacaResult<()> {
        self.delete_entry(id).await
    }
}

#[async_trait]
impl<V: VectorStore, E: EmbeddingProvider> MemoryRetriever for IsolatedMemoryManager<V, E> {
    async fn auto_retrieve(&self, context: &MemoryQueryContext) -> MacacaResult<Vec<MemoryEntry>> {
        let history_snippet = context
            .history
            .iter()
            .rev()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let query = format!("{} {}", context.description, history_snippet);
        self.retrieve_entries(query.trim(), 10).await
    }
}

/// Convenience type alias for testing.
pub type TestIsolatedMemoryManager =
    IsolatedMemoryManager<crate::vector::InMemoryVectorStore, crate::embedding::MockEmbedding>;

// ── Milvus multi-tenancy helpers ─────────────────────────────────────────────

/// Helper functions for Milvus multi-tenancy naming conventions.
pub mod naming {
    use macaca_proto::{AgentId, ApplicationId};

    /// Database name for an application: `app_{uuid}` (truncated, alphanumeric).
    pub fn app_database_name(app_id: &ApplicationId) -> String {
        // Milvus database names: alphanumeric + underscore, max 64 chars.
        format!("app_{}", app_id.0.as_simple())
    }

    /// Collection name for an agent: `agent_{uuid}`.
    pub fn agent_collection_name(agent_id: &AgentId) -> String {
        format!("agent_{}", agent_id.0.as_simple())
    }

    /// Collection name for shared memory within an application.
    pub fn shared_collection_name() -> String {
        "shared_memory".into()
    }
}

#[cfg(test)]
mod tests {
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

        let mgr: IsolatedMemoryManager<InMemoryVectorStore, MockEmbedding> =
            IsolatedMemoryManager::new(
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
}
