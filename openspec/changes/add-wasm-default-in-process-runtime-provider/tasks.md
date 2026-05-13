## 1. Boundary and Impact
- [x] 1.1 Review dependency policy, runtime provider contract, package admission contract, and runtime-host module layout.
- [x] 1.2 Run GitNexus impact analysis for runtime provider and related public symbols.

## 2. OpenSpec
- [x] 2.1 Add default provider behavior spec.
- [x] 2.2 Add runtime error taxonomy spec.
- [x] 2.3 Add compiled artifact cache spec.
- [x] 2.4 Validate OpenSpec change strictly.

## 3. Provider-neutral DTOs
- [x] 3.1 Add `WasmRuntimeErrorKind`, `WasmRuntimeErrorReport`, and compiled cache key/report DTOs.
- [x] 3.2 Add deterministic cache key and sanitized error report tests.

## 4. Runtime Host Provider Modules
- [x] 4.1 Split `wasm_runtime_provider` into focused modules under `src/wasm_runtime_provider/`.
- [x] 4.2 Add default provider descriptor/factory, engine adapter, compile cache, instance session, error mapper, diagnostics, and registry.
- [x] 4.3 Keep each Rust file below 500 lines and document public items with detailed English comments.

## 5. Compile, Instantiate, Invoke
- [x] 5.1 Add controlled artifact loader that reads bytes from file references without logging raw bytes.
- [x] 5.2 Compile and instantiate minimal WASM modules through the private engine adapter.
- [x] 5.3 Invoke lifecycle/export functions through provider-neutral commands and return sanitized results.
- [x] 5.4 Preserve unavailable provider fallback when default provider cannot execute.

## 6. Validation
- [x] 6.1 Run `cargo check -p macaca-runtime-host`.
- [x] 6.2 Run `cargo test -p macaca-runtime-host wasm_default_runtime`.
- [x] 6.3 Run `cargo test -p macaca-integration-tests application_platform_contracts`.
- [x] 6.4 Run `openspec validate add-wasm-default-in-process-runtime-provider --strict`.
- [x] 6.5 Run GitNexus detect changes and verify affected scope.
