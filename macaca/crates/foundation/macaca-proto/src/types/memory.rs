//! Memory layer, entry, and compression result types for agent recall surfaces.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::identity::{AgentId, MemoryId};

// ── Memory Types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryLayer {
    Session,
    File,
    Vector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: MemoryId,
    pub layer: MemoryLayer,
    pub content: String,
    pub metadata: serde_json::Value,
    pub agent_id: Option<AgentId>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressResult {
    pub entries_before: usize,
    pub entries_after: usize,
    pub bytes_saved: usize,
}
