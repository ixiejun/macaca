## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, `docs/superpowers/plans/2026-05-07-macaca-os-route-c-microkernel-ecosystem-plan.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-11-optional-evm-dapp.md`.
- [x] 1.2 Review Phase 10 optional Web3 contracts and service boundaries before implementation.
- [x] 1.3 Run GitNexus impact before modifying each selected existing symbol; warn before editing any HIGH or CRITICAL impact symbol.

## 2. EVM Protocol Contracts

- [x] 2.1 Add provider-neutral EVM/DApp contracts, expected in `macaca/crates/macaca-proto/src/evm.rs`.
- [x] 2.2 Export EVM contracts from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.3 Define chain id, contract address, ABI/function refs, deploy/call/read/subscribe/estimate/receipt requests and results, gas policy, events, availability, and structured errors.
- [x] 2.4 Preserve unknown/custom chains, ABI refs, function refs, account ids, contract ids, gas metadata, and DApp metadata without provider/chain hardcoding.
- [x] 2.5 Add serde roundtrip tests for deploy, call, read, subscription, gas estimate, receipt, availability, errors, and custom metadata.

## 3. Optional EVM Facade and Null Object

- [x] 3.1 Add optional EVM facade/service boundary on top of Phase 10 Web3 without making EVM a base OS dependency.
- [x] 3.2 Implement unavailable/null EVM behavior for deploy, call, read, subscribe, estimate gas, and receipt lookup.
- [x] 3.3 Ensure absent EVM does not block kernel startup, service registry usage, application loading, Web3 v0 behavior, A2A Payment v0, ordinary task flows, or trace replay.
- [x] 3.4 Add structured logs for availability checks, unavailable results, request rejection, and no-op/null service path.

## 4. Policy Enforcement

- [x] 4.1 Model signing, payment, gas, module availability, permission scope, and compliance policy inputs for EVM commands.
- [x] 4.2 Ensure deploy and state-changing call commands are denied before adapter execution when required signing/payment/gas/compliance policy is missing or rejected.
- [x] 4.3 Ensure read, estimate, receipt, and subscription commands still pass availability, permission, and compliance checks.
- [x] 4.4 Add tests proving denied policy paths return structured EVM errors and do not execute adapters.

## 5. Mock EVM Adapter

- [x] 5.1 Add deterministic mock adapter behavior for no-network deploy, call, read, gas estimate, receipt lookup, and subscription/event tests.
- [x] 5.2 Ensure mock deploy returns a simulated contract address with explicit simulated provenance.
- [x] 5.3 Ensure mock state-changing call returns simulated transaction/receipt data only after policy approval.
- [x] 5.4 Ensure mock read and gas estimate return bounded deterministic values without external network or real EVM execution.
- [x] 5.5 Add tests proving mock outputs are marked as simulated and are not treated as real chain evidence.

## 6. Trace and Audit Events

- [x] 6.1 Emit trace/audit-compatible events for availability, deploy request/result, call request/result, read request/result, subscription request/event, gas estimate, receipt lookup, unavailable result, policy denial, and failure.
- [x] 6.2 Ensure event payloads include chain id, operation, status, request id, contract address when available, transaction id or receipt id when available, session/task scope when available, timestamp, and error code when present.
- [x] 6.3 Ensure logs and events exclude private keys, seed phrases, credentials, raw encrypted payloads, provider secrets, raw unbounded ABI arguments, and unredacted signatures.
- [x] 6.4 Add tests proving unavailable, denied, and mock EVM events are trace/audit compatible.

## 7. DApp Capability Metadata and SDK Facade

- [x] 7.1 Add application/package DApp capability metadata for optional `web3.evm` usage without provider instantiation.
- [x] 7.2 Add SDK-facing EVM facade shape that constructs commands and delegates to service boundaries.
- [x] 7.3 Ensure application runtime and SDK code do not define provider-specific EVM semantics or bypass policy.
- [x] 7.4 Add tests proving DApp capability declarations remain metadata and absent EVM returns structured unavailable data.

## 8. Substrate/Frontier Adapter Boundary Documentation

- [x] 8.1 Add `macaca/docs/optional-evm-substrate-frontier-adapter-boundary.md`.
- [x] 8.2 Document that provider adapters own Substrate/Frontier/RPC mapping, provider-specific errors, ABI invocation encoding, receipt normalization, and subscription transport.
- [x] 8.3 Document that kernel/service boundaries own registry, policy, availability, trace, and audit coordination only.
- [x] 8.4 Document that application/SDK layers own command construction only and web shell owns display/approval surfaces only.

## 9. Regression and Verification

- [x] 9.1 Run `openspec validate add-optional-evm-dapp-v0 --strict`.
- [x] 9.2 Run `cargo test -p macaca-proto evm`.
- [x] 9.3 Run `cargo test -p macaca-kernel evm`.
- [x] 9.4 Run `cargo test -p macaca-app dapp`.
- [x] 9.5 Run `cargo test -p macaca-sdk evm`.
- [x] 9.6 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 9.7 Run `cargo check --workspace`.
- [x] 9.8 Run hardcode/secrets scan over new EVM/DApp files for app/workflow/provider/driver/gateway/model/chain/business constants and private-key/seed/credential leakage.
- [x] 9.9 Run `npx gitnexus detect-changes --repo agent` before committing and verify affected flows align with Phase 11 scope.
