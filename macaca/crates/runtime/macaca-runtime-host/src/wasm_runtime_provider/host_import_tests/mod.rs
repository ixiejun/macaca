//! Contract tests for the WASM host-import bridge (Facade module tree).
//!
//! These tests prove unified call-path invariants: privileged host imports must route
//! through ServiceRuntime, emit replayable audit evidence, and reject missing scope,
//! capability, or oversized payloads before crossing service boundaries.
//!
//! Module layout (Contract Test + Object Mother):
//! - `support.rs` — service registration helpers and traced command builders
//! - `service_call_tests.rs` — generic service.call and MCP invoke routing
//! - `domain_import_tests.rs` — task, agent delegate, and trace-only imports
//! - `audit_genui_tests.rs` — audit replay and GenUI surface storage
//! - `admission_guard_tests.rs` — trace/capability/payload/service admission guards
//! - `policy_sanitization_tests.rs` — policy overrides and result sanitization

mod admission_guard_tests;
mod audit_genui_tests;
mod domain_import_tests;
mod domain_pack_service_call_tests;
mod foundation_config_service_call_tests;
mod foundation_random_service_call_tests;
mod foundation_time_service_call_tests;
mod knowledge_domain_pack_service_call_tests;
mod policy_sanitization_tests;
mod service_call_tests;
mod support;
