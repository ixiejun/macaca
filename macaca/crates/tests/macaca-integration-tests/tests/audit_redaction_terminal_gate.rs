//! P5 §7.3 — canonical audit redaction terminal gate (VC entry point).
//!
//! Named integration-test facade for tasks §7.3 Definition of Done. Ensures
//! `macaca_proto::audit_redaction` is the single source of truth for JSON/metadata
//! redaction across audit export, WASM host imports, and execution event stores.

#[path = "audit_redaction_terminal_gate/gate.rs"]
mod gate;

#[test]
fn audit_redaction_terminal_state_is_canonical() {
    gate::assert_audit_redaction_terminal_state();
}
