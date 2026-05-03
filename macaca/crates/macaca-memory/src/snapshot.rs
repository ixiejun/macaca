use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macaca_proto::{MacacaResult, MemoryEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub captured_at: DateTime<Utc>,
    pub entries: Vec<MemoryEntry>,
}

impl MemorySnapshot {
    pub fn new(entries: Vec<MemoryEntry>) -> Self {
        Self {
            captured_at: Utc::now(),
            entries,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[async_trait]
pub trait MemorySnapshotStore {
    async fn snapshot(&self, limit: usize) -> MacacaResult<MemorySnapshot>;
    async fn replay_snapshot(&self, snapshot: &MemorySnapshot) -> MacacaResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::FileMemory;
    use crate::store::MemoryStore;
    use chrono::Utc;
    use macaca_proto::{MemoryId, MemoryLayer};
    use tempfile::TempDir;

    fn entry(content: &str) -> macaca_proto::MemoryEntry {
        macaca_proto::MemoryEntry {
            id: MemoryId::new(),
            layer: MemoryLayer::File,
            content: content.to_string(),
            metadata: serde_json::Value::Null,
            agent_id: None,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    #[tokio::test]
    async fn file_snapshot_replays_entries() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();
        let src = FileMemory::new(src_dir.path().to_path_buf());
        let dst = FileMemory::new(dst_dir.path().to_path_buf());
        let first = entry("snapshot one");
        let second = entry("snapshot two");

        src.store(first.clone()).await.unwrap();
        src.store(second.clone()).await.unwrap();

        let snapshot = src.snapshot(10).await.unwrap();
        dst.replay_snapshot(&snapshot).await.unwrap();
        let restored = dst.list(None, 10).await.unwrap();

        assert_eq!(snapshot.len(), 2);
        assert!(restored.iter().any(|entry| entry.id == first.id));
        assert!(restored.iter().any(|entry| entry.id == second.id));
    }
}
