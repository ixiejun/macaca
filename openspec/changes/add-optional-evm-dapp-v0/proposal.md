# Change: Add Optional EVM / DApp Module v0

## Why

Route C Phase 11 needs an optional EVM/DApp capability layer on top of the Phase 10 optional Web3 foundation. Macaca must let future AI + Web3 applications express contract operations without turning EVM, Substrate/Frontier, RPC clients, wallets, or DApp provider logic into base OS dependencies.

Without an explicit EVM/DApp service contract, contract deploy/call/read flows may bypass signing, payment, gas, compliance, trace, or audit policy. That would violate the microkernel boundary and make future provider replacement unsafe.

## What Changes

- Add provider-neutral EVM/DApp protocol contracts for chain ids, contract addresses, ABI references, deploy/call/read/subscribe/estimate/receipt commands, gas policy, contract events, availability, and structured EVM errors.
- Add an optional EVM service/facade boundary with Null Object unavailable behavior when no EVM module is installed.
- Add mock-only EVM adapter semantics for deterministic no-network tests; real Substrate/Frontier/EVM RPC adapters remain future work.
- Require all contract deploy/call paths to pass signing, payment, gas, module availability, and compliance policy before adapter execution.
- Emit trace/audit-compatible events for EVM availability, deploy, call, read, subscription, gas estimate, receipt lookup, unavailable, policy-denied, and failure paths.
- Add DApp capability metadata and SDK facade shape without letting applications or SDKs instantiate concrete providers.
- Add a Substrate/Frontier adapter boundary document that defines future ownership without implementing a real node or concrete RPC provider.
- Preserve Route C regression matrix scenarios `RC-APP-001` and `RC-TRACE-001`.
- Require detailed English comments and structured logs for all new Rust code during implementation.

## Impact

- Affected specs: `optional-evm-dapp-v0`
- Affected crates: `macaca-proto`, `macaca-kernel`, `macaca-app`, `macaca-sdk`, documentation, and targeted integration tests
- Affected code areas: EVM protocol contracts, optional EVM service availability, mock adapter boundary, DApp capability metadata, SDK facade surface, policy enforcement, and trace/audit events
- Regression matrix references: `RC-APP-001`, `RC-TRACE-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: EVM/DApp remains an optional Web3 submodule; absence returns structured unavailable and base OS remains usable.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 11 must not break YAML application loading or trace behavior.
- Follows `macaca/docs/route-c-phase-template.md`: includes Superpowers brainstorm/write-plan, OpenSpec proposal/design/tasks/spec, additive implementation, targeted tests, integration smoke, GitNexus gates, and commit gates.
- Follows `macaca/docs/route-c-architecture-governance.md`: uses Null Object, Adapter/Bridge, Command, Strategy, Facade, Observer, Memento, and Specification; all contract operations must be policy-checked, traceable, and auditable.

## Non-Goals

- Do not implement a custom EVM.
- Do not run a real chain node, Substrate node, Frontier runtime, browser wallet, remote RPC client, or external network call.
- Do not add real EVM/Substrate/Frontier/RPC dependencies in Phase 11.
- Do not default-install, default-enable, or require EVM for base OS, ordinary applications, Web3 v0, A2A Payment v0, trace replay, or task execution.
- Do not hardcode chain names, provider names, token names, contract names, application names, workflow names, gateway names, driver names, model names, or business routes.
- Do not expose private keys, seed phrases, credentials, raw encrypted payloads, provider secrets, or unredacted signatures in contracts, logs, events, tests, or storage.
- Do not treat mock adapter output as real chain evidence.
