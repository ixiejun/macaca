//! Concatenated chat_orchestrator module sources for static contract tests.

/// Returns all chat_orchestrator Rust sources concatenated for static checks.
pub(crate) fn chat_orchestrator_module_sources() -> String {
    [
        include_str!("agent_activity_adapter.rs"),
        include_str!("agent_execution_adapter.rs"),
        include_str!("application_service_adapter.rs"),
        include_str!("dto.rs"),
        include_str!("executor_adapter.rs"),
        include_str!("executor_event_adapter.rs"),
        include_str!("framework_chat_path.rs"),
        include_str!("mod.rs"),
        include_str!("route_chat_v2.rs"),
        include_str!("route_legacy.rs"),
        include_str!("route_stop.rs"),
        include_str!("session_persistence_adapter.rs"),
        include_str!("sse_adapter.rs"),
        include_str!("wasm_chat_path.rs"),
        include_str!("wasm_dispatch_adapter.rs"),
    ]
    .join("\n")
}
