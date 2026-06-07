# Phase 10 Optional Web3 Node v0 Implementation Plan

## Scope

Implement Route C Phase 10 as an optional Web3 node/system-module foundation. The phase creates contracts and mock/unavailable behavior only. It must not implement real chains, wallets, RPC providers, private-key management, EVM, DApp calls, or real Web3 settlement.

## Architecture Choice

Use a protocol-first optional system service design:

- `macaca-proto`: provider-neutral Web3 value objects and commands.
- `macaca-kernel`: optional Web3 availability/policy facade and Null Object unavailable service.
- `macaca-ipc`: service-call bridge shape for future local/remote Web3 providers.
- `macaca-app`: application capability request metadata for Web3 without hard dependency.
- `macaca-web`: thin shell only; display/forward availability and denial data without defining Web3 semantics.

## Required Design Patterns

- Null Object for absent Web3 service.
- Adapter/Bridge for node/wallet/signing/transaction implementations.
- Proxy for local node vs remote RPC.
- Strategy for signing, fee, network, and compliance policies.
- Facade for application-facing Web3 calls.
- Observer for trace/audit-compatible lifecycle events.
- Memento for transaction receipt and signing decision replay.
- Specification for permission, region, availability, and request validation.

## Implementation Slices

### Slice 10.1: Web3 protocol and unavailable service

- Add `macaca/crates/macaca-proto/src/web3.rs`.
- Define `WalletId`, `ChainId`, `Web3Address`, `Web3CapabilityKind`, `Web3Availability`, `Web3UnavailableReason`, `Web3Error`, and metadata fields.
- Add unavailable/null behavior in kernel service boundary.
- Tests prove absence is structured unavailable and base OS does not require Web3.

### Slice 10.2: Wallet/signing service contract

- Define `SigningRequest`, `SigningPolicy`, `SigningDecision`, `SigningProof`, and service trait boundaries.
- Require policy approval before signing.
- Mock signing creates bounded proof only; no private keys.
- Logs must include request id, wallet id, chain id, policy status, timestamp, and no secrets.

### Slice 10.3: Transaction and chain query contract

- Define `TransactionRequest`, `TransactionReceipt`, `ChainQueryRequest`, `ChainQueryResponse`.
- Add mock transaction and unavailable chain-query behavior.
- Tests prove unavailable query/transaction does not affect non-Web3 app flows.

### Slice 10.4: Compliance and region disabled state

- Add policy states for `disabled_by_policy`, `region_blocked`, `unavailable`, and `available`.
- Denied calls emit trace/audit-compatible events.
- Tests prove disabled region returns policy-denied and produces a bounded event payload.

## Files Expected

- New: `macaca/crates/macaca-proto/src/web3.rs`
- Possible new: `macaca/crates/macaca-proto/src/web3_tests.rs`
- Possible new: `macaca/crates/macaca-kernel/src/web3.rs`
- Possible new: `macaca/crates/macaca-kernel/src/web3_event.rs`
- Possible new: `macaca/crates/macaca-ipc/src/web3_bridge.rs`
- Possible update: `macaca/crates/macaca-proto/src/lib.rs`
- Possible update: `macaca/crates/macaca-kernel/src/lib.rs`
- Possible update: `macaca/crates/macaca-ipc/src/lib.rs`
- Possible update: `macaca/crates/macaca-app/src/lib.rs`
- Possible update: `macaca/crates/macaca-web/src/lib.rs`

## Mandatory Constraints

- Do not add real Web3 dependencies.
- Do not default-install or default-enable Web3.
- Do not expose private key material through protocol, logs, events, tests, or storage.
- Do not hardcode chain names, providers, app names, workflow names, driver names, gateway names, payment providers, or business routing.
- Do not make A2A Payment v0 depend on Web3.
- Do not break YAML applications, `/api/chat/v2`, task board, trace, or no-network baseline.
- Keep every Rust source file below 500 lines; split tests/modules when needed.
- All new Rust code must include detailed English comments and structured `tracing` logs at key execution nodes.

## Verification

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

## Commit Plan

Use one or more small commits after implementation is approved:

- Commit 1: Web3 proto and tests.
- Commit 2: kernel unavailable/policy/trace facade.
- Commit 3: IPC/app/web shell integration and regression tests.

Do not commit unrelated dirty files.
