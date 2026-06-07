//! Session lineage persistence for compaction successors and forks.
//!
//! `SessionLineageStore` is the **Repository** adapter for lineage DTOs defined
//! in `macaca-proto::context_lineage`. It serializes value objects to the shared
//! `PersistBackend` without importing `macaca-context` service logic, preserving
//! foundation-layer dependency direction mandated by Route C governance.

use std::sync::Arc;

use macaca_proto::{LineageKind, MacacaError, MacacaResult, SessionLineage, TranscriptSegment};
use tracing::{debug, trace};

use crate::PersistBackend;

/// Key namespace prefix for all lineage-related rows in the persistence backend.
const LINEAGE_PREFIX: &str = "context_lineage";

/// Repository for session lineage trees and transcript segment metadata.
///
/// The store writes JSON-encoded [`SessionLineage`] and [`TranscriptSegment`]
/// value objects under deterministic keys so tip resolution and successor
/// counting can be performed without loading full conversation transcripts.
pub struct SessionLineageStore {
    store: Arc<dyn PersistBackend>,
}

impl SessionLineageStore {
    /// Wrap a shared persistence backend as a lineage repository.
    pub fn new(store: Arc<dyn PersistBackend>) -> Self {
        Self { store }
    }

    /// Persist one lineage node and update root tip / child index pointers.
    ///
    /// This method is idempotent at the key level: repeated saves for the same
    /// `session_id` overwrite the prior row, while root tip always reflects the
    /// latest written successor for audit replay convergence.
    pub async fn save_lineage(&self, lineage: &SessionLineage) -> MacacaResult<()> {
        debug!(
            service_id = "persist.lineage",
            session_id = %lineage.session_id,
            root_session_id = %lineage.root_session_id,
            lineage_kind = ?lineage.lineage_kind,
            "persisting session lineage node"
        );
        let data = serde_json::to_vec(lineage).map_err(json_error)?;
        self.store
            .set(&session_key(&lineage.session_id), &data)
            .await?;
        self.store
            .set(
                &root_child_key(&lineage.root_session_id, &lineage.session_id),
                lineage.session_id.as_bytes(),
            )
            .await?;
        self.store
            .set(
                &root_tip_key(&lineage.root_session_id),
                lineage.session_id.as_bytes(),
            )
            .await
    }

    /// Load lineage metadata for one session id, if present.
    pub async fn load_lineage(&self, session_id: &str) -> MacacaResult<Option<SessionLineage>> {
        trace!(
            service_id = "persist.lineage",
            session_id = %session_id,
            "loading session lineage node"
        );
        let Some(data) = self.store.get(&session_key(session_id)).await? else {
            return Ok(None);
        };
        serde_json::from_slice(&data).map(Some).map_err(json_error)
    }

    /// Resolve the latest session id for the lineage tree containing `session_id`.
    ///
    /// When no lineage row exists the input id is returned unchanged so callers
    /// can treat unknown sessions as self-rooted without special cases.
    pub async fn resolve_tip(&self, session_id: &str) -> MacacaResult<String> {
        let Some(lineage) = self.load_lineage(session_id).await? else {
            return Ok(session_id.to_string());
        };
        let Some(data) = self
            .store
            .get(&root_tip_key(&lineage.root_session_id))
            .await?
        else {
            return Ok(session_id.to_string());
        };
        String::from_utf8(data).map_err(|err| MacacaError::Persist(err.to_string()))
    }

    /// Number of compaction successors recorded for the lineage tree containing `session_id`.
    pub async fn count_compaction_successors(&self, session_id: &str) -> MacacaResult<u32> {
        let root = self
            .load_lineage(session_id)
            .await?
            .map(|l| l.root_session_id)
            .unwrap_or_else(|| session_id.to_string());
        let list = self.list_root_lineage(&root).await?;
        Ok(list
            .iter()
            .filter(|l| l.lineage_kind == LineageKind::CompactionSuccessor)
            .count() as u32)
    }

    /// List all lineage nodes indexed under one root session id.
    pub async fn list_root_lineage(
        &self,
        root_session_id: &str,
    ) -> MacacaResult<Vec<SessionLineage>> {
        let mut output = Vec::new();
        let keys = self
            .store
            .list_keys(&root_child_prefix(root_session_id))
            .await?;
        for key in keys {
            let Some(session_id_bytes) = self.store.get(&key).await? else {
                continue;
            };
            let Ok(session_id) = String::from_utf8(session_id_bytes) else {
                continue;
            };
            if let Some(lineage) = self.load_lineage(&session_id).await? {
                output.push(lineage);
            }
        }
        output.sort_by(|a, b| lineage_rank(a.lineage_kind).cmp(&lineage_rank(b.lineage_kind)));
        Ok(output)
    }

    /// Persist one transcript segment row linked to a lineage node.
    pub async fn save_segment(&self, segment: &TranscriptSegment) -> MacacaResult<()> {
        debug!(
            service_id = "persist.lineage",
            segment_id = %segment.segment_id,
            session_id = %segment.session_id,
            "persisting transcript segment metadata"
        );
        let data = serde_json::to_vec(segment).map_err(json_error)?;
        self.store
            .set(&segment_key(&segment.segment_id), &data)
            .await
    }

    /// Load one transcript segment by id, if present.
    pub async fn load_segment(&self, segment_id: &str) -> MacacaResult<Option<TranscriptSegment>> {
        let Some(data) = self.store.get(&segment_key(segment_id)).await? else {
            return Ok(None);
        };
        serde_json::from_slice(&data).map(Some).map_err(json_error)
    }
}

fn session_key(session_id: &str) -> String {
    format!("{LINEAGE_PREFIX}/session/{session_id}")
}

