# Change: Add Optional Web3 Node Module v0

## Why

Route C Phase 10 needs a safe optional Web3 foundation before Macaca can support future EVM/DApp modules, chain-backed A2A settlement, or third-party Web3 plugins. Web3 must not become a base OS dependency: ordinary YAML applications, chat/session flows, goal/task execution, and trace replay must continue to work when no node, wallet, RPC provider, or signing service is installed.

Without an explicit optional-module contract, Web3 logic would likely leak into kernel, app runtime, web shell, payment adapters, or provider-specific code paths. That would violate the microkernel boundary and make absence, compliance denial, and audit behavior inconsistent.

## What Changes

- Add provider-neutral Web3 protocol contracts for wallets, chains, addresses, availability, signing requests, signing policy/decisions, transaction requests/receipts, chain queries, and structured Web3 errors.
- Add an optional Web3 service/facade boundary where missing Web3 is represented by a Null Object unavailable service instead of panic, hang, or implicit dependency failure.
- Add policy hooks for signing, transaction, fee/network constraints, region/compliance denial, and module availability.
- Add trace/audit-compatible Web3 events for availability checks, signing decisions, transaction submission/receipt, chain queries, unavailable results, policy denials, and failures.
- Add mock-only adapters for deterministic no-network tests; real node/RPC/wallet/private-key/EVM integrations remain future work.
- Preserve Route C regression matrix scenarios `RC-APP-001` and `RC-TRACE-001`.
- Add detailed English comments and structured logs for all new Rust code during implementation.

## Impact

- Affected specs: `optional-web3-node-v0`
- Affected crates: `macaca-proto`, `macaca-kernel`, `macaca-ipc`, `macaca-app`, `macaca-web`, and integration tests
- Affected code areas: Web3 protocol contracts, optional service availability, policy facade, trace/audit events, IPC bridge surface, application capability metadata, and thin-shell exposure
- Regression matrix references: `RC-APP-001`, `RC-TRACE-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: Web3 Node Module is optional; absence returns structured unavailable and base OS remains usable.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 10 explicitly protects YAML application loading and trace behavior.
- Follows `macaca/docs/route-c-phase-template.md`: includes Superpowers brainstorm/write-plan, OpenSpec proposal/design/tasks/spec, GitNexus impact, additive implementation, targeted tests, integration smoke, detect_changes, and commit gates.
- Follows `macaca/docs/route-c-architecture-governance.md`: uses Null Object, Adapter/Bridge, Proxy, Strategy, Facade, Observer, Memento, and Specification; all Web3 calls must be policy-checked and traceable.

## Non-Goals

- Do not implement real blockchain nodes, remote RPC clients, wallet storage, private-key management, signing providers, hardware wallets, EVM execution, DApp calls, or real Web3 settlement.
- Do not default-install, default-enable, or require Web3 for base OS, A2A Payment v0, YAML applications, `/api/chat/v2`, task flows, or trace replay.
- Do not bind kernel, protocol, app runtime, or web shell to any concrete chain, node provider, wallet provider, token, application, workflow, gateway, driver, payment provider, model, or business route.
- Do not expose private keys, seed phrases, credentials, raw encrypted payloads, or provider secrets in contracts, logs, events, tests, or storage.
