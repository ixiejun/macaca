## 1. Preparation

- [x] 1.1 Re-read `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-02-system-service-contract.md`.
- [x] 1.2 Re-read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, and `macaca/docs/route-c-architecture-governance.md`.
- [x] 1.3 Re-read Phase 01 primitive contracts in `macaca/crates/macaca-proto/src/kernel.rs` and `macaca/crates/macaca-kernel/src/facade.rs`.
- [x] 1.4 Run GitNexus impact before editing existing symbols in `macaca-proto`, `macaca-kernel`, `macaca-llm`, `macaca-memory`, `macaca-task`, `macaca-driver`, `macaca-skill`, or `macaca-gateway`.
- [x] 1.5 Warn before proceeding if GitNexus reports HIGH or CRITICAL upstream risk.

## 2. `macaca-proto` service descriptors

- [x] 2.1 Add `macaca/crates/macaca-proto/src/service.rs`.
- [x] 2.2 Define `ServiceType`, `ServiceCapability`, `ServiceDescriptor`, `ServiceLifecycleState`, `ServiceHealth`, `ServiceCommand`, `ServiceCallResult`, `CleanupPolicy`, `TraceSchemaRef`, and `ServiceError`.
- [x] 2.3 Export the new module from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.4 Add serde round-trip tests for service descriptor, lifecycle state, command, result, and error values.
- [x] 2.5 Ensure service type is extensible and not a closed provider/business enum.
- [x] 2.6 Ensure all new code has detailed English comments explaining purpose, lifecycle, trace/audit model, and operating constraints.

## 3. `macaca-kernel` service contracts

- [x] 3.1 Add `macaca/crates/macaca-kernel/src/system_service.rs` with `SystemService` and provider-neutral mock service support.
- [x] 3.2 Add `macaca/crates/macaca-kernel/src/service_lifecycle.rs` with lifecycle transition validation.
- [x] 3.3 Add `macaca/crates/macaca-kernel/src/service_call.rs` with `ServiceCallContext`, service call executor, trace-required middleware, and structured call errors.
- [x] 3.4 Ensure every call path requires `TraceContext` before dispatch.
- [x] 3.5 Ensure successful and failed calls emit trace/audit events through the Phase 01 trace boundary.
- [x] 3.6 Add structured logging for registration, lifecycle transition, call accepted/rejected, call completed/failed, and cleanup boundaries.
- [x] 3.7 Export new additive modules from `macaca/crates/macaca-kernel/src/lib.rs`.
- [x] 3.8 Ensure all new code has detailed English comments explaining service boundary responsibilities and why concrete provider logic remains outside kernel.

## 4. Built-in adapter skeleton slice 1

- [x] 4.1 Add LLM service adapter skeleton in `macaca/crates/macaca-llm` or a crate-local adapter module selected by existing project structure.
- [x] 4.2 Add Task service adapter skeleton in `macaca/crates/macaca-task` or a crate-local adapter module selected by existing project structure.
- [x] 4.3 Add Trace service adapter skeleton in the crate that currently owns trace primitives, preserving current runtime behavior.
- [x] 4.4 Verify each adapter exports a descriptor, health state, supported scopes, required permissions, trace schema, and cleanup policy.
- [x] 4.5 Do not migrate existing runtime calls in this slice.

## 5. Built-in adapter skeleton slice 2

- [x] 5.1 Add Driver service adapter skeleton in `macaca/crates/macaca-driver` or a crate-local adapter module selected by existing project structure.
- [x] 5.2 Add Skill service adapter skeleton in `macaca/crates/macaca-skill` or a crate-local adapter module selected by existing project structure.
- [x] 5.3 Add Gateway service adapter skeleton in `macaca/crates/macaca-gateway` or a crate-local adapter module selected by existing project structure.
- [x] 5.4 Add Memory service adapter skeleton in `macaca/crates/macaca-memory` or a crate-local adapter module selected by existing project structure.
- [x] 5.5 Verify adapter descriptors contain no hardcoded application, provider, driver, gateway, model, workflow, or chain names.
- [x] 5.6 Do not migrate existing runtime calls in this slice.

## 6. Tests and regression checks

- [x] 6.1 Add `macaca/crates/macaca-kernel/tests/system_service_contract.rs`.
- [x] 6.2 Test mock service registration, start, call, stop, cleanup, and health.
- [x] 6.3 Test failed service calls return structured `ServiceError`.
- [x] 6.4 Test missing trace context is rejected before dispatch.
- [x] 6.5 Test valid trace context emits trace/audit event.
- [x] 6.6 Test adapter skeleton descriptors for LLM, Task, Trace, Driver, Skill, Gateway, and Memory.
- [x] 6.7 Test lifecycle transition validation rejects invalid transitions.

## 7. Verification

- [x] 7.1 Run `openspec validate add-system-service-contract --strict`.
- [x] 7.2 Run `cargo test -p macaca-proto service`.
- [x] 7.3 Run `cargo test -p macaca-kernel system_service`.
- [x] 7.4 Run targeted adapter tests for `macaca-llm`, `macaca-memory`, `macaca-task`, `macaca-driver`, `macaca-skill`, and `macaca-gateway`.
- [x] 7.5 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 7.6 Run `cargo check --workspace`.
- [x] 7.7 Run `rg -n "FULLSTACK|NEWSROOM|discord|telegram|claude|opencode" macaca/crates/macaca-kernel/src/system_service.rs macaca/crates/macaca-kernel/src/service_call.rs macaca/crates/macaca-proto/src/service.rs` and verify new service contract code does not introduce hardcode.
- [x] 7.8 Run `git diff --check`.
- [x] 7.9 Run GitNexus `detect_changes` before finalizing implementation.

Verification note: `cargo test -p macaca-kernel system_service` compiled successfully but the name filter did not execute integration tests, so `cargo test -p macaca-kernel --test system_service_contract` was also run and executed the 6 contract tests.
