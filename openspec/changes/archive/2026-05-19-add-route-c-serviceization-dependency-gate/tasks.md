## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-05-08-s0-serviceization-boundary-audit-plan.md`.
- [x] 1.2 Read `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`.
- [x] 1.3 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-serviceization-allowlist.md` if present, and `macaca/docs/route-c-architecture-governance.md`.
- [x] 1.4 Run GitNexus impact before modifying any existing symbol; warn before editing HIGH or CRITICAL impact symbols.

## 2. Dependency Layer Model

- [x] 2.1 Add `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`.
- [x] 2.2 Define stable crate layer classifications for all current workspace crates.
- [x] 2.3 Fail unknown workspace crates with an actionable diagnostic.
- [x] 2.4 Keep test/helper code below 500 lines; split if needed.
- [x] 2.5 Add detailed English comments explaining layer ownership and why each rule exists.

## 3. Cargo Metadata Visitor

- [x] 3.1 Run `cargo metadata --no-deps --format-version 1` from the workspace in the test.
- [x] 3.2 Parse workspace packages and direct workspace dependency edges.
- [x] 3.3 Visit each edge, classify source/target layers, and evaluate boundary rules.
- [x] 3.4 Emit deterministic diagnostics with rule id, source crate, target crate, layers, rationale, and replacement path.
- [x] 3.5 Avoid new dependencies unless existing workspace dependencies are insufficient.

## 4. Boundary Rules and Allowlist

- [x] 4.1 Implement `kernel-no-provider-deps`.
- [x] 4.2 Implement `presentation-no-provider-construction-hub`.
- [x] 4.3 Implement `cli-no-web-internals`.
- [x] 4.4 Implement `optional-not-base-required`.
- [x] 4.5 Implement `service-provider-no-presentation`.
- [x] 4.6 Add a test-local allowlist table for current migration debt.
- [x] 4.7 Ensure new forbidden edges fail unless allowlisted.

## 5. Documentation

- [x] 5.1 Add or update `macaca/docs/route-c-serviceization-allowlist.md`.
- [x] 5.2 Ensure every allowlist row includes rule id, from crate, to crate, current reason, replacement service/facade path, target migration phase, expiry condition, and owner/status.
- [x] 5.3 Update `macaca/docs/route-c-architecture-governance.md` to reference the executable dependency gate.
- [x] 5.4 Document that new exceptions require OpenSpec and allowlist update.

## 6. Verification

- [x] 6.1 Run `openspec validate add-route-c-serviceization-dependency-gate --strict`.
- [x] 6.2 Run `cargo metadata --no-deps --format-version 1`.
- [x] 6.3 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 6.4 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 6.5 Run `cargo check --workspace`.
- [x] 6.6 Run `npx gitnexus detect-changes --repo agent` before committing.
