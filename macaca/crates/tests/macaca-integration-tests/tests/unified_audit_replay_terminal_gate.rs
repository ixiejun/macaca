//! P5 §9.1 — unified audit replay single-chain terminal gate (VC entry point).
//!
//! Named integration-test facade for tasks §9.1 Definition of Done. Complements
//! crate-local `unified_audit_replay_convergence_tests` with a CI-discoverable
//! gate identifier in `macaca-integration-tests`.

#[path = "unified_audit_replay_terminal_gate/gate.rs"]
mod gate;

#[test]
fn unified_audit_replay_single_chain_terminal_state() {
    gate::assert_unified_audit_replay_single_chain_terminal_state();
}
