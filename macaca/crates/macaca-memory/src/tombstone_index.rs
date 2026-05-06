//! **TombstoneIndex** — async snapshot of deleted memory row ids for consumers that must not revive
//! governance-deleted evidence (for example compiled knowledge digest compilation).
//!
//! ## Design
//! - **Dependency inversion**: storage details live in implementations; callers only depend on id strings
//!   matching [`macaca_proto::MemoryId`] wire form (`Uuid` hyphenated lower hex).
//! - **Shared registry**: [`SharedTombstoneRegistry`] backs the workspace `memory_forget` tool path alongside
//!   [`crate::manager::MemoryManager::forget`] so deletes stay consistent with digest filtering even when no
//!   [`GovernedMemoryFacade`] is mounted.
//! - **Governed facade**: [`GovernanceFacadeTombstones`] proxies the in-facade tombstone journal for tests and
//!   future integrations that wrap memory behind governance.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryId};
use tokio::sync::RwLock;

use crate::governance::facade::GovernedMemoryFacade;
use crate::governance::promotion::MemoryPromotionPolicy;

/// Async source of tombstoned memory identifiers (canonical `MemoryId` string encoding).
#[async_trait]
pub trait TombstoneIndex: Send + Sync {
    /// Sorted order is unspecified; callers should build a [`HashSet`] when they need frequent membership checks.
    async fn tombstoned_memory_id_strings(&self) -> MacacaResult<Vec<String>>;
}

/// No-op index: always reports an empty tombstone set.
#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyTombstoneIndex;

#[async_trait]
impl TombstoneIndex for EmptyTombstoneIndex {
    async fn tombstoned_memory_id_strings(&self) -> MacacaResult<Vec<String>> {
        Ok(Vec::new())
    }
}

/// In-memory registry keyed by [`MemoryId`], typically shared between digest compilation and deletion tools.
#[derive(Clone, Debug)]
pub struct SharedTombstoneRegistry {
    ids: Arc<RwLock<HashSet<MemoryId>>>,
}

impl SharedTombstoneRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ids: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Records one deleted memory identity (idempotent insert).
    pub async fn record(&self, id: MemoryId) {
        self.ids.write().await.insert(id);
    }
}

impl Default for SharedTombstoneRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TombstoneIndex for SharedTombstoneRegistry {
    async fn tombstoned_memory_id_strings(&self) -> MacacaResult<Vec<String>> {
        let g = self.ids.read().await;
        Ok(g.iter().map(|m| m.0.to_string()).collect())
    }
}

/// Combines multiple tombstone sources (for example workspace registry + governance facade) with
/// deterministic de-duplication.
#[derive(Clone)]
pub struct MergedTombstoneIndex {
    parts: Arc<Vec<Arc<dyn TombstoneIndex>>>,
}

impl MergedTombstoneIndex {
    #[must_use]
    pub fn new(parts: Vec<Arc<dyn TombstoneIndex>>) -> Self {
        Self {
            parts: Arc::new(parts),
        }
    }
}

#[async_trait]
impl TombstoneIndex for MergedTombstoneIndex {
    async fn tombstoned_memory_id_strings(&self) -> MacacaResult<Vec<String>> {
        let mut acc: Vec<String> = Vec::new();
        for p in self.parts.iter() {
            let mut slice = p.tombstoned_memory_id_strings().await?;
            acc.append(&mut slice);
        }
        acc.sort_unstable();
        acc.dedup();
        Ok(acc)
    }
}

/// Reads tombstones recorded by [`GovernedMemoryFacade`] (governance decorator journal).
#[derive(Clone)]
pub struct GovernanceFacadeTombstones<P>
where
    P: MemoryPromotionPolicy + Send + Sync + 'static,
{
    inner: Arc<GovernedMemoryFacade<P>>,
}

impl<P> GovernanceFacadeTombstones<P>
where
    P: MemoryPromotionPolicy + Send + Sync + 'static,
{
    #[must_use]
    pub fn new(inner: Arc<GovernedMemoryFacade<P>>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<P> TombstoneIndex for GovernanceFacadeTombstones<P>
where
    P: MemoryPromotionPolicy + Send + Sync + 'static,
{
    async fn tombstoned_memory_id_strings(&self) -> MacacaResult<Vec<String>> {
        Ok(self
            .inner
            .tombstones()
            .await
            .into_iter()
            .map(|row| row.memory_id.0.to_string())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::MemoryId;

    #[tokio::test]
    async fn merged_index_unions_sorted_unique() {
        let a = SharedTombstoneRegistry::new();
        let b = SharedTombstoneRegistry::new();
        let id1 = MemoryId::new();
        let id2 = MemoryId::new();
        a.record(id1).await;
        b.record(id1).await;
        b.record(id2).await;
        let merged =
            MergedTombstoneIndex::new(vec![Arc::new(a) as Arc<dyn TombstoneIndex>, Arc::new(b)]);
        let rows = merged.tombstoned_memory_id_strings().await.unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn shared_registry_reports_inserted_ids() {
        let reg = SharedTombstoneRegistry::new();
        let id = MemoryId::new();
        reg.record(id).await;
        let rows = TombstoneIndex::tombstoned_memory_id_strings(&reg)
            .await
            .unwrap();
        assert!(rows.iter().any(|s| *s == id.0.to_string()));
    }
}
