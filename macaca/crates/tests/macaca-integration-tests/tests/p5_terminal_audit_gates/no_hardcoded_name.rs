//! P5 §6.1.3 — no-hardcoded-name terminal audit gate.
//!
//! Proves OS-layer production code does not embed agent role names or
//! provider/model routing literals. Values must flow from manifests, service
//! descriptors, or configuration — never from shell/kernel branches.

use super::scanner::assert_token_family_gate;

const GATE_ID: &str = "no-hardcoded-name";

const FAMILIES: &[&str] = &["hardcoded-agent-role", "provider-model-routing-name"];

/// Main entry for the no-hardcoded-name VC gate.
pub fn assert_no_hardcoded_name_gate() {
    assert_token_family_gate(GATE_ID, FAMILIES, None);
}
