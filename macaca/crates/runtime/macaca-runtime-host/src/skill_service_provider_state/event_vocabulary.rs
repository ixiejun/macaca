//! Shared event and snapshot vocabulary for Skill governance state.
//!
//! Centralizes deterministic identifier generation and snapshot filter predicates
//! so every mutation module emits consistent, replay-friendly audit keys without
//! duplicating timestamp encoding rules.

use macaca_skill::SkillLifecycleState;

/// Return whether one lifecycle value should appear in a governance snapshot.
///
/// When explicit lifecycle filters are provided they take precedence.  Otherwise
/// archived records are hidden unless the caller opts in with `include_archived`.
pub(crate) fn snapshot_includes_lifecycle(
    lifecycle: &SkillLifecycleState,
    include_archived: bool,
    lifecycle_filters: &[SkillLifecycleState],
) -> bool {
    if !lifecycle_filters.is_empty() {
        return lifecycle_filters.contains(lifecycle);
    }
    include_archived || lifecycle != &SkillLifecycleState::Archived
}

/// Build a monotonic, collision-resistant governance event identifier.
///
/// Nanosecond timestamps keep ids sortable for bounded replay while remaining
/// provider-neutral (no application or shell semantics are embedded).
pub(crate) fn event_id(prefix: &str, captured_at: chrono::DateTime<chrono::Utc>) -> String {
    let nanos = captured_at
        .timestamp_nanos_opt()
        .unwrap_or_else(|| captured_at.timestamp_micros() * 1_000);
    format!("{prefix}-{nanos}")
}
