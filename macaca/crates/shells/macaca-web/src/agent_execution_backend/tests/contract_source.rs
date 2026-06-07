//! Compile-time contract source paths for agent execution backend tests.
//!
//! Centralizes relative paths for static wiring scans so test modules stay
//! decoupled from filesystem layout changes (Contract Test pattern).

/// Returns relative paths (from `tests/`) to agent execution backend sources under test.
pub(crate) fn agent_execution_backend_test_module_sources() -> [&'static str; 8] {
    [
        "../mod.rs",
        "mod.rs",
        "support.rs",
        "execution_control_policy.rs",
        "heartbeat_evidence.rs",
        "execution_envelope.rs",
        "heartbeat_shell_contract.rs",
        "static_wiring.rs",
    ]
}
