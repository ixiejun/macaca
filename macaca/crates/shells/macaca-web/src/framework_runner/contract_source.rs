//! Concatenated framework_runner module sources for static contract tests.

/// Returns all framework_runner Rust sources concatenated for static checks.
pub(crate) fn framework_runner_module_sources() -> String {
    [
        include_str!("agent_factory.rs"),
        include_str!("agent_factory_build.rs"),
        include_str!("agent_factory_coordinator.rs"),
        include_str!("agent_resolution.rs"),
        include_str!("build_mode.rs"),
        include_str!("channel_emitter_adapter.rs"),
        include_str!("context_prompt_builder.rs"),
        include_str!("context_snapshot_builder.rs"),
        include_str!("coordinator_builder.rs"),
        include_str!("driver_trace_adapter.rs"),
        include_str!("execution_control_middleware.rs"),
        include_str!("executor_emitter_adapter.rs"),
        include_str!("mod.rs"),
        include_str!("prompt_helpers.rs"),
        include_str!("request_composition.rs"),
        include_str!("runtime_agent_builders.rs"),
        include_str!("runtime_execution_control.rs"),
        include_str!("skill_policy.rs"),
        include_str!("sse_emitter_adapter.rs"),
        include_str!("tool_trace.rs"),
        include_str!("traced_builders.rs"),
    ]
    .join("\n")
}
