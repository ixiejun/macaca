## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-05-08-s1-service-runtime-v1-plan.md`.
- [x] 1.2 Read `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`.
- [x] 1.3 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-serviceization-allowlist.md`, and `macaca/docs/route-c-architecture-governance.md`.
- [x] 1.4 Inspect existing `macaca-proto`, `macaca-ipc`, `macaca-kernel`, and `macaca-runtime-host` service-related modules.
- [x] 1.5 Run GitNexus impact before modifying any existing symbol; warn before editing HIGH or CRITICAL impact symbols.

## 2. OpenSpec and Boundary Checks

- [x] 2.1 Validate this OpenSpec change with `openspec validate add-service-runtime-v1 --strict`.
- [x] 2.2 Confirm S1 does not require new S0 allowlist rows before coding.
- [x] 2.3 If implementation adds a forbidden dependency edge, stop and update OpenSpec/allowlist only if architecturally justified.

## 3. Service Provider Factory

- [x] 3.1 Add `macaca/crates/macaca-runtime-host/src/service_provider.rs`.
- [x] 3.2 Define descriptor-driven `ServiceProviderFactory`.
- [x] 3.3 Define factory context and structured factory/runtime errors.
- [x] 3.4 Add test-friendly mock factory support without provider/app/workflow hardcoding.
- [x] 3.5 Add detailed English comments explaining how Abstract Factory avoids provider construction hardcoding.

## 4. Runtime Decorator Chain

- [x] 4.1 Add `macaca/crates/macaca-runtime-host/src/service_decorator.rs`.
- [x] 4.2 Define `ServiceRuntimeDecorator` and runtime call context.
- [x] 4.3 Implement trace-required decorator that rejects before bus dispatch.
- [x] 4.4 Implement policy Strategy and policy decorator.
- [x] 4.5 Add deterministic allow and deny policy strategies for tests.
- [x] 4.6 Add resource, entitlement, and metering extension points without claiming real enforcement.
- [x] 4.7 Add structured logs for decorator allow/deny/reject nodes.

## 5. ServiceRuntime Facade

- [x] 5.1 Add `macaca/crates/macaca-runtime-host/src/service_runtime.rs`.
- [x] 5.2 Define `ServiceRuntime`, config, state records, snapshots, events, and event sinks.
- [x] 5.3 Register provider factories and local bus handlers.
- [x] 5.4 Start services with lifecycle transitions and structured events.
- [x] 5.5 Call services through `macaca-ipc::ServiceBus`, not direct external provider calls.
- [x] 5.6 Stop and cleanup services with lifecycle transitions.
- [x] 5.7 Return deterministic snapshots sorted by service id.
- [x] 5.8 Emit logs and runtime events for register/start/call/reject/complete/fail/stop/cleanup/snapshot.
- [x] 5.9 Keep each Rust file below 500 lines; split helpers if needed.

## 6. Wiring

- [x] 6.1 Update `macaca/crates/macaca-runtime-host/Cargo.toml` with only necessary dependencies.
- [x] 6.2 Update `macaca/crates/macaca-runtime-host/src/lib.rs` exports.
- [x] 6.3 Ensure no application/provider/workflow/model/driver/gateway/chain/business names are hardcoded.
- [x] 6.4 Ensure S1 remains additive and does not change existing Web/CLI/application flows.

## 7. Tests

- [x] 7.1 Add `macaca/crates/macaca-runtime-host/tests/service_runtime.rs`.
- [x] 7.2 Test mock service register/start/call/stop/cleanup.
- [x] 7.3 Test deterministic snapshot lifecycle and health.
- [x] 7.4 Test missing trace rejection before dispatch.
- [x] 7.5 Test deny policy rejection before dispatch.
- [x] 7.6 Test runtime events and service bus trace events at key nodes.
- [x] 7.7 Test duplicate service registration failure.
- [x] 7.8 Test unknown service call failure.
- [x] 7.9 Test provider failure transitions to Failed and emits an event.

## 8. Documentation

- [x] 8.1 Update `macaca/docs/route-c-architecture-governance.md` to reference host-owned `ServiceRuntime`.
- [x] 8.2 Document trace and policy decorator requirements.
- [x] 8.3 Document that provider migrations happen in later S phases.
- [x] 8.4 Do not remove S0 allowlist rows unless implementation genuinely eliminates that dependency debt.

## 9. Verification

- [x] 9.1 Run `openspec validate add-service-runtime-v1 --strict`.
- [x] 9.2 Run `cargo fmt --check`.
- [x] 9.3 Run `cargo test -p macaca-runtime-host service_runtime`.
- [x] 9.4 Run `cargo test -p macaca-ipc service_bus`.
- [x] 9.5 Run `cargo test -p macaca-kernel system_service`.
- [x] 9.6 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 9.7 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 9.8 Run `cargo check --workspace`.
- [x] 9.9 Run `npx gitnexus detect-changes --repo agent` before committing.
