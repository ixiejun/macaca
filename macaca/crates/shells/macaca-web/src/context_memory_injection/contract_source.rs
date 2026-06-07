//! Concatenated memory-injection module sources for static contract tests.

/// Returns all memory-injection Rust sources concatenated for static scans.
pub(crate) fn context_memory_injection_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("adapter.rs"),
        include_str!("active_recall.rs"),
        include_str!("preflight_recall.rs"),
    ]
    .join("\n")
}
