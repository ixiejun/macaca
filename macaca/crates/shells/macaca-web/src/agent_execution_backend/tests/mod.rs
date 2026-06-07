//! Agent execution backend integration tests — Facade module tree.
//!
//! Decomposed from the former monolithic `tests.rs` (656 lines) into focused
//! submodules. Each submodule validates a distinct contract surface:
//! execution control policy, heartbeat evidence, envelope semantics, shell
//! contracts, static wiring, skill evolution boundaries, and architecture guards.
//!
//! Design patterns: Facade (this module), Contract Test, Guard Clause.

mod support;

mod architecture_guards;
mod contract_source;
mod execution_control_policy;
mod execution_envelope;
mod heartbeat_evidence;
mod heartbeat_shell_contract;
mod skill_self_evolution_boundary;
mod static_wiring;
