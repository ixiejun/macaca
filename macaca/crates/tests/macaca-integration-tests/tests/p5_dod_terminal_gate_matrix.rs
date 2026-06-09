//! P5 §9.5 — unified Definition-of-Done terminal gate matrix (VC entry point).
//!
//! Single CI-discoverable orchestrator that runs every P5 terminal gate in a
//! deterministic order. Complements individual gate test targets listed in
//! `tasks.md` §13 so operators can prove full microkernel refactor DoD with one
//! command: `cargo test -p macaca-integration-tests --test p5_dod_terminal_gate_matrix`.
//!
//! Design pattern: **Facade + Command** — this file is the facade; `gate.rs`
//! dispatches `cargo test` subprocess commands for each registered terminal gate.

#[path = "p5_dod_terminal_gate_matrix/gate.rs"]
mod gate;

#[test]
fn p5_dod_terminal_gate_matrix_all_gates_green() {
    gate::assert_p5_dod_terminal_gate_matrix();
}
