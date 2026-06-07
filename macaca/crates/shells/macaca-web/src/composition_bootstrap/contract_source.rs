//! Concatenated composition_bootstrap module sources for static contract tests.

/// Returns all composition bootstrap Rust sources concatenated for static checks.
pub(crate) fn composition_bootstrap_module_sources() -> String {
    [
        include_str!("app_state_assembly.rs"),
        include_str!("application_discovery.rs"),
        include_str!("bootstrap_ctx.rs"),
        include_str!("bootstrap_path_helpers.rs"),
        include_str!("config_and_kernel.rs"),
        include_str!("mod.rs"),
        include_str!("post_bootstrap_hooks.rs"),
        include_str!("service_client_facades.rs"),
        include_str!("service_runtime_wiring.rs"),
        include_str!("serve.rs"),
        include_str!("tooling_and_persist.rs"),
    ]
    .join("\n")
}
