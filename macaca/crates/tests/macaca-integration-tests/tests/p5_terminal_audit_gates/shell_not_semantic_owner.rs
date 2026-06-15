//! P5 §6.1.4 — shell-not-semantic-owner terminal audit gate.
//!
//! Proves presentation shells remain adapters: they must not read retired direct
//! provider fields, start applications outside service commands, or host
//! in-shell agent execution loops.

use super::scanner::assert_token_family_gate;

const GATE_ID: &str = "shell-not-semantic-owner";

const FAMILIES: &[&str] = &[
    "web-direct-runtime-field",
    "application-runtime-direct-start",
    "shell-semantic-execution-owner",
];

/// Shell crate prefix used to scope semantic-ownership violations to presentation layers.
const SHELL_CRATE_PREFIX: &str = "crates/shells/";

/// Main entry for the shell-not-semantic-owner VC gate.
pub fn assert_shell_not_semantic_owner_gate() {
    assert_token_family_gate(GATE_ID, FAMILIES, Some(SHELL_CRATE_PREFIX));
}
