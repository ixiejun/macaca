//! Concatenated `composition_bootstrap` module sources for static contract tests.
//!
//! **Contract Test pattern**: P5 split the web composition root into phase modules;
//! escape-hatch and audit-replay tests must scan the *entire* tree without hard-coding
//! per-file paths. This helper uses compile-time `include_str!` to join every bootstrap
//! slice into one string so integration tests can assert forbidden tokens stay absent.
//!
//! Trade-off: adding a new bootstrap file requires updating the registry array below;
//! the explicit list is intentional so CI fails when a module is added but not enrolled
//! in boundary scans (fail-fast over silent coverage gaps).

/// Returns all composition bootstrap Rust sources concatenated for static checks.
pub(crate) fn composition_bootstrap_module_sources() -> String {
    [
        include_str!("app_state_assembly.rs"),
        include_str!("application_discovery.rs"),
        include_str!("bootstrap_ctx.rs"),
        include_str!("bootstrap_path_helpers.rs"),
        include_str!("domain_pack_wiring.rs"),
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
