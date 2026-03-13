use std::collections::HashMap;

use async_trait::async_trait;
use tracing::debug;

use macaca_proto::{AgentId, MacacaResult, MemoryEntry, MemoryId, TaskContext};

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

    /// Store an entry in session and file layers; also embed + upsert into the vector
    /// layer if both vector store and embedding provider are configured.
    pub async fn store(&self, entry: MemoryEntry) -> MacacaResult<MemoryId> {
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

    /// Retrieve entries from all available layers, deduplicated by MemoryId.
    pub async fn retrieve(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
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
                                if let Some(mid_str) = hit.payload.get("memory_id").and_then(|v| v.as_str()) {
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

    /// List entries from the file layer (persistent).
    pub async fn list(&self, agent_id: Option<&AgentId>, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        self.file.list(agent_id, limit).await
    }
}

#[async_trait]
impl<V: VectorStore, E: EmbeddingProvider> MemoryRetriever for MemoryManager<V, E> {
    async fn auto_retrieve(&self, context: &TaskContext) -> MacacaResult<Vec<MemoryEntry>> {
        // Build a composite query from description and recent history items.
        let history_snippet = context.history.iter().rev().take(3).cloned().collect::<Vec<_>>().join(" ");
        let query = format!("{} {}", context.description, history_snippet);
        self.retrieve(query.trim(), 10).await
    }
}

/// Convenience type alias for a MemoryManager that uses the in-memory test
/// implementations. Useful for tests in other crates.
pub type TestMemoryManager = MemoryManager<InMemoryVectorStore, MockEmbedding>;

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{AgentId, MemoryLayer, MemoryId, TaskId};
    use chrono::Utc;
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
        mgr.store(make_entry("agent memory content")).await.unwrap();
        let results = mgr.retrieve("agent memory", 10).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|e| e.content.contains("agent memory")));
    }

    #[tokio::test]
    async fn deduplication_across_layers() {
        let dir = TempDir::new().unwrap();
        let mgr = make_manager(&dir);
        mgr.store(make_entry("duplicate content")).await.unwrap();
        let results = mgr.retrieve("duplicate", 10).await.unwrap();
        // Same entry stored in both layers should appear only once.
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn auto_retrieve_uses_description() {
        let dir = TempDir::new().unwrap();
        let mgr = make_manager(&dir);
        mgr.store(make_entry("rust async programming")).await.unwrap();
        mgr.store(make_entry("unrelated topic")).await.unwrap();

        let ctx = TaskContext {
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
        let id = mgr.store(make_entry("no vectors")).await.unwrap();
        let results = mgr.retrieve("no vectors", 10).await.unwrap();
        assert!(results.iter().any(|e| e.id == id));
    }
}
