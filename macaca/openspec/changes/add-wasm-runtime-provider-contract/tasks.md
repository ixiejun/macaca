## 1. Boundary Review
- [x] 1.1 Review existing WASM skeleton, Application ABI, runtime-host application host factory, ServiceRuntime, and Route C boundary docs.
- [x] 1.2 Run GitNexus impact analysis for existing symbols before editing them.

## 2. Provider DTO Contract
- [x] 2.1 Add provider-neutral WASM runtime descriptor, engine capabilities, execution profile, availability, unavailable reason, diagnostics, session request, and resource envelope DTOs.
- [x] 2.2 Add deterministic serialization and sanitization tests.
- [x] 2.3 Export DTOs from `macaca-proto`.

## 3. Runtime Host Provider Contract
- [x] 3.1 Add `WasmApplicationRuntimeProvider` and `WasmExecutionSession` traits.
- [x] 3.2 Add unavailable provider/session implementations that never execute guest code.
- [x] 3.3 Log provider selection, unavailable state, session rejection, and command rejection with sanitized metadata.
- [x] 3.4 Export runtime provider contracts from `macaca-runtime-host`.

## 4. Validation
- [x] 4.1 Run `cargo test -p macaca-proto wasm_runtime`.
- [x] 4.2 Run `cargo test -p macaca-runtime-host wasm_runtime_provider`.
- [x] 4.3 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 4.4 Run `openspec validate add-wasm-runtime-provider-contract --strict`.
- [x] 4.5 Run GitNexus detect changes and verify affected scope.
