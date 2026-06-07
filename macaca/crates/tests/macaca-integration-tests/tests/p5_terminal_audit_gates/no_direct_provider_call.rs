//! P5 §6.1.2 — no-direct-provider-call terminal audit gate.
//!
//! Proves serviceized capabilities are not invoked through legacy provider handles,
//! compat adapters, or direct runtime catalog reads outside approved migration
//! surfaces. Shell/bootstrap seams remain temporarily allowlisted until P3/P4
//! composition-root convergence deletes them.

use super::scanner::assert_token_family_gate;

const GATE_ID: &str = "no-direct-provider-call";

const FAMILIES: &[&str] = &[
    "provider-compat-construction",
    "direct-runtime-catalog-read",
    "web-direct-runtime-field",
    "application-runtime-direct-start",
];

/// Main entry for the no-direct-provider-call VC gate.
pub fn assert_no_direct_provider_call_gate() {
    assert_token_family_gate(GATE_ID, FAMILIES, None);
}
