//! Shell workspace dependency purity gate (VC-shell-deps / P5 §4.4.4).
//!
//! Verifies presentation shells do not grow new direct workspace crate edges.
//! CLI is at terminal purity (proto + sdk); Web tracks a frozen migration baseline.

#[path = "shell_dependency_purity_gate/allowlist.rs"]
mod allowlist;
#[path = "shell_dependency_purity_gate/gate.rs"]
mod gate;

#[test]
fn shell_dependency_purity_gate_cli_is_terminal_proto_sdk_only() {
    gate::assert_cli_shell_workspace_dependency_purity();
}

#[test]
fn shell_dependency_purity_gate_web_matches_frozen_baseline() {
    gate::assert_web_shell_workspace_dependency_baseline();
}
