//! Provider-neutral approval rules for key-value state mutations.
//!
//! This Specification evaluates bounded facts before a service provider is
//! selected. It never receives a key, value, namespace string, or provider
//! handle, keeping approval decisions replay-safe and portable.

use serde::{Deserialize, Serialize};

/// Sanitized mutation facts supplied by application admission and policy layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueApprovalFacts {
    /// True when the operation can affect every key in a namespace or prefix.
    pub namespace_wide: bool,
    /// True when a write replaces an existing value without a revision guard.
    pub overwrite_without_revision: bool,
    /// Bounded number of entries affected by a batch mutation.
    pub batch_entries: u32,
    /// Policy ceiling above which an otherwise valid batch needs approval.
    pub approval_batch_threshold: u32,
}

/// Counter-only capacity requested before a state command reaches a provider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueResourceReservation {
    pub byte_units: u64,
    pub entry_units: u32,
    pub batch_operations: u32,
    pub watch_slots: u32,
    pub snapshot_units: u64,
    pub mutation_operations: u32,
    pub request_units: u32,
}

/// Policy-owned ceilings for a caller's key-value state capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValueResourceLimits {
    pub max_byte_units: u64,
    pub max_entry_units: u32,
    pub max_batch_operations: u32,
    pub max_watch_slots: u32,
    pub max_snapshot_units: u64,
    pub max_mutation_operations: u32,
    pub max_request_units: u32,
}

/// Sanitized failure code for capacity admission before provider dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyValueResourceFailure {
    QuotaExceeded,
}

/// Calculate the next reservation without retaining namespace, key, or value data.
///
/// Runtime service decorators persist the returned counters and own their release
/// lifecycle. This contract helper makes quota rejection deterministic before a
/// concrete provider can observe a side-effecting command.
pub fn reserve_key_value_resources(
    current: KeyValueResourceReservation,
    requested: KeyValueResourceReservation,
    limits: KeyValueResourceLimits,
) -> Result<KeyValueResourceReservation, KeyValueResourceFailure> {
    let next = KeyValueResourceReservation {
        byte_units: current.byte_units.saturating_add(requested.byte_units),
        entry_units: current.entry_units.saturating_add(requested.entry_units),
        batch_operations: current
            .batch_operations
            .saturating_add(requested.batch_operations),
        watch_slots: current.watch_slots.saturating_add(requested.watch_slots),
        snapshot_units: current
            .snapshot_units
            .saturating_add(requested.snapshot_units),
        mutation_operations: current
            .mutation_operations
            .saturating_add(requested.mutation_operations),
        request_units: current
            .request_units
            .saturating_add(requested.request_units),
    };
    (next.byte_units <= limits.max_byte_units
        && next.entry_units <= limits.max_entry_units
        && next.batch_operations <= limits.max_batch_operations
        && next.watch_slots <= limits.max_watch_slots
        && next.snapshot_units <= limits.max_snapshot_units
        && next.mutation_operations <= limits.max_mutation_operations
        && next.request_units <= limits.max_request_units)
        .then_some(next)
        .ok_or(KeyValueResourceFailure::QuotaExceeded)
}

/// Return whether a state mutation needs explicit approval before provider dispatch.
///
/// Restore, migration, and compaction always modify broad state. Namespace-wide
/// deletion, unsafe overwrite, and large batches are also approval-gated. Other
/// commands remain policy-owned and do not gain an implicit approval requirement.
pub fn requires_key_value_approval(command: &str, facts: KeyValueApprovalFacts) -> bool {
    matches!(
        command,
        "kv.restore_namespace" | "kv.migrate_namespace" | "kv.compact_namespace"
    ) || (command == "kv.delete" && facts.namespace_wide)
        || (matches!(command, "kv.put" | "kv.compare_and_set") && facts.overwrite_without_revision)
        || (matches!(command, "kv.batch_put" | "kv.batch_delete")
            && facts.batch_entries > facts.approval_batch_threshold)
}
