# Change: Define microkernel primitive boundary

## Why

Route C Phase 01 must turn the Phase 0 governance boundary into additive Rust contracts. Macaca needs a small, stable microkernel primitive layer before later phases can move system services, WASM applications, Store, plugins, and optional Web3/EVM modules behind explicit service boundaries.

## What Changes

- Add `macaca-proto` kernel primitive value types for service identity, capability identity, capability descriptors, service scope, trace context, policy requests/decisions, resource scopes, and structured primitive errors.
- Add `macaca-kernel` facade/registry/policy/resource/trace traits and skeleton implementations that expose kernel-owned invariants without taking ownership of provider-specific capabilities.
- Add an additive `macaca-sdk` entry point for querying microkernel primitives through the facade instead of depending on `macaca-web` or provider-specific internals.
- Add kernel primitive tests proving serialization, registration/query behavior, duplicate-resource errors, default policy behavior, and deny behavior.
- Add deprecation guidance for direct kernel internals only where a safe facade alternative exists.
- Require detailed English comments for all new code so future maintainers can understand each primitive's purpose and operating model.

## Impact

- Affected specs: `microkernel-primitives`
- Affected crates: `macaca-proto`, `macaca-kernel`, `macaca-sdk`
- Affected tests: `macaca-proto` tests, `macaca-kernel/tests/kernel_primitives.rs`, Route C baseline integration tests
- Regression matrix references: `RC-APP-001`, `RC-GOAL-001`, `RC-PIPE-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: kernel owns invariants; services own replaceable capabilities; applications own business behavior.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 01 must preserve YAML app loading, goal pipeline behavior, and no-network pipeline baseline.
- Follows `macaca/docs/route-c-phase-template.md`: OpenSpec first, additive-first implementation, targeted tests, integration smoke, GitNexus impact/detect_changes.
- Follows `macaca/docs/route-c-architecture-governance.md`: no provider/application hardcode, all capability calls model policy and trace boundaries.

## Non-Goals

- Do not implement WASM Application ABI, Store, entitlement, GenUI, Web3, EVM, or package runtime guards.
- Do not migrate `macaca-web` orchestration into `macaca-kernel`.
- Do not implement concrete LLM, driver, gateway, skill, MCP, memory, or persistence providers in kernel.
- Do not hardcode application names, workflow names, driver names, gateway names, provider names, or business-specific routing.
- Do not replace existing runtime behavior in this phase; all entries are additive compatibility contracts.
