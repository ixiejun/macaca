## 1. Boundary and Impact
- [x] 1.1 Review existing application certification fixtures, WASM admission, runtime provider traits, guest harness, and Route C governance docs.
- [x] 1.2 Run GitNexus impact analysis for certification/testkit/provider symbols and record unavailable index results.
- [x] 1.3 Confirm this slice remains runtime-scoped and does not implement a real hardened daemon.

## 2. OpenSpec
- [x] 2.1 Add certification profile requirements for dev/default/hardened.
- [x] 2.2 Add conformance fixture requirements.
- [x] 2.3 Add negative security test requirements.
- [x] 2.4 Add hardened provider contract requirements.
- [x] 2.5 Add regression matrix requirements.
- [x] 2.6 Validate OpenSpec change strictly.

## 3. Tests First
- [x] 3.1 Add runtime-host tests for certification profile ordering and sanitized reports.
- [x] 3.2 Add runtime-host tests for valid/minimal, GenUI, host import, resource exhausted, ABI mismatch, and unavailable provider fixtures.
- [x] 3.3 Add runtime-host tests for raw env, raw filesystem, raw network, missing trace, missing capability, oversized payload, and timeout/resource exhaustion negative cases.
- [x] 3.4 Add runtime-host tests for hardened provider envelope trace, cancellation, backpressure, timeout, diagnostics, and provider-neutral adapter behavior.
- [x] 3.5 Run the new tests and confirm they fail before implementation.

## 4. Runtime Implementation
- [x] 4.1 Add runtime-scoped certification/conformance module with detailed English comments.
- [x] 4.2 Implement Specification/Visitor certification runner over runtime fixture bundles.
- [x] 4.3 Implement Memento-style sanitized report with bounded identifiers and reason codes.
- [x] 4.4 Implement Template Method profile flow for dev/default/hardened profiles.
- [x] 4.5 Implement hardened provider request/response envelope and mock Adapter.
- [x] 4.6 Add key execution logs for profile start, fixture evaluation, negative case rejection, hardened envelope dispatch, and report completion.

## 5. Governance Docs
- [x] 5.1 Update Route C regression matrix with WASM certification gates.
- [x] 5.2 Update Route C architecture governance with runtime-owned WASM certification/hardened provider ownership.

## 6. Validation
- [x] 6.1 Run `cargo test -p macaca-runtime-host wasm_certification --manifest-path macaca/Cargo.toml`.
- [x] 6.2 Run `cargo test -p macaca-integration-tests --test application_platform_contracts --manifest-path macaca/Cargo.toml`.
- [x] 6.3 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries --manifest-path macaca/Cargo.toml`.
- [x] 6.4 Run `openspec validate add-wasm-certification-fixtures-hardened-provider-contract --strict`.
- [x] 6.5 Run line-count checks for touched Rust files.
- [x] 6.6 Run GitNexus detect changes and verify affected scope.
