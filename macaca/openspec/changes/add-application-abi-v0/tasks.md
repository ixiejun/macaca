## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-05-application-abi-v0.md`.
- [x] 1.2 Review existing package/runtime guard code in `macaca-proto`, `macaca-app`, and `macaca-sdk`.
- [x] 1.3 Review current YAML application loading and current web/framework application startup paths that must not regress.
- [x] 1.4 Run GitNexus impact before modifying each selected symbol; warn before editing any HIGH or CRITICAL impact symbol.

## 2. Protocol Contracts

- [x] 2.1 Add `macaca/crates/macaca-proto/src/application_abi.rs` with Application ABI v0 value objects and data contracts.
- [x] 2.2 Export Application ABI contracts from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.3 Define ABI exports for `app:init`, `app:start`, `app:handle_event`, `app:render`, `app:pause`, `app:resume`, `app:shutdown`, and `app:upgrade`.
- [x] 2.4 Define ABI imports for `macaca:capability/request`, `macaca:task/create_goal`, `macaca:task/query`, `macaca:trace/emit`, `macaca:ui/render`, `macaca:storage/get`, `macaca:storage/set`, `macaca:payment/create_intent`, and `macaca:service/call`.
- [x] 2.5 Add serde roundtrip tests for ABI declaration, host command, lifecycle event, render request/result, checkpoint, and structured error payloads.
- [x] 2.6 Add tests proving unknown future imports/exports/commands remain structured and unsupported execution returns structured errors.

## 3. Application Framework ABI

- [x] 3.1 Add `macaca/crates/macaca-app/src/abi.rs` with `ApplicationAbiInstance`, `ApplicationAbiAdapter`, descriptor, load result, and unavailable-runtime contracts.
- [x] 3.2 Add `macaca/crates/macaca-app/src/lifecycle.rs` with an explicit lifecycle state machine and structured invalid-transition errors.
- [x] 3.3 Add `macaca/crates/macaca-app/src/host.rs` with the `ApplicationHost` facade and host import command dispatch boundary.
- [x] 3.4 Ensure `ApplicationHost` requires trace context for task, trace, storage, payment, UI, capability, and service-call boundaries where Route C governance requires trace.
- [x] 3.5 Route task create-goal, task query, trace emit, and app-scoped storage through existing safe paths when available.
- [x] 3.6 Return structured `Unavailable`, `DisabledByPolicy`, or `RuntimeUnavailable` results for imports that are declared but not implemented in Phase 05.
- [x] 3.7 Add structured tracing/logging for lifecycle transitions, host commands, adapter selection, unavailable imports, and checkpoint operations.

## 4. YAML Application ABI Adapter

- [x] 4.1 Add a YAML application ABI adapter that converts existing `AppManifest` / package descriptor data into an ABI descriptor without hardcoded application names.
- [x] 4.2 Preserve app id, app name, version, entry agent, workflow references, capabilities, allowed tools, package runtime kind, and declared services where available.
- [x] 4.3 Keep existing YAML application loading behavior compatible while the ABI adapter is introduced additively.
- [x] 4.4 Emit lifecycle events for YAML initialization/start boundaries where they can be observed without changing current execution semantics.
- [x] 4.5 Add tests with real repository YAML application fixtures proving ABI descriptor conversion is generic and preserves required metadata.

## 5. WASM Loader Stub

- [x] 5.1 Add a WASM ABI declaration/manifest metadata loader stub that does not instantiate or execute WASM bytes.
- [x] 5.2 Return structured `RuntimeUnavailable` for execution requests when no WASM runtime is installed.
- [x] 5.3 Log and trace WASM metadata load and runtime-unavailable decisions.
- [x] 5.4 Add tests proving WASM metadata can load while execution fails explicitly without panic.

## 6. SDK Surface

- [x] 6.1 Add `macaca/crates/macaca-sdk/src/application.rs` with developer-facing ABI declaration and host command builders.
- [x] 6.2 Export SDK application helpers from `macaca/crates/macaca-sdk/src/lib.rs`.
- [x] 6.3 Ensure SDK helpers depend only on stable protocol/application contracts and do not expose web/framework internals.
- [x] 6.4 Add SDK tests for building ABI declarations and host commands.

## 7. Framework / Web Integration

- [x] 7.1 Add the minimal integration needed for current application startup code to create or inspect ABI descriptors for YAML apps.
- [x] 7.2 Ensure `/api/chat/v2` session creation and existing framework runner paths continue to compile and run.
- [x] 7.3 Mark any unsafe direct application host path that should not be used by future applications as deprecated, but do not delete it.
- [x] 7.4 Add or update tests proving deprecated direct paths are not used by the new ABI adapter/host path.

## 8. Regression And Verification

- [x] 8.1 Run `openspec validate add-application-abi-v0 --strict`.
- [x] 8.2 Run `cargo test -p macaca-proto application_abi`.
- [x] 8.3 Run `cargo test -p macaca-app application_abi`.
- [x] 8.4 Run `cargo test -p macaca-sdk application`.
- [x] 8.5 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 8.6 Run `cargo check -p macaca-web`.
- [x] 8.7 Run `cargo check -p macaca-framework`.
- [x] 8.8 Run `cargo check --workspace`.
- [x] 8.9 Run a hardcode scan over new ABI files for demo app names, workflow names, provider names, driver names, gateway names, model names, chain names, and business-specific routing.
- [x] 8.10 Run `npx gitnexus detect-changes --repo agent` before committing and verify affected flows match the expected Phase 05 scope.
