//! Contract tests for HeartbeatLane cadence and dispatch boundaries.
//!
//! These tests prove runtime-host heartbeat supervision stays non-blocking,
//! cadence-aware, and evidence-gated without embedding application behavior.
//! Module layout (Contract Test + Test Double + Object Mother):
//! - `test_doubles.rs` — generic service/backend fakes for boundary exercises
//! - `fixtures.rs` — Object Mother profile builders and polling helpers
//! - `nonblocking_tick.rs` — tick must not await slow agent execution
//! - `cadence_tick.rs` — distinct agent cadences dispatch on separate ticks
//! - `run_history.rs` — run memento records success and evidence failures

mod cadence_tick;
mod fixtures;
mod nonblocking_tick;
mod run_history;
mod test_doubles;
