//! Concatenated app-ui route module sources for static contract tests.

/// Returns all app-ui route Rust sources concatenated for static scans.
pub(crate) fn app_ui_routes_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("types.rs"),
        include_str!("adapter.rs"),
        include_str!("context.rs"),
        include_str!("asset_handlers.rs"),
        include_str!("bridge_adapter.rs"),
        include_str!("bridge_handlers.rs"),
    ]
    .join("\n")
}
