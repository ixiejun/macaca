//! Concatenated skill-operations module sources for static contract tests.
//!
//! Boundary and thin-adapter tests scan this bundle instead of a monolithic
//! `skill_operations_routes.rs` file so assertions stay stable across splits.

/// Returns all skill-operations Rust sources concatenated for static scans.
pub(crate) fn skill_operations_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("adapter.rs"),
        include_str!("types.rs"),
        include_str!("snapshot_handlers.rs"),
        include_str!("lifecycle_handlers.rs"),
        include_str!("curation_handlers.rs"),
        include_str!("materialization_handlers.rs"),
        include_str!("proposal_handlers.rs"),
        include_str!("evaluation_handlers.rs"),
    ]
    .join("\n")
}
