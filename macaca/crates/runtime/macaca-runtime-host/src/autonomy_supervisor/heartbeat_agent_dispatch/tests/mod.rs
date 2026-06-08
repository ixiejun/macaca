//! Contract tests for manifest-declared heartbeat agent dispatch.
//!
//! Tests exercise the Strategy through the same ServiceRuntime bus boundaries
//! used in production. Fixtures live in `fixtures.rs`; scenarios in
//! `contract_tests.rs` to keep each file under the 500-line gate.

mod contract_tests;
mod fixtures;
