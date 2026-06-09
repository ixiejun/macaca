//! P5 §7.2 — provider-neutral logging terminal gate (VC entry point).
//!
//! Named integration-test facade for tasks §7.2 Definition of Done. Ensures OS-layer
//! production tracing uses provider-neutral dimensions (`service_id` / `command` /
//! `trace_id` / `reason_code` / stable ids) instead of application/provider/model
//! names as primary log keys.

#[path = "provider_neutral_logging_terminal_gate/gate.rs"]
mod gate;

#[test]
fn provider_neutral_logging_terminal_state_is_enforced() {
    gate::assert_provider_neutral_logging_terminal_state();
}
