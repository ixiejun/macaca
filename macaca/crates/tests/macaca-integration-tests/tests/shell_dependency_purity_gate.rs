//! Shell workspace dependency purity gate (VC-shell-deps / P5 §4.4.4 / §9.4).
//!
//! Verifies presentation shells (`macaca-cli`, `macaca-web`) depend only on
//! `macaca-proto` and `macaca-sdk` among workspace members at terminal state.

#[path = "shell_dependency_purity_gate/allowlist.rs"]
mod allowlist;
#[path = "shell_dependency_purity_gate/gate.rs"]
mod gate;

#[test]
fn shell_dependency_purity_gate_cli_is_terminal_proto_sdk_only() {
    gate::assert_cli_shell_workspace_dependency_purity();
}

/// P5 §6.1.7 — web-shell exception inventory must be zero rows at terminal state.
#[test]
fn shell_dependency_purity_gate_web_allowlist_terminal_state_is_zero_rows() {
    gate::assert_web_shell_workspace_dependency_allowlist_terminal_state();
}

#[test]
fn shell_dependency_purity_gate_web_is_terminal_proto_sdk_only() {
    gate::assert_web_shell_workspace_dependency_purity();
}
