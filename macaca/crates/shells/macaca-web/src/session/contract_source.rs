//! Concatenated session module sources for static contract tests.
//!
//! Contract tests assert session persistence and trace wiring without depending on a
//! single monolithic `session.rs` file. This helper keeps those assertions stable across
//! the Facade + Adapter module split.

/// Returns all session Rust sources concatenated for `include_str!`-style checks.
pub(crate) fn session_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("types.rs"),
        include_str!("trace_mapping.rs"),
        include_str!("trace_collector.rs"),
        include_str!("turn_model.rs"),
        include_str!("persistence.rs"),
        include_str!("list_handlers.rs"),
        include_str!("crud_handlers.rs"),
        include_str!("sse_handlers.rs"),
    ]
    .join("\n")
}
