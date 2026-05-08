# Change: Add Route C ecosystem hardening

## Why

Route C has established the microkernel boundary, service contracts, IPC, package manifest/runtime guard, Application ABI, GenUI, plugin runtime, Store/Entitlement, A2A payment, optional Web3/EVM modules, and Web/CLI thin-shell direction. Phase 13 must turn those foundations into a developer-facing ecosystem that third parties can use without modifying Macaca source code.

The current system has many foundational primitives, but it still lacks a single certification path that proves application/plugin/skill/MCP packages can be documented, packaged, checked, traced, audited, and rejected with structured diagnostics. Without this phase, the platform risks becoming internally powerful but externally hard to develop for.

## What Changes

- Add developer documentation for applications, plugins, GenUI, Store submission, Web3/DApp development, and certification expectations.
- Add SDK/package fixtures that represent YAML apps, WASM-stub apps, GenUI apps, gateway plugins, driver plugins, paid skills, Web3 optional apps, and EVM optional DApps.
- Add a package compatibility checker in `macaca-app` using Specification + Visitor patterns.
- Add certification tests in `macaca-integration-tests` using Template Method style package-class test flows.
- Add upgrade compatibility policy for OS version, ABI version, package manifest version, optional service availability, and commercial metadata.
- Ensure every checker decision is presentation-neutral, traceable, auditable, and explainable through structured diagnostics and logs.

## Impact

- Affected specs: `ecosystem-hardening`
- Affected crates:
  - `macaca-app`: additive compatibility checker.
  - `macaca-sdk`: examples and developer-facing package fixtures.
  - `macaca-integration-tests`: package certification tests.
- Affected docs:
  - `macaca/docs/developer/*`
  - `macaca/docs/SYSTEM_OVERVIEW.md`
- Regression scope: all Route C regression matrix scenarios, especially RC-APP-001, RC-TRACE-001, RC-SKILL-001, RC-DRIVER-001, RC-RECOVERY-001, and optional Web3/EVM unavailable-safe behavior.

## Non-Goals

- Do not implement a real marketplace backend.
- Do not implement payment settlement or blockchain execution beyond existing optional-module contracts.
- Do not implement a real WASM runtime; WASM execution remains unavailable with structured diagnostics.
- Do not replace existing YAML applications or require third-party developers to modify Macaca source code.
- Do not add application-specific, provider-specific, driver-specific, gateway-specific, chain-specific, or demo-only hardcoding.
