## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, `docs/superpowers/plans/2026-05-07-macaca-os-route-c-microkernel-ecosystem-plan.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-10-optional-web3-node.md`.
- [x] 1.2 Review Phase 09 A2A Payment v0 contracts and current service registry / IPC / app / web thin-shell boundaries before implementation.
- [x] 1.3 Run GitNexus impact before modifying each selected existing symbol; warn before editing any HIGH or CRITICAL impact symbol.

## 2. Web3 Protocol Contracts

- [x] 2.1 Add `macaca/crates/macaca-proto/src/web3.rs` with provider-neutral Web3 contracts.
- [x] 2.2 Export Web3 contracts from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.3 Define `WalletId`, `ChainId`, `Web3Address`, `Web3CapabilityKind`, `Web3Availability`, `Web3UnavailableReason`, `SigningRequest`, `SigningPolicy`, `SigningDecision`, `SigningProof`, `TransactionRequest`, `TransactionReceipt`, `ChainQueryRequest`, `ChainQueryResponse`, and `Web3Error`.
- [x] 2.4 Support unknown/custom chains, wallets, assets, methods, networks, and metadata without provider/chain hardcoding.
- [x] 2.5 Add serde roundtrip tests for availability, signing, transaction, chain query, custom chain/wallet identifiers, and structured errors.

## 3. Optional Web3 Service and Null Object

- [x] 3.1 Add kernel-level optional Web3 facade/service boundary.
- [x] 3.2 Implement unavailable/null Web3 service that returns structured unavailable for signing, transaction, and chain-query requests.
- [x] 3.3 Ensure absent Web3 does not block kernel startup, service registry usage, application loading, A2A Payment v0, or ordinary task flows.
- [x] 3.4 Add structured logs for availability check, unavailable result, request rejection, and no-op/null service path.

## 4. Wallet and Signing Policy Boundary

- [x] 4.1 Define wallet/signing adapter traits without real private-key management.
- [x] 4.2 Implement signing policy evaluation for explicit approval, module availability, region/compliance state, and requested operation scope.
- [x] 4.3 Add mock signing adapter that returns bounded signing proof only after policy approval.
- [x] 4.4 Ensure signing contracts, logs, events, and tests never expose private keys, seed phrases, credentials, or raw encrypted payloads.
- [x] 4.5 Add tests proving signing request requires policy approval and denied signing returns structured Web3 error.

## 5. Transaction and Chain Query Boundary

- [x] 5.1 Define transaction adapter traits and chain-query adapter traits.
- [x] 5.2 Implement mock transaction path that returns deterministic receipt after policy approval.
- [x] 5.3 Implement unavailable chain-query behavior that does not affect base OS.
- [x] 5.4 Add tests for mock transaction receipt, unavailable chain query, and no-network execution.

## 6. Compliance, Region, and Availability State

- [x] 6.1 Model `available`, `unavailable`, `disabled_by_policy`, and `region_blocked` states.
- [x] 6.2 Ensure disabled or region-blocked Web3 calls return structured policy-denied errors.
- [x] 6.3 Add tests proving region/compliance denial blocks signing, transaction, and chain query consistently.

## 7. Trace and Audit Events

- [x] 7.1 Emit trace/audit-compatible events for availability, signing request, signing decision, transaction request, transaction receipt, chain query, unavailable result, policy denial, and failure.
- [x] 7.2 Ensure event payloads include wallet id, chain id, capability, operation, status, request id, transaction id/receipt id when available, session/task scope when available, timestamp, and error code when present.
- [x] 7.3 Ensure logs and events exclude private keys, seed phrases, credentials, raw encrypted payloads, provider secrets, and raw signatures when not explicitly bounded/redacted.
- [x] 7.4 Add tests proving unavailable and denied Web3 events are trace/audit compatible.

## 8. IPC, App, and Web Thin-Shell Integration

- [x] 8.1 Add IPC bridge contracts for Web3 service calls without binding to one transport or provider.
- [x] 8.2 Add application capability request metadata for optional Web3 usage without making application runtime depend on Web3.
- [x] 8.3 Add web thin-shell exposure for Web3 availability/denial data without defining Web3 semantics in `macaca-web`.
- [x] 8.4 Add tests proving base web/app paths compile and optional Web3 absence is represented as data.

## 9. Regression and Verification

- [x] 9.1 Run `openspec validate add-optional-web3-node-v0 --strict`.
- [x] 9.2 Run `cargo test -p macaca-proto web3`.
- [x] 9.3 Run `cargo test -p macaca-kernel web3`.
- [x] 9.4 Run `cargo test -p macaca-ipc web3`.
- [x] 9.5 Run `cargo test -p macaca-app web3`.
- [x] 9.6 Run `cargo test -p macaca-web web3`.
- [x] 9.7 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 9.8 Run `cargo check --workspace`.
- [x] 9.9 Run hardcode/secrets scan over new Web3 files for app/workflow/provider/driver/gateway/model/chain/business constants and private-key/seed/credential leakage.
- [x] 9.10 Run `npx gitnexus detect-changes --repo agent` before committing and verify affected flows align with Phase 10 scope.
