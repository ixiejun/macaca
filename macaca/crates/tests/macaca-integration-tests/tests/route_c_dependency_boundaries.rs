//! Route C serviceization dependency boundary gate.
//!
//! This test is intentionally implemented as an executable architecture
//! specification instead of a prose-only checklist. Route C moves Macaca toward
//! a microkernel Agent OS where kernel owns only stable invariants, replaceable
//! capabilities live behind system services, and Web/CLI remain presentation
//! shells. Cargo dependency edges are not the whole architecture, but they are a
//! cheap and deterministic first guardrail: if a shell or kernel crate starts
//! depending directly on provider implementations, that coupling is visible
//! before it spreads through runtime code.
//!
//! The helper module is loaded through an explicit path so the integration test
//! can keep a small entry point while the implementation stays below the
//! project file-size limit.

#[path = "route_c_dependency_boundaries/allowlist.rs"]
mod allowlist;
#[path = "route_c_dependency_boundaries/application_execution.rs"]
mod application_execution;
#[path = "route_c_dependency_boundaries/gate.rs"]
mod gate;

#[test]
fn route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges() {
    gate::assert_route_c_dependency_boundaries();
}

/// P5 §6.1.1 — explicit terminal assertion that Route C migration allowlist is empty.
#[test]
fn route_c_dependency_allowlist_terminal_state_is_zero_rows() {
    gate::assert_route_c_allowlist_terminal_state();
}
