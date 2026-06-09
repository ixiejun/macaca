//! P5 §9.2 — coordination-patch retirement terminal audit gate.
//!
//! After unified execution-path convergence (P1 §2.6), production code must not
//! reintroduce multi-path reconciliation markers that distinguished parallel YAML/WASM
//! backends (`legacy_unmarked`, `non_authoritative`, lifecycle suppression patches).
//!
//! Design pattern: **Strategy** — reuses the shared static scanner with the retired
//! `multi-path-coordination-patch` token family. Contract-test modules (`*_tests.rs`)
//! may still mention retired tokens to document the convergence contract; production
//! `src/` modules must remain clean so audit replay stays single-chain.
//!
//! Note: `graph_owner` as a domain field on `TaskGraphOwner` is intentional and
//! distinct from coordination-patch tokens — this gate targets patch literals only.

use super::scanner::assert_token_family_gate;

const GATE_ID: &str = "coordination-patch-retirement";

const FAMILIES: &[&str] = &["multi-path-coordination-patch"];

/// Main entry for the coordination-patch retirement VC gate (tasks §9.2).
pub fn assert_coordination_patch_retirement_gate() {
    eprintln!(
        "p5_terminal_audit_gate event=coordination_patch_retirement_start gate={GATE_ID}"
    );
    assert_token_family_gate(GATE_ID, FAMILIES, None);
    eprintln!(
        "p5_terminal_audit_gate event=coordination_patch_retirement_pass gate={GATE_ID}"
    );
}
