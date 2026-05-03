use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use tracing::debug;

use macaca_proto::{AgentId, MacacaError, MacacaResult, MemoryEntry, MemoryId};

use crate::store::MemoryStore;

struct Inner {
    entries: HashMap<MemoryId, MemoryEntry>,
}

/// In-memory store with TTL eviction.
pub struct SessionMemory {
    inner: Arc<RwLock<Inner>>,
    ttl: Duration,
}

impl SessionMemory {
    pub fn new(ttl: Duration) -> Self {
        let inner = Arc::new(RwLock::new(Inner {
            entries: HashMap::new(),
        }));

        // Spawn background eviction task.
        let evict_inner = Arc::clone(&inner);
        tokio::spawn(async move {
            let interval = tokio::time::interval(Duration::from_secs(30));
            tokio::pin!(interval);
            loop {
                interval.as_mut().tick().await;
                let now = Utc::now();
                let mut guard = evict_inner.write().await;
                let before = guard.entries.len();
                guard
                    .entries
                    .retain(|_, e| e.expires_at.map_or(true, |exp| exp > now));
                let evicted = before - guard.entries.len();
                if evicted > 0 {
                    debug!(evicted, "session memory: evicted expired entries");
                }
            }
        });

        Self { inner, ttl }
    }

    /// Evict all expired entries immediately (useful for testing).
    pub async fn evict_expired(&self) {
        let now = Utc::now();
        let mut guard = self.inner.write().await;
        guard
            .entries
            .retain(|_, e| e.expires_at.map_or(true, |exp| exp > now));
    }
}

#[async_trait]
impl MemoryStore for SessionMemory {
    async fn store(&self, mut entry: MemoryEntry) -> MacacaResult<MemoryId> {
        // Set expiry from TTL if not already set.
        if entry.expires_at.is_none() {
            entry.expires_at = Some(
                Utc::now()
                    + chrono::Duration::from_std(self.ttl)
                        .map_err(|e| MacacaError::Memory(e.to_string()))?,
            );
        }
        let id = entry.id;
        let mut guard = self.inner.write().await;
        guard.entries.insert(id, entry);
        Ok(id)
    }

    async fn retrieve(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        let now = Utc::now();
        let guard = self.inner.read().await;
        let query_lower = query.to_lowercase();
        let mut results: Vec<MemoryEntry> = guard
            .entries
            .values()
            .filter(|e| {
                let not_expired = e.expires_at.map_or(true, |exp| exp > now);
                let matches = e.content.to_lowercase().contains(&query_lower);
                not_expired && matches
            })
            .cloned()
            .collect();
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        results.truncate(limit);
        Ok(results)
    }

    async fn get(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>> {
        let now = Utc::now();
        let guard = self.inner.read().await;
        let entry = guard.entries.get(id).and_then(|e| {
            if e.expires_at.map_or(true, |exp| exp > now) {
                Some(e.clone())
            } else {
                None
            }
        });
        Ok(entry)
    }

    async fn delete(&self, id: &MemoryId) -> MacacaResult<()> {
        let mut guard = self.inner.write().await;
        guard.entries.remove(id);
        Ok(())
    }

    async fn list(
        &self,
        agent_id: Option<&AgentId>,
        limit: usize,
    ) -> MacacaResult<Vec<MemoryEntry>> {
        let now = Utc::now();
        let guard = self.inner.read().await;
        let mut results: Vec<MemoryEntry> = guard
            .entries
            .values()
            .filter(|e| {
                let not_expired = e.expires_at.map_or(true, |exp| exp > now);
                let matches_agent = agent_id.map_or(true, |aid| e.agent_id.as_ref() == Some(aid));
                not_expired && matches_agent
            })
            .cloned()
            .collect();
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        results.truncate(limit);
        Ok(results)
    }
}

#[async_trait]
impl crate::snapshot::MemorySnapshotStore for SessionMemory {
    async fn snapshot(&self, limit: usize) -> MacacaResult<crate::snapshot::MemorySnapshot> {
        let entries = self.list(None, limit).await?;
        Ok(crate::snapshot::MemorySnapshot::new(entries))
    }

    async fn replay_snapshot(
        &self,
        snapshot: &crate::snapshot::MemorySnapshot,
    ) -> MacacaResult<()> {
        for entry in snapshot.entries.iter().cloned() {
            self.store(entry).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{MemoryId, MemoryLayer};

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
    async fn store_and_get() {
        let mem = SessionMemory::new(Duration::from_secs(60));
        let entry = make_entry("hello world");
        let id = mem.store(entry.clone()).await.unwrap();
        let retrieved = mem.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "hello world");
    }

    #[tokio::test]
    async fn retrieve_by_substring() {
        let mem = SessionMemory::new(Duration::from_secs(60));
        mem.store(make_entry("the quick brown fox")).await.unwrap();
        mem.store(make_entry("lazy dog")).await.unwrap();

        let results = mem.retrieve("quick", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("quick"));
    }

    #[tokio::test]
    async fn retrieve_case_insensitive() {
        let mem = SessionMemory::new(Duration::from_secs(60));
        mem.store(make_entry("Hello World")).await.unwrap();

        let results = mem.retrieve("hello", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn delete_entry() {
        let mem = SessionMemory::new(Duration::from_secs(60));
        let id = mem.store(make_entry("to delete")).await.unwrap();
        mem.delete(&id).await.unwrap();
        assert!(mem.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn ttl_eviction() {
        let mem = SessionMemory::new(Duration::from_millis(1));
        let entry = make_entry("ephemeral");
        let id = mem.store(entry).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        mem.evict_expired().await;
        assert!(mem.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_by_agent() {
        let mem = SessionMemory::new(Duration::from_secs(60));
        let agent = AgentId::new();
        let mut e1 = make_entry("for agent");
        e1.agent_id = Some(agent);
        let mut e2 = make_entry("for other");
        e2.agent_id = Some(AgentId::new());
        mem.store(e1).await.unwrap();
        mem.store(e2).await.unwrap();

        let results = mem.list(Some(&agent), 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "for agent");
    }

    #[tokio::test]
    async fn retrieve_limit_respected() {
        let mem = SessionMemory::new(Duration::from_secs(60));
        for i in 0..5 {
            mem.store(make_entry(&format!("entry {i}"))).await.unwrap();
        }
        let results = mem.retrieve("entry", 3).await.unwrap();
        assert_eq!(results.len(), 3);
    }
}
