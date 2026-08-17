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
