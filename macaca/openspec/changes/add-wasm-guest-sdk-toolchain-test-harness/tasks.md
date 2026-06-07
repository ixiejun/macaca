## 1. Boundary and Impact
- [x] 1.1 Review `macaca-sdk` ApplicationKit/AbilityKit/TestKit, WIT schema, existing Application ABI DTOs, and runtime host import bridge.
- [x] 1.2 Run GitNexus impact analysis for relevant host command/TestKit/runtime symbols.
- [x] 1.3 Confirm this slice is runtime-scoped and does not hard-code provider, gateway, driver, workflow, or business names.

## 2. OpenSpec
- [x] 2.1 Add guest SDK facade/proxy contract spec.
- [x] 2.2 Add WIT binding/toolchain workflow contract spec.
- [x] 2.3 Add local mock host harness behavior spec.
- [x] 2.4 Add example app fixture contract spec.
- [x] 2.5 Validate OpenSpec change strictly.

## 3. Runtime Harness
- [x] 3.1 Add runtime-scoped harness module with detailed English comments.
- [x] 3.2 Add guest facade/proxy helpers for service, storage, render, trace, and memory/context-style service calls.
- [x] 3.3 Add mock host import outcomes for success, denied, unavailable, and unsupported.
- [x] 3.4 Add sanitized trace and metadata handling with logs at key execution points.

## 4. Toolchain Fixtures
- [x] 4.1 Add deterministic WIT label fixture generation.
- [x] 4.2 Add deterministic package/artifact/permission/dependency fixture generation.
- [x] 4.3 Add contract checks for ABI/import/permission consistency.
- [x] 4.4 Add runtime example fixtures for headless, GenUI render, memory/context, and service unavailable shapes.

## 5. Validation
- [x] 5.1 Run `cargo test -p macaca-runtime-host wasm_guest_sdk --manifest-path macaca/Cargo.toml`.
- [x] 5.2 Run `cargo test -p macaca-runtime-host wasm_toolchain --manifest-path macaca/Cargo.toml`.
- [x] 5.3 Run `cargo test -p macaca-runtime-host wasm_local_harness --manifest-path macaca/Cargo.toml`.
- [x] 5.4 Run `cargo test -p macaca-integration-tests --test application_platform_contracts --manifest-path macaca/Cargo.toml`.
- [x] 5.5 Run `openspec validate add-wasm-guest-sdk-toolchain-test-harness --strict`.
- [x] 5.6 Run GitNexus detect changes and verify affected scope.
