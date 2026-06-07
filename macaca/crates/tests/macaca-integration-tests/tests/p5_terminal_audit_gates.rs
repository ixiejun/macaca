//! P5 terminal static audit gates (§6.1.2–6.1.4).
//!
//! Named VC entry points complement `serviceization_escape_hatches` freeze rules
//! with OpenSpec-aligned gate identifiers for CI matrices and governance docs.
//!
//! Design pattern: **Facade** — this file wires independent gate modules; each
//! gate applies the **Strategy** of filtering forbidden-token families.

#[path = "p5_terminal_audit_gates/scanner.rs"]
mod scanner;
#[path = "p5_terminal_audit_gates/surfaces.rs"]
mod surfaces;
#[path = "p5_terminal_audit_gates/tokens.rs"]
mod tokens;

#[path = "p5_terminal_audit_gates/no_direct_provider_call.rs"]
mod no_direct_provider_call;
#[path = "p5_terminal_audit_gates/no_hardcoded_name.rs"]
mod no_hardcoded_name;
#[path = "p5_terminal_audit_gates/shell_not_semantic_owner.rs"]
mod shell_not_semantic_owner;

#[test]
fn p5_no_direct_provider_call_gate_rejects_bypasses() {
    no_direct_provider_call::assert_no_direct_provider_call_gate();
}

#[test]
fn p5_no_hardcoded_name_gate_rejects_os_layer_literals() {
    no_hardcoded_name::assert_no_hardcoded_name_gate();
}

#[test]
fn p5_shell_not_semantic_owner_gate_rejects_shell_execution_ownership() {
    shell_not_semantic_owner::assert_shell_not_semantic_owner_gate();
}
