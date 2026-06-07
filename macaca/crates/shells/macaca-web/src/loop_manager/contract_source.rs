//! Concatenated loop_manager module sources for static contract tests.
//!
//! Contract tests assert unified delegation / execution-control wiring without
//! depending on a single monolithic `loop_manager.rs` file. This helper keeps
//! those assertions stable across the Facade + Adapter module split.

/// Returns all loop_manager Rust sources concatenated for `include_str!`-style checks.
pub(crate) fn loop_manager_module_sources() -> String {
    [
        include_str!("mod.rs"),
        include_str!("loop_orchestrator.rs"),
        include_str!("plan_loop_orchestrator.rs"),
        include_str!("plan_event_consumer.rs"),
        include_str!("plan_event_context.rs"),
        include_str!("plan_event_goal_ready.rs"),
        include_str!("plan_event_goal_lifecycle.rs"),
        include_str!("plan_event_handlers.rs"),
        include_str!("worker_loop_orchestrator.rs"),
        include_str!("worker_execution_adapter.rs"),
        include_str!("agent_execution_adapter.rs"),
        include_str!("execution_control_adapter.rs"),
        include_str!("decomposition_adapter.rs"),
        include_str!("goal_route.rs"),
        include_str!("planner_helpers.rs"),
        include_str!("sse_channel_adapter.rs"),
    ]
    .join("\n")
}
