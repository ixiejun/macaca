## 1. Boundary and Impact
- [x] 1.1 Review current runtime provider session, artifact loading, host invocation path, design patterns, and related OpenSpec changes.
- [x] 1.2 Run GitNexus impact analysis for the related WASM provider symbols and record that the current index does not yet include the newest symbols.
- [x] 1.3 Classify admission policy as proto/Application Framework vocabulary and runtime enforcement as runtime-host guards.

## 2. OpenSpec
- [x] 2.1 Add resource policy spec.
- [x] 2.2 Add sandbox policy spec.
- [x] 2.3 Add WASI policy spec.
- [x] 2.4 Add resource audit spec.
- [x] 2.5 Validate OpenSpec change strictly.

## 3. Provider-Neutral Policy DTOs
- [x] 3.1 Add resource policy, quota key, sandbox policy, WASI policy, and audit DTOs.
- [x] 3.2 Add deterministic policy merge and sanitization tests.
- [x] 3.3 Keep DTO modules data-only and documented with detailed English comments.

## 4. Runtime Enforcement
- [x] 4.1 Add runtime guard modules for payload limits and concurrency admission.
- [x] 4.2 Install guards in default provider session creation and dispatch.
- [x] 4.3 Emit sanitized audit logs for deny, throttle, and resource exhaustion paths.
- [x] 4.4 Preserve unavailable fallback and raw WASI/env/fs/network deny-by-default behavior.

## 5. Validation
- [x] 5.1 Run `cargo test -p macaca-proto wasm_resource`.
- [x] 5.2 Run `cargo test -p macaca-runtime-host wasm_sandbox`.
- [x] 5.3 Run `cargo fmt --all --manifest-path macaca/Cargo.toml`.
- [x] 5.4 Run `openspec validate add-wasm-sandbox-resource-governance --strict`.
- [x] 5.5 Run GitNexus detect changes and verify affected scope before any commit.
