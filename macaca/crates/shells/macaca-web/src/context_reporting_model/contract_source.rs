//! Concatenated context-reporting model module sources for static contract tests.

/// Returns all context-reporting model Rust sources concatenated for static scans.
pub(crate) fn context_reporting_model_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("assembly_finalize.rs"),
        include_str!("assembly_service.rs"),
    ]
    .join("\n")
}
