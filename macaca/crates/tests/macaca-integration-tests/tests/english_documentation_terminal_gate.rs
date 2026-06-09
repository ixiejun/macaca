//! P5 §7.1 — English module documentation terminal gate (VC entry point).
//!
//! Named integration-test facade for tasks §7.1 Definition of Done. Ensures
//! P5 migrated/split OS modules carry module-level `//!` documentation that
//! explains purpose, runtime behavior, and design trade-offs for operators and
//! future refactors.

#[path = "english_documentation_terminal_gate/gate.rs"]
mod gate;

#[test]
fn english_documentation_terminal_state_is_enforced() {
    gate::assert_english_documentation_terminal_state();
}
