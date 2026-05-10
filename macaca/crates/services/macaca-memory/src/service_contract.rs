//! Provider-neutral Memory service contract for Route C S5.
//!
//! Memory storage, vector topology, governance, and active recall are important
//! OS capabilities, but they must stay replaceable.  This module defines the
//! command/result/snapshot DTOs that upper layers use without knowing whether
//! the underlying backend is local, remote, vector-based, file-backed, or a
//! third-party plugin.

use chrono::{DateTime, Utc};
use macaca_proto::{MacacaError, MacacaResult, MemoryEntry, MemoryId, MemoryLayer, TraceContext};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::core::{MemoryCapabilitySet, MemoryScope, MemoryVisibility};
use crate::vector_topology::{
    DefaultVectorTopologyResolver, VectorMemoryTopology, VectorTopologyResolver,
};

/// Stable service id used by runtime-host registration and SDK clients.
pub const MEMORY_SERVICE_ID: &str = "service.memory";

/// Command names accepted by the Memory service provider adapter.
pub const MEMORY_REMEMBER_COMMAND: &str = "memory.remember";
pub const MEMORY_RECALL_COMMAND: &str = "memory.recall";
pub const MEMORY_PREFETCH_COMMAND: &str = "memory.prefetch";
pub const MEMORY_GET_COMMAND: &str = "memory.get";
pub const MEMORY_FORGET_COMMAND: &str = "memory.forget";
pub const MEMORY_STATUS_COMMAND: &str = "memory.status";
pub const MEMORY_SNAPSHOT_COMMAND: &str = "memory.snapshot";

/// Provider-neutral policy hints for memory operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryPolicyHints {
    pub privacy_tier: Option<String>,
    pub max_results: Option<usize>,
    pub metadata: BTreeMap<String, String>,
}

/// Command for writing one memory item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRememberCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub content: String,
    pub layer: MemoryLayer,
    pub metadata: Value,
    pub policy: MemoryPolicyHints,
}

impl MemoryRememberCommand {
    /// Build a scoped write command and validate the isolation boundary.
    pub fn new(
        scope: MemoryScope,
        trace: TraceContext,
        content: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_scope_and_trace(&scope, &trace)?;
        let content = content.into();
        if content.trim().is_empty() {
            return Err(MacacaError::Memory(
                "Memory remember command requires non-empty content".into(),
            ));
        }
        Ok(Self {
            scope,
            trace,
            content,
            layer: MemoryLayer::Session,
            metadata: Value::Null,
            policy: MemoryPolicyHints::default(),
        })
    }
}

/// Command for scoped recall/search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub query: String,
    pub limit: usize,
    pub policy: MemoryPolicyHints,
}

impl MemoryRecallCommand {
    /// Build a scoped recall command with bounded result size.
    pub fn new(
        scope: MemoryScope,
        trace: TraceContext,
        query: impl Into<String>,
        limit: usize,
    ) -> MacacaResult<Self> {
        validate_scope_and_trace(&scope, &trace)?;
        validate_no_global_recall(&scope)?;
        let query = query.into();
        if query.trim().is_empty() {
            return Err(MacacaError::Memory(
                "Memory recall command requires non-empty query".into(),
            ));
        }
        if limit == 0 {
            return Err(MacacaError::Memory(
                "Memory recall command requires positive limit".into(),
            ));
        }
        Ok(Self {
            scope,
            trace,
            query,
            limit,
            policy: MemoryPolicyHints::default(),
        })
    }
}

/// Command for prompt-oriented prefetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPrefetchCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub query: String,
    pub limit: usize,
    pub policy: MemoryPolicyHints,
}

/// Command for scoped point lookup by memory id.
///
/// This command exists so tools and upper adapters do not need to reach around
/// the Memory Service to call a concrete runtime facade for read-only lookups.
/// The service provider still validates the scope and delegates the actual
/// storage strategy to the injected memory facade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGetCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub id: MemoryId,
    pub policy: MemoryPolicyHints,
}

/// Command for scoped deletion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryForgetCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub id: MemoryId,
    pub policy: MemoryPolicyHints,
}

/// Command for lightweight service status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatusCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
}

/// Command for deterministic snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryServiceSnapshotCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub include_governance: bool,
}

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
///
/// The optional entry preserves the old `memory_get` tool behavior while
/// keeping the transport shape explicit and stable across providers.
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

impl From<VectorMemoryTopology> for MemoryTopologyLabels {
    fn from(value: VectorMemoryTopology) -> Self {
        Self {
            application_namespace: value.database,
            agent_collection: value.collection,
            shared_collection: value.shared_collection,
        }
    }
}

/// Deterministic Memory Service snapshot.
///
/// This Memento contains operational metadata only.  It intentionally omits
/// memory bodies, embedding vectors, secrets, and raw user input by default.
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

/// Resolve provider-neutral topology labels from a scope.
pub fn topology_labels_for_scope(scope: &MemoryScope) -> MacacaResult<MemoryTopologyLabels> {
    DefaultVectorTopologyResolver.resolve(scope).map(Into::into)
}

fn validate_scope_and_trace(scope: &MemoryScope, trace: &TraceContext) -> MacacaResult<()> {
    scope.validate()?;
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "Memory service command requires trace_id".into(),
        ));
    }
    Ok(())
}

fn validate_no_global_recall(scope: &MemoryScope) -> MacacaResult<()> {
    if matches!(
        scope.visibility,
        MemoryVisibility::ApplicationShared | MemoryVisibility::GlobalSystem
    ) {
        return Err(MacacaError::Memory(
            "Memory recall requires AgentPrivate or SessionShared scope in S5".into(),
        ));
    }
    Ok(())
}
