//! Shared session lineage and transcript segment contracts.
//!
//! These types are the **persistence-facing DTO layer** for context compaction
//! trees. They live in `macaca-proto` so foundation crates (`macaca-persist`)
//! and service crates (`macaca-context`) can share serde-stable shapes without
//! inverting dependency direction (foundation must not depend on service logic).
//!
//! # Design pattern
//!
//! This module applies the **Shared Kernel** pattern from domain-driven design:
//! a minimal, versioned contract crate owns cross-cutting value objects while
//! behavioral engines remain in upper layers.

use serde::{Deserialize, Serialize};
use tracing::debug;

/// Kind of node within a session lineage tree.
///
/// The tree models how conversations evolve through compaction successors,
/// forks, and isolated child sessions. Kinds are persisted as snake_case JSON
/// so audit replay and HTTP surfaces remain stable across releases.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineageKind {
    /// Root conversation node; `root_session_id == session_id`.
    Root,
    /// Successor created when compaction rewrites history into a new session id.
    CompactionSuccessor,
    /// Branch spawned from a parent session (fork-join orchestration).
    Fork,
    /// Child session isolated from parent prompt history (sandboxed context).
    IsolatedChild,
}

/// Logical lineage metadata for one session node.
///
/// Runtime and persistence layers use this struct to distinguish "same
/// conversation root" from "successor / fork / isolated child" without
/// overloading plain session identifiers. All fields are provider-neutral
/// strings — no application names or model identifiers appear here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionLineage {
    /// Stable root identifier for the entire lineage tree.
    pub root_session_id: String,
    /// Identifier of this node within the tree.
    pub session_id: String,
    /// Direct parent session when this node was derived from another session.
    pub parent_session_id: Option<String>,
    /// Semantic role of this node within the lineage tree.
    pub lineage_kind: LineageKind,
}

impl SessionLineage {
    /// Construct a root lineage node where the session is its own root.
    ///
    /// Root nodes have no parent and use [`LineageKind::Root`]. Compaction and
    /// fork flows always retain `root_session_id` so tip resolution can walk the
    /// tree without re-parsing conversation history.
    pub fn root(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        debug!(
            service_id = "context_lineage",
            session_id = %session_id,
            lineage_kind = "root",
            "constructed root session lineage"
        );
        Self {
            root_session_id: session_id.clone(),
            session_id,
            parent_session_id: None,
            lineage_kind: LineageKind::Root,
        }
    }

    /// Construct a compaction-successor lineage node.
    ///
    /// Successors preserve `root_session_id` while pointing at the compacted
    /// parent via `parent_session_id`. Persistence stores successors as separate
    /// session rows so original event payloads remain auditable.
    pub fn successor(
        root_session_id: impl Into<String>,
        parent_session_id: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        let root_session_id = root_session_id.into();
        let parent_session_id = parent_session_id.into();
        let session_id = session_id.into();
        debug!(
            service_id = "context_lineage",
            root_session_id = %root_session_id,
            parent_session_id = %parent_session_id,
            session_id = %session_id,
            lineage_kind = "compaction_successor",
            "constructed compaction successor lineage"
        );
        Self {
            root_session_id,
            parent_session_id: Some(parent_session_id),
            session_id,
            lineage_kind: LineageKind::CompactionSuccessor,
        }
    }
}

/// Persistable transcript segment metadata associated with a lineage node.
///
/// Segments form a linked list via `predecessor_segment_id` so compaction can
/// reference prior transcript slices without deleting durable event logs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptSegment {
    /// Stable identifier for this segment row.
    pub segment_id: String,
    /// Owning session for the segment (matches `lineage.session_id`).
    pub session_id: String,
    /// Previous segment in the transcript chain, when known.
    pub predecessor_segment_id: Option<String>,
    /// Lineage metadata carried with the segment for audit replay.
    pub lineage: SessionLineage,
}
