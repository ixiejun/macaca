//! Concatenated skill-operations module sources for static contract tests.
//!
//! Boundary tests scan this bundle instead of a monolithic `skill_operations.rs`
//! so assertions stay stable across Facade splits.

/// Returns all skill-operations Rust sources concatenated for static scans.
pub(crate) fn skill_operations_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("types.rs"),
        include_str!("support.rs"),
        include_str!("live_client.rs"),
        include_str!("output.rs"),
        include_str!("snapshot.rs"),
        include_str!("curation.rs"),
        include_str!("lifecycle.rs"),
        include_str!("proposals.rs"),
    ]
    .join("\n")
}
