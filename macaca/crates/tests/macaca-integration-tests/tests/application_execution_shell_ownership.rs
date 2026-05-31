//! Application execution shell ownership boundary gate.
//!
//! Web, CLI, and frontend code are presentation adapters. They may parse user
//! input, call focused SDK/SystemFacade clients, render state, and subscribe to
//! replayable events. They must not implement application execution provider
//! lifecycle, EventLog persistence, projection, approval/cancel semantics, or
//! provider strategy logic. This test keeps that rule executable for the
//! application execution protocol platform proposal.

#[path = "application_execution_shell_ownership/gate.rs"]
mod gate;

#[test]
fn shell_surfaces_do_not_own_application_execution_semantics() {
    gate::assert_shell_surfaces_do_not_own_application_execution_semantics();
}
