//! Memory service command DTOs.
//!
//! Commands carry the full scope and trace context across the service boundary.
//! Constructors validate generic OS isolation rules before providers receive a
//! command, keeping provider implementations focused on storage strategies.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::scope::{
    validate_no_global_recall, validate_scope_and_trace, MemoryPolicyHints, MemoryScope,
};
use crate::{MacacaError, MacacaResult, MemoryId, MemoryLayer, TraceContext};

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
