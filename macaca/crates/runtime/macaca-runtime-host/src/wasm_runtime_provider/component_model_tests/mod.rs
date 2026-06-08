//! Contract tests for the component-model WASM runtime provider (Facade module tree).
//!
//! These tests prove industrial WASM component invariants: provider descriptors must advertise
//! component capabilities, sessions must require trace context, invalid artifacts must be
//! rejected without leaking raw bytes, and declared host-command plans must route through
//! the unified ServiceRuntime portal with placeholder resolution and audit-safe orchestration.
//!
//! Module layout (Contract Test + Object Mother):
//! - `support.rs` — fixture builders and mock service registration helpers
//! - `descriptor_admission_tests.rs` — provider descriptor and artifact admission guards
//! - `host_import_routing_tests.rs` — service.call routing through host-import bridge
//! - `host_command_plan_tests.rs` — declared host-command plans and payload interpolation
//! - `host_command_orchestration_tests.rs` — deduplication and chained host-command results
//! - `lifecycle_timeout_tests.rs` — wall-time timeout without export invocation

mod descriptor_admission_tests;
mod host_command_orchestration_tests;
mod host_command_plan_tests;
mod host_import_routing_tests;
mod lifecycle_timeout_tests;
mod support;
