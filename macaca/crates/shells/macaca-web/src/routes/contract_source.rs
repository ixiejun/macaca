//! Concatenated routes module sources for static contract tests.
//!
//! Contract tests assert HTTP route wiring without depending on a single
//! monolithic `routes.rs` file. This helper keeps those assertions stable
//! across the Facade + Adapter module split.

/// Returns all routes Rust sources concatenated for `include_str!`-style checks.
pub(crate) fn routes_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("shared.rs"),
        include_str!("status.rs"),
        include_str!("apps.rs"),
        include_str!("apps_agents.rs"),
        include_str!("apps_reload.rs"),
        include_str!("skills_mcp.rs"),
        include_str!("todos.rs"),
        include_str!("task_schedules.rs"),
        include_str!("autonomy_schedules.rs"),
        include_str!("autonomy_sanitizers.rs"),
        include_str!("session_inspect.rs"),
        include_str!("drivers.rs"),
        include_str!("context_runtime.rs"),
    ]
    .join("\n")
}
