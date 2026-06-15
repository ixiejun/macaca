//! P5 §6.1.2 — no-direct-provider-call terminal audit gate.
//!
//! Proves serviceized capabilities are not invoked through retired provider
//! handles, bridge adapters, or direct runtime catalog reads.

use super::scanner::assert_token_family_gate;

const GATE_ID: &str = "no-direct-provider-call";

const FAMILIES: &[&str] = &[
    concat!("provider-", "com", "pat", "-construction"),
    "direct-runtime-catalog-read",
    "web-direct-runtime-field",
    "application-runtime-direct-start",
];

/// Main entry for the no-direct-provider-call VC gate.
pub fn assert_no_direct_provider_call_gate() {
    assert_token_family_gate(GATE_ID, FAMILIES, None);
}
