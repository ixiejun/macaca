//! P5 §7.4 — audit blind-spot terminal gate (VC entry point).
//!
//! Named integration-test facade for tasks §7.4 Definition of Done. Ensures
//! service.call evidence is emitted for routed execution and can be replayed
//! by `trace_id` / `session_id` through `system.service_audit` without unaudited
//! router construction in production bootstrap paths.

#[path = "audit_blind_spot_terminal_gate/gate.rs"]
mod gate;

#[test]
fn audit_blind_spot_terminal_state_has_no_replay_gaps() {
    gate::assert_audit_blind_spot_terminal_state();
}
