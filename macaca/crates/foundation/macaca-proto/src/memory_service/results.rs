//! Memory service result and snapshot DTOs.
//!
//! Result types are intentionally provider-neutral Mementos. They carry stable
//! timestamps and topology labels that can be audited without exposing backend
//! implementation details.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::scope::MemoryCapabilitySet;
use super::MEMORY_SERVICE_ID;
use crate::{MemoryEntry, MemoryId};

/// Result for memory write operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRememberResult {
    pub id: MemoryId,
    pub stored_at: DateTime<Utc>,
}

/// Result for recall and prefetch operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallResult {
    pub entries: Vec<MemoryEntry>,
    pub total_candidates: usize,
    pub returned_at: DateTime<Utc>,
}

impl MemoryRecallResult {
    /// Wrap entries with deterministic count metadata.
    pub fn new(entries: Vec<MemoryEntry>) -> Self {
        let total_candidates = entries.len();
        Self {
            entries,
            total_candidates,
            returned_at: Utc::now(),
        }
    }
}

/// Result for scoped point lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGetResult {
    pub entry: Option<MemoryEntry>,
    pub returned_at: DateTime<Utc>,
}

impl MemoryGetResult {
    /// Wrap an optional entry with deterministic response metadata.
    pub fn new(entry: Option<MemoryEntry>) -> Self {
        Self {
            entry,
            returned_at: Utc::now(),
        }
    }
}

/// Provider-neutral topology labels exposed by Memory Service snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryTopologyLabels {
    pub application_namespace: String,
    pub agent_collection: String,
    pub shared_collection: Option<String>,
}

/// Deterministic Memory Service snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryServiceSnapshot {
    pub service_id: String,
    pub provider_id: String,
    pub healthy: bool,
    pub capabilities: MemoryCapabilitySet,
    pub topology: Option<MemoryTopologyLabels>,
    pub governance_counts: BTreeMap<String, u64>,
    pub last_audit_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl MemoryServiceSnapshot {
    /// Build a snapshot from provider status and optional topology labels.
    pub fn new(
        provider_id: impl Into<String>,
        healthy: bool,
        capabilities: MemoryCapabilitySet,
        topology: Option<MemoryTopologyLabels>,
    ) -> Self {
        Self {
            service_id: MEMORY_SERVICE_ID.into(),
            provider_id: provider_id.into(),
            healthy,
            capabilities,
            topology,
            governance_counts: BTreeMap::new(),
            last_audit_ids: Vec::new(),
            captured_at: Utc::now(),
        }
    }
}
