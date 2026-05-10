use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;
use tracing::debug;

use macaca_proto::{AgentId, MacacaError, MacacaResult, MemoryEntry, MemoryId};

use crate::store::MemoryStore;

/// File-backed memory store.
/// Each entry is stored as `{base_path}/{memory_id}.json`.
pub struct FileMemory {
    base_path: PathBuf,
}

impl FileMemory {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    async fn ensure_dir(&self) -> MacacaResult<()> {
        fs::create_dir_all(&self.base_path)
            .await
            .map_err(MacacaError::Io)
    }

    fn entry_path(&self, id: &MemoryId) -> PathBuf {
        self.base_path.join(format!("{}.json", id.0))
    }

    /// Load all entries from disk, ignoring files that fail to parse.
    async fn load_all(&self) -> MacacaResult<Vec<MemoryEntry>> {
        self.ensure_dir().await?;
        let mut dir = fs::read_dir(&self.base_path)
            .await
            .map_err(MacacaError::Io)?;

        let mut entries = Vec::new();
        while let Some(entry) = dir.next_entry().await.map_err(MacacaError::Io)? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            match fs::read_to_string(&path).await {
                Ok(data) => match serde_json::from_str::<MemoryEntry>(&data) {
                    Ok(mem) => entries.push(mem),
                    Err(e) => debug!(?path, "file_memory: failed to parse entry: {e}"),
                },
                Err(e) => debug!(?path, "file_memory: failed to read entry: {e}"),
            }
        }
        Ok(entries)
    }
}

#[async_trait]
impl MemoryStore for FileMemory {
    async fn store(&self, entry: MemoryEntry) -> MacacaResult<MemoryId> {
        self.ensure_dir().await?;
        let id = entry.id;
        let data = serde_json::to_string_pretty(&entry)?;
        let path = self.entry_path(&id);
        fs::write(&path, data).await.map_err(MacacaError::Io)?;
        Ok(id)
    }

    async fn retrieve(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        let query_lower = query.to_lowercase();
        let mut entries: Vec<MemoryEntry> = self
            .load_all()
            .await?
            .into_iter()
            .filter(|e| e.content.to_lowercase().contains(&query_lower))
            .collect();
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries.truncate(limit);
        Ok(entries)
    }

    async fn get(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>> {
        let path = self.entry_path(id);
        match fs::read_to_string(&path).await {
            Ok(data) => {
                let entry = serde_json::from_str(&data)?;
                Ok(Some(entry))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(MacacaError::Io(e)),
        }
    }

    async fn delete(&self, id: &MemoryId) -> MacacaResult<()> {
        let path = self.entry_path(id);
        match fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(MacacaError::Io(e)),
        }
    }

    async fn list(
        &self,
        agent_id: Option<&AgentId>,
        limit: usize,
    ) -> MacacaResult<Vec<MemoryEntry>> {
        let mut entries: Vec<MemoryEntry> = self
            .load_all()
            .await?
            .into_iter()
            .filter(|e| agent_id.map_or(true, |aid| e.agent_id.as_ref() == Some(aid)))
            .collect();
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries.truncate(limit);
        Ok(entries)
    }
}

#[async_trait]
impl crate::snapshot::MemorySnapshotStore for FileMemory {
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
    use tempfile::TempDir;

    fn make_entry(content: &str) -> MemoryEntry {
        MemoryEntry {
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
    async fn store_and_get() {
        let dir = TempDir::new().unwrap();
        let mem = FileMemory::new(dir.path().to_path_buf());
        let entry = make_entry("persisted content");
        let id = mem.store(entry).await.unwrap();
        let got = mem.get(&id).await.unwrap().unwrap();
        assert_eq!(got.content, "persisted content");
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let dir = TempDir::new().unwrap();
        let mem = FileMemory::new(dir.path().to_path_buf());
        let result = mem.get(&MemoryId::new()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn retrieve_by_substring() {
        let dir = TempDir::new().unwrap();
        let mem = FileMemory::new(dir.path().to_path_buf());
        mem.store(make_entry("rust programming")).await.unwrap();
        mem.store(make_entry("python scripting")).await.unwrap();

        let results = mem.retrieve("rust", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("rust"));
    }

    #[tokio::test]
    async fn retrieve_case_insensitive() {
        let dir = TempDir::new().unwrap();
        let mem = FileMemory::new(dir.path().to_path_buf());
        mem.store(make_entry("Rust Programming")).await.unwrap();

        let results = mem.retrieve("rust", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn delete_entry() {
        let dir = TempDir::new().unwrap();
        let mem = FileMemory::new(dir.path().to_path_buf());
        let id = mem.store(make_entry("to be deleted")).await.unwrap();
        mem.delete(&id).await.unwrap();
        assert!(mem.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_missing_is_ok() {
        let dir = TempDir::new().unwrap();
        let mem = FileMemory::new(dir.path().to_path_buf());
        mem.delete(&MemoryId::new()).await.unwrap();
    }

    #[tokio::test]
    async fn list_by_agent() {
        let dir = TempDir::new().unwrap();
        let mem = FileMemory::new(dir.path().to_path_buf());
        let agent = AgentId::new();
        let mut e1 = make_entry("agent entry");
        e1.agent_id = Some(agent);
        let e2 = make_entry("other entry");
        mem.store(e1).await.unwrap();
        mem.store(e2).await.unwrap();

        let results = mem.list(Some(&agent), 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "agent entry");
    }

    #[tokio::test]
    async fn retrieve_limit_respected() {
        let dir = TempDir::new().unwrap();
        let mem = FileMemory::new(dir.path().to_path_buf());
        for i in 0..5 {
            mem.store(make_entry(&format!("entry {i}"))).await.unwrap();
        }
        let results = mem.retrieve("entry", 3).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn persists_across_instances() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();

        let entry = make_entry("durable");
        let id = {
            let mem = FileMemory::new(path.clone());
            mem.store(entry).await.unwrap()
        };

        // Re-open with a new instance.
        let mem2 = FileMemory::new(path);
        let got = mem2.get(&id).await.unwrap().unwrap();
        assert_eq!(got.content, "durable");
    }
}
