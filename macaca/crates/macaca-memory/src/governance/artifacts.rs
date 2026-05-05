use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::scope::MemoryScope;

/// Artifact kinds emitted by governance and knowledge compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactKind {
    MemoryReport,
    ProjectDecisionLog,
    WikiDigest,
    GovernanceSummary,
}

/// Artifact payload kept deliberately abstract so callers can render Markdown,
/// JSON, or a future UI payload without changing the governance contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactContent {
    pub content_type: String,
    pub body: String,
    pub redacted: bool,
    pub metadata: Value,
}

/// One memory artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryArtifact {
    pub id: String,
    pub kind: ArtifactKind,
    pub scope: MemoryScope,
    pub path: Option<String>,
    pub content: ArtifactContent,
    pub updated_at: DateTime<Utc>,
}

/// A small list wrapper for batch artifact emission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryArtifactList {
    pub items: Vec<MemoryArtifact>,
}

/// Logical scope tags for artifact visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryArtifactScope {
    Private,
    Shared,
    Public,
}