fn root_tip_key(root_session_id: &str) -> String {
    format!("{LINEAGE_PREFIX}/root/{root_session_id}/tip")
}

fn root_child_prefix(root_session_id: &str) -> String {
    format!("{LINEAGE_PREFIX}/root/{root_session_id}/child/")
}

fn root_child_key(root_session_id: &str, session_id: &str) -> String {
    format!("{}{session_id}", root_child_prefix(root_session_id))
}

fn segment_key(segment_id: &str) -> String {
    format!("{LINEAGE_PREFIX}/segment/{segment_id}")
}

/// Stable ordering for lineage listing: root before successors before forks.
fn lineage_rank(kind: LineageKind) -> u8 {
    match kind {
        LineageKind::Root => 0,
        LineageKind::CompactionSuccessor => 1,
        LineageKind::Fork => 2,
        LineageKind::IsolatedChild => 3,
    }
}

fn json_error(error: serde_json::Error) -> MacacaError {
    MacacaError::Json(error)
}

#[cfg(test)]
mod tests {
    use macaca_context::{
        ContextRenderInput, ContextRenderable, ContextSourceKind, ContextSourceReference,
        DefaultPruningPolicy, DefaultSourceRenderer, LineageKind, SessionLineage,
        TranscriptSegment, TrustLevel,
    };

    use super::*;
    use crate::{AppendEventCommand, EventLog, RedbStore};

    fn test_store() -> (SessionLineageStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = RedbStore::open(dir.path().join("lineage.redb")).unwrap();
        (SessionLineageStore::new(Arc::new(store)), dir)
    }

    #[tokio::test]
    async fn lineage_tip_resolves_to_latest_successor() {
        let (store, _dir) = test_store();
        let root = SessionLineage::root("root");
        let child = SessionLineage::successor("root", "root", "child");

        store.save_lineage(&root).await.unwrap();
        assert_eq!(store.resolve_tip("root").await.unwrap(), "root");
        store.save_lineage(&child).await.unwrap();
        assert_eq!(store.resolve_tip("root").await.unwrap(), "child");
        assert_eq!(store.resolve_tip("child").await.unwrap(), "child");
    }

    #[tokio::test]
    async fn count_compaction_successors_tracks_successor_nodes() {
        let (store, _dir) = test_store();
        store
            .save_lineage(&SessionLineage::root("root"))
            .await
            .unwrap();
        assert_eq!(store.count_compaction_successors("root").await.unwrap(), 0);
        store
            .save_lineage(&SessionLineage::successor("root", "root", "child"))
            .await
            .unwrap();
        assert_eq!(store.count_compaction_successors("root").await.unwrap(), 1);
        assert_eq!(store.count_compaction_successors("child").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn lineage_listing_preserves_root_and_successor() {
        let (store, _dir) = test_store();
        store
            .save_lineage(&SessionLineage::root("root"))
            .await
            .unwrap();
        store
            .save_lineage(&SessionLineage::successor("root", "root", "child"))
            .await
            .unwrap();

        let lineage = store.list_root_lineage("root").await.unwrap();
        assert_eq!(lineage.len(), 2);
        assert_eq!(lineage[0].lineage_kind, LineageKind::Root);
        assert_eq!(lineage[1].lineage_kind, LineageKind::CompactionSuccessor);
    }

    #[tokio::test]
    async fn transcript_segment_roundtrips_without_deleting_original_lineage() {
        let (store, _dir) = test_store();
        let lineage = SessionLineage::successor("root", "root", "child");
        let segment = TranscriptSegment {
            segment_id: "seg-2".into(),
            session_id: "child".into(),
            predecessor_segment_id: Some("seg-1".into()),
            lineage: lineage.clone(),
        };

        store
            .save_lineage(&SessionLineage::root("root"))
            .await
            .unwrap();
        store.save_lineage(&lineage).await.unwrap();
        store.save_segment(&segment).await.unwrap();

        let loaded = store.load_segment("seg-2").await.unwrap().unwrap();
        assert_eq!(loaded.predecessor_segment_id.as_deref(), Some("seg-1"));
        assert!(store.load_lineage("root").await.unwrap().is_some());
    }

    /// Cross-crate contract: context pruning must not mutate durable EventLog payloads.
    ///
    /// This test intentionally depends on `macaca-context` as a **dev-dependency**
    /// only. Production `macaca-persist` must remain free of service-layer imports.
    #[tokio::test]
    async fn large_output_pruning_preserves_original_event_payload() {
        let dir = tempfile::tempdir().unwrap();
        let redb = Arc::new(RedbStore::open(dir.path().join("events.redb")).unwrap());
        let log = EventLog::new(redb);
        let large_stdout = "0123456789".repeat(1_000);
        log.append_command(AppendEventCommand::new(
            "session-1",
            "tool_result",
            "agent",
            serde_json::json!({
                "tool_name": "shell",
                "output": large_stdout,
            }),
        ))
        .await;

        let renderer = DefaultSourceRenderer::new(DefaultPruningPolicy {
            max_full_bytes: 100,
            max_excerpt_bytes: 80,
            drop_empty: true,
        });
        let snippet = renderer.render(ContextRenderInput::new(
            ContextSourceReference::new(
                "events/session-1/1",
                ContextSourceKind::CommandOutput,
                "shell stdout",
            )
            .artifact_ref("events/session-1/00000001"),
            large_stdout.clone(),
            TrustLevel::Untrusted,
        ));

        let events = log.query("session-1", 0, 10).await;
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].payload["output"].as_str(),
            Some(large_stdout.as_str())
        );
        assert!(snippet.text.len() < large_stdout.len());
        assert_eq!(
            snippet.source.artifact_ref.as_deref(),
            Some("events/session-1/00000001")
        );
    }
}
