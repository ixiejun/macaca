//! Concatenated framework_toolkit module sources for static contract tests.
//!
//! Contract tests assert service-backed toolkit assembly and unified delegation
//! wiring without depending on a single monolithic `framework_toolkit.rs` file.
//! This helper keeps those assertions stable across the Facade + Adapter split.

/// Returns all framework_toolkit Rust sources concatenated for source-scan checks.
pub(crate) fn framework_toolkit_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("policy.rs"),
        include_str!("builder.rs"),
        include_str!("mcp_bridge.rs"),
        include_str!("agent_tools.rs"),
    ]
    .join("\n")
}
