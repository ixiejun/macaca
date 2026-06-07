//! Concatenated skill-mcp module sources for static contract tests.

/// Returns all skill-mcp Rust sources concatenated for static scans.
pub(crate) fn skill_mcp_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("types.rs"),
        include_str!("snapshot.rs"),
        include_str!("governance_telemetry.rs"),
        include_str!("server_resolution.rs"),
        include_str!("probe.rs"),
        include_str!("events.rs"),
    ]
    .join("\n")
}
