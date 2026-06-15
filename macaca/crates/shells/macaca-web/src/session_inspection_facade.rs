//! Session inspection Facade ports for Web routes.
//!
//! The HTTP layer should render session diagnostics without constructing
//! persistence helpers directly. This module defines a provider-neutral port and
//! the current in-process Adapter over host-owned lineage/event-log stores. The
//! same trait can later be implemented by the host-composition crate when the
//! web shell stops depending on local host types.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_host_composition::context::CompactionSummaryEnvelope;
use macaca_host_composition::persist::{EventLog, PersistBackend, SessionLineageStore};
use macaca_proto::{
    AppendEventCommand, LineageKind, MacacaResult, SessionLineage, TranscriptSegment,
};

/// Provider-neutral outcome for a manual session compaction request.
///
/// Routes serialize this read model directly. It contains only bounded lineage
/// identifiers and a reference-only summary, never raw transcript content.
#[derive(Clone, Debug)]
pub struct ManualSessionCompactionSnapshot {
    pub root_session_id: String,
    pub source_session_id: String,
    pub successor_session_id: String,
    pub source_segment_id: String,
    pub successor_segment_id: String,
    pub summary: String,
}

/// Provider-neutral lineage read model returned by session inspection routes.
#[derive(Clone, Debug)]
pub struct SessionLineageSnapshot {
    pub requested_session_id: String,
    pub lineage_tip_session_id: String,
    pub lineage: Vec<SessionLineage>,
}

/// Focused shell-facing port for session inspection and compaction.
///
/// Design pattern: **Facade + Memento**. Implementations own persistence
/// memento reads/writes and sanitized audit events; route handlers own only
/// HTTP parsing and response projection.
#[async_trait]
pub trait SystemSessionInspectionClient: Send + Sync {
    async fn manual_compaction_snapshot(
        &self,
        session_id: String,
        focus_topic: Option<String>,
    ) -> MacacaResult<ManualSessionCompactionSnapshot>;

    async fn lineage_snapshot(&self, session_id: String) -> MacacaResult<SessionLineageSnapshot>;
}

/// In-process Adapter over host-owned persistence stores.
///
/// This implementation is intentionally narrow. It holds only the stores needed
/// for the route contract, emits provider-neutral trace logs, and keeps all
/// lineage write semantics outside HTTP handlers.
pub struct WebSessionInspectionClient {
    session_store: Arc<dyn PersistBackend>,
    event_log: Arc<EventLog>,
}

impl WebSessionInspectionClient {
    /// Create the in-process session inspection client from bootstrap stores.
    pub fn new(session_store: Arc<dyn PersistBackend>, event_log: Arc<EventLog>) -> Self {
        Self {
            session_store,
            event_log,
        }
    }
}

#[async_trait]
impl SystemSessionInspectionClient for WebSessionInspectionClient {
    async fn manual_compaction_snapshot(
        &self,
        session_id: String,
        focus_topic: Option<String>,
    ) -> MacacaResult<ManualSessionCompactionSnapshot> {
        tracing::info!(
            session_id = %session_id,
            has_focus_topic = focus_topic.is_some(),
            "Manual session compaction requested through session inspection Facade"
        );

        let lineage_store = SessionLineageStore::new(Arc::clone(&self.session_store));
        let existing = lineage_store.load_lineage(&session_id).await?;
        let root_session_id = existing
            .as_ref()
            .map(|lineage| lineage.root_session_id.clone())
            .unwrap_or_else(|| session_id.clone());
        if existing.is_none() {
            lineage_store
                .save_lineage(&SessionLineage::root(&session_id))
                .await?;
        }

        let successor_session_id = uuid::Uuid::new_v4().to_string();
        let source_segment_id = format!("segment-{session_id}");
        let successor_segment_id = format!("segment-{successor_session_id}");
        let active_task = focus_topic
            .clone()
            .unwrap_or_else(|| "Continue from latest session state.".to_string());
        let envelope = CompactionSummaryEnvelope {
            root_session_id: root_session_id.clone(),
            source_segment_id: source_segment_id.clone(),
            successor_segment_id: successor_segment_id.clone(),
            resolved: vec!["Manual compaction requested.".into()],
            decisions: vec!["Created successor lineage without deleting source session.".into()],
            current_state: "Original session history remains available for audit.".into(),
            open_questions: Vec::new(),
            active_task,
            important_ids_and_paths: vec![session_id.clone(), successor_session_id.clone()],
        };
        let successor = SessionLineage::successor(
            root_session_id.clone(),
            session_id.clone(),
            successor_session_id.clone(),
        );
        lineage_store.save_lineage(&successor).await?;
        lineage_store
            .save_segment(&TranscriptSegment {
                segment_id: successor_segment_id.clone(),
                session_id: successor_session_id.clone(),
                predecessor_segment_id: Some(source_segment_id.clone()),
                lineage: successor,
            })
            .await?;

        let summary = envelope.render_reference_only();
        self.event_log
            .append_command(AppendEventCommand::new(
                &session_id,
                "context_compaction",
                "context",
                serde_json::json!({
                    "root_session_id": root_session_id,
                    "source_session_id": session_id,
                    "successor_session_id": successor_session_id,
                    "source_segment_id": source_segment_id,
                    "successor_segment_id": successor_segment_id,
                    "focus_topic": focus_topic,
                    "summary": summary.clone(),
                }),
            ))
            .await;
        self.event_log
            .append_command(AppendEventCommand::new(
                &successor_session_id,
                "context_lineage_updated",
                "context",
                serde_json::json!({
                    "root_session_id": envelope.root_session_id,
                    "parent_session_id": session_id,
                    "successor_session_id": successor_session_id,
                    "lineage_kind": LineageKind::CompactionSuccessor,
                }),
            ))
            .await;

        tracing::info!(
            source_session_id = %session_id,
            successor_session_id = %successor_session_id,
            "Manual session compaction lineage snapshot persisted"
        );

        Ok(ManualSessionCompactionSnapshot {
            root_session_id: envelope.root_session_id,
            source_session_id: session_id,
            successor_session_id,
            source_segment_id,
            successor_segment_id,
            summary,
        })
    }

    async fn lineage_snapshot(&self, session_id: String) -> MacacaResult<SessionLineageSnapshot> {
        let lineage_store = SessionLineageStore::new(Arc::clone(&self.session_store));
        let lineage = lineage_store.load_lineage(&session_id).await?;
        let root = lineage
            .as_ref()
            .map(|lineage| lineage.root_session_id.clone())
            .unwrap_or_else(|| session_id.clone());
        let tip = lineage_store.resolve_tip(&session_id).await?;
        let lineage = lineage_store.list_root_lineage(&root).await?;

        tracing::info!(
            requested_session_id = %session_id,
            lineage_tip_session_id = %tip,
            lineage_rows = lineage.len(),
            "Session lineage snapshot prepared for shell route"
        );

        Ok(SessionLineageSnapshot {
            requested_session_id: session_id,
            lineage_tip_session_id: tip,
            lineage,
        })
    }
}
