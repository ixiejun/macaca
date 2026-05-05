use std::collections::HashMap;

use async_trait::async_trait;
use tracing::debug;

use crate::store::MemoryQueryContext;
use macaca_proto::{AgentId, MacacaResult, MemoryEntry, MemoryId};

use crate::embedding::MockEmbedding;
use crate::file::FileMemory;
use crate::session::SessionMemory;
use crate::store::{EmbeddingProvider, MemoryRetriever, MemoryStore, VectorStore};
use crate::vector::InMemoryVectorStore;

/// Combines session, file, and optional vector layers into a unified memory interface.
pub struct MemoryManager<V: VectorStore, E: EmbeddingProvider> {
    session: SessionMemory,
    file: FileMemory,
    vector: Option<V>,
    embedding: Option<E>,
}

impl<V: VectorStore, E: EmbeddingProvider> MemoryManager<V, E> {
    pub fn new(
        session: SessionMemory,
        file: FileMemory,
        vector: Option<V>,
        embedding: Option<E>,
    ) -> Self {
        Self {
            session,
            file,
            vector,
            embedding,
        }
    }

    async fn store_entry(&self, entry: MemoryEntry) -> MacacaResult<MemoryId> {
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
                        "content": entry.content,
                        "layer": format!("{:?}", entry.layer),
                    });
                    if let Err(e) = vec_store.upsert(&id.0.to_string(), vector, payload).await {
                        debug!("memory_manager: vector upsert failed: {e}");
                    }
                }
                Ok(_) => {}
                Err(e) => debug!("memory_manager: embed failed: {e}"),
            }
        }

        Ok(id)
    }

    async fn retrieve_entries(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        let (session_res, file_res) = tokio::join!(
            self.session.retrieve(query, limit),
            self.file.retrieve(query, limit)
        );

        let mut seen: HashMap<MemoryId, MemoryEntry> = HashMap::new();

        for entry in session_res?.into_iter().chain(file_res?.into_iter()) {
            seen.entry(entry.id).or_insert(entry);
        }

        // Query vector layer if available.
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
                                            // Try to load from file layer by ID.
                                            if let Ok(Some(entry)) = self.file.get(&mid).await {
                                                seen.insert(mid, entry);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => debug!("memory_manager: vector search failed: {e}"),
                    }
                }
                Ok(_) => {}
                Err(e) => debug!("memory_manager: embed for retrieve failed: {e}"),
            }
        }

        let mut results: Vec<MemoryEntry> = seen.into_values().collect();
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        results.truncate(limit);
        Ok(results)
    }

    async fn list_entries(
        &self,
        agent_id: Option<&AgentId>,
        limit: usize,
    ) -> MacacaResult<Vec<MemoryEntry>> {
        self.file.list(agent_id, limit).await
    }

    pub async fn get_entry(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>> {
        if let Some(entry) = self.session.get(id).await? {
            return Ok(Some(entry));
        }
        self.file.get(id).await
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

    pub async fn list_memories(
        &self,
        agent_id: Option<&AgentId>,
        limit: usize,
    ) -> MacacaResult<crate::facade::RecallResult> {
        self.list_entries(agent_id, limit)
            .await
            .map(crate::facade::RecallResult::new)
    }

    pub async fn forget(&self, input: crate::facade::ForgetMemory) -> MacacaResult<()> {
        let (s_res, f_res) =
            tokio::join!(self.session.delete(&input.id), self.file.delete(&input.id));
        s_res?;
        f_res?;

        if let Some(vector) = &self.vector {
            if let Err(e) = vector.delete(&input.id.0.to_string()).await {
                debug!("memory_manager: vector delete failed: {e}");
            }
        }
        Ok(())
    }

    /// Store an entry through the legacy direct manager API.
    #[deprecated(note = "use MemoryManager::remember_text for new text memories")]
    pub async fn store(&self, entry: MemoryEntry) -> MacacaResult<MemoryId> {
        self.store_entry(entry).await
    }

    /// Retrieve entries through the legacy direct manager API.
    #[deprecated(note = "use MemoryManager::recall with RecallQuery")]
    pub async fn retrieve(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        self.retrieve_entries(query, limit).await
    }

    /// List persistent entries through the legacy direct manager API.
    #[deprecated(note = "use MemoryManager::list_memories")]
    pub async fn list(
        &self,
        agent_id: Option<&AgentId>,
        limit: usize,
    ) -> MacacaResult<Vec<MemoryEntry>> {
        self.list_entries(agent_id, limit).await
    }
}

#[async_trait]
impl<V: VectorStore, E: EmbeddingProvider> MemoryRetriever for MemoryManager<V, E> {
    async fn auto_retrieve(&self, context: &MemoryQueryContext) -> MacacaResult<Vec<MemoryEntry>> {
        // Build a composite query from description and recent history items.
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

/// Convenience type alias for a MemoryManager that uses the in-memory test
/// implementations. Useful for tests in other crates.
pub type TestMemoryManager = MemoryManager<InMemoryVectorStore, MockEmbedding>;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{AgentId, MemoryId, MemoryLayer, TaskId};
    use std::time::Duration;
    use tempfile::TempDir;

    fn make_manager(dir: &TempDir) -> MemoryManager<InMemoryVectorStore, MockEmbedding> {
        MemoryManager::new(
            SessionMemory::new(Duration::from_secs(60)),
            FileMemory::new(dir.path().to_path_buf()),
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
    async fn store_and_retrieve() {
        let dir = TempDir::new().unwrap();
        let mgr = make_manager(&dir);
        mgr.store_entry(make_entry("agent memory content"))
            .await
            .unwrap();
        let results = mgr.retrieve_entries("agent memory", 10).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|e| e.content.contains("agent memory")));
    }

    #[tokio::test]
    async fn deduplication_across_layers() {
        let dir = TempDir::new().unwrap();
        let mgr = make_manager(&dir);
        mgr.store_entry(make_entry("duplicate content"))
            .await
            .unwrap();
        let results = mgr.retrieve_entries("duplicate", 10).await.unwrap();
        // Same entry stored in both layers should appear only once.
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn auto_retrieve_uses_description() {
        let dir = TempDir::new().unwrap();
        let mgr = make_manager(&dir);
        mgr.store_entry(make_entry("rust async programming"))
            .await
            .unwrap();
        mgr.store_entry(make_entry("unrelated topic"))
            .await
            .unwrap();

        let ctx = MemoryQueryContext {
            task_id: TaskId::new(),
            description: "rust async".to_string(),
            agent_id: AgentId::new(),
            history: vec![],
        };
        let results = mgr.auto_retrieve(&ctx).await.unwrap();
        assert!(results.iter().any(|e| e.content.contains("rust async")));
    }

    #[tokio::test]
    async fn store_without_vector_layer() {
        let dir = TempDir::new().unwrap();
        // No vector/embedding layer.
        let mgr: MemoryManager<InMemoryVectorStore, MockEmbedding> = MemoryManager::new(
            SessionMemory::new(Duration::from_secs(60)),
            FileMemory::new(dir.path().to_path_buf()),
            None,
            None,
        );
        let id = mgr.store_entry(make_entry("no vectors")).await.unwrap();
        let results = mgr.retrieve_entries("no vectors", 10).await.unwrap();
        assert!(results.iter().any(|e| e.id == id));
    }

    #[tokio::test]
    async fn facade_remember_recall_and_forget_text() {
        let dir = TempDir::new().unwrap();
        let mgr = make_manager(&dir);

        let id = mgr
            .remember_text(crate::facade::RememberText::new("facade memory"))
            .await
            .unwrap();
        let result = mgr
            .recall(crate::facade::RecallQuery::new("facade", 10))
            .await
            .unwrap();

        assert!(result.entries.iter().any(|entry| entry.id == id));

        mgr.forget(crate::facade::ForgetMemory { id })
            .await
            .unwrap();
        let result = mgr
            .recall(crate::facade::RecallQuery::new("facade", 10))
            .await
            .unwrap();
        assert!(result.entries.iter().all(|entry| entry.id != id));
    }
}
