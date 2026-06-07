# Design: Optional Web3 Node Module v0

## Context

Route C separates system invariants from replaceable capabilities. Web3 node and wallet behavior changes frequently across chains, RPC transports, signing systems, compliance rules, and user policies. Therefore Phase 10 treats Web3 as an optional system module and service surface, not as a kernel-owned provider implementation.

Phase 09 A2A Payment v0 intentionally avoided real settlement. Phase 10 establishes the optional Web3 substrate that future payment settlement, EVM/DApp support, and Web3 plugins may use without forcing Web3 into base OS.

## Goals

- Define provider-neutral Web3 contracts in `macaca-proto`.
- Represent absent Web3 through a Null Object unavailable service.
- Provide facade and adapter boundaries that can support local node, remote RPC, wallet, signing, transaction, and chain-query providers later.
- Enforce policy before signing or transaction submission.
- Emit trace/audit-compatible events for every meaningful Web3 lifecycle action.
- Preserve base OS behavior when Web3 is absent, disabled, or region-blocked.
- Keep all new Rust files below 500 lines with detailed English comments and structured logs.

## Non-Goals

- No real chain, RPC, wallet, private-key, EVM, DApp, token, or payment settlement implementation.
- No default Web3 module installation.
- No migration of existing app/task/payment flows to Web3.
- No provider-specific schema or hardcoded chain/provider names.

## Design Decisions

### 1. Protocol-first contracts

Add `macaca-proto/src/web3.rs` with data-only contracts:

- `WalletId`
- `ChainId`
- `Web3Address`
- `Web3CapabilityKind`
- `Web3Availability`
- `Web3UnavailableReason`
- `SigningRequest`
- `SigningPolicy`
- `SigningDecision`
- `SigningProof`
- `TransactionRequest`
- `TransactionReceipt`
- `ChainQueryRequest`
- `ChainQueryResponse`
- `Web3Error`

Pattern: Value Object + Command + State.

All chain, wallet, asset, method, and network identifiers should be string-backed and metadata-extensible. The contracts must not encode one chain or provider.

### 2. Optional service facade and Null Object

Kernel/service-facing code should expose a Web3 facade that returns `Web3Availability::unavailable(...)` when no module is installed. This avoids optional dependency panics and gives applications structured absence.

Pattern: Null Object + Facade.

The unavailable service must be safe for base OS startup and tests. It should log availability checks and return structured errors for signing, transaction, and query requests.

### 3. Adapter/Bridge and Proxy boundaries

Future implementations may be local nodes, remote RPC endpoints, enterprise gateways, wallet plugins, or sandboxed modules. Phase 10 should define adapter traits without concrete provider logic.

Pattern: Adapter / Bridge + Proxy.

Adapters must report availability and capabilities through typed descriptors, not through provider names.

### 4. Policy before signing and transaction

Signing and transaction commands require policy evaluation before execution. Policies cover module availability, explicit approvals, fee/network constraints, region/compliance disablement, and permission scope.

Pattern: Strategy + Specification.

Default behavior is conservative: absent modules are unavailable, disabled regions are denied, and mock signing/transaction paths only work when tests provide an approving policy.

### 5. Trace and audit events

Every availability check, signing request, signing decision, transaction request, receipt, chain query, unavailable result, policy denial, and failure must produce bounded trace/audit-compatible events.

Pattern: Observer + Memento.

Events must include wallet id, chain id, capability, operation, status, request id, transaction id or receipt id when available, session/task scope when present, timestamp, and error code when present. Events must not include secrets, private keys, credentials, seed phrases, or raw encrypted payloads.

### 6. Thin shell and app integration

`macaca-app` may carry Web3 capability request metadata, and `macaca-web` may expose availability/denial data as a thin shell. Neither crate should define Web3 semantics or provider behavior.

Pattern: Command + Facade.

This keeps application framework and presentation layer aligned with Route C boundaries.

## Alternatives Considered

### Concrete Web3 crate now

Rejected for Phase 10 because real node/RPC/wallet dependencies would add security risk and make optional behavior harder to validate.

### Embed Web3 in A2A payment

Rejected because Web3 is broader than payment and must also support future DApp and chain-query modules.

### Kernel-owned Web3 provider

Rejected because Web3 provider behavior is replaceable and ecosystem-specific. Kernel may coordinate policy/registry/trace but must not implement concrete Web3 provider logic.

## Risks and Mitigations

- Risk: Web3 accidentally becomes required by base OS.
  - Mitigation: add absence tests and Route C baseline verification.
- Risk: provider or chain hardcoding leaks into contracts.
  - Mitigation: string-backed value objects, metadata maps, and hardcode scans.
- Risk: signing exposes secrets.
  - Mitigation: protocol only carries requests, decisions, bounded proofs, and redacted event payloads.
- Risk: policy bypass via mock helpers.
  - Mitigation: mock signing/transaction must still pass policy in tests.
- Risk: trace/audit gaps.
  - Mitigation: event sink tests for unavailable, denied, signing, transaction, and query paths.

## Verification Plan

- `openspec validate add-optional-web3-node-v0 --strict`
- `cargo test -p macaca-proto web3`
- `cargo test -p macaca-kernel web3`
- `cargo test -p macaca-ipc web3`
- `cargo test -p macaca-app web3`
- `cargo test -p macaca-web web3`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cargo check --workspace`
- hardcode/secrets scan over new Web3 files
- `npx gitnexus detect-changes --repo agent`
