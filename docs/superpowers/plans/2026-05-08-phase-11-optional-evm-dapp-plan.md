# Phase 11 Optional EVM / DApp v0 Implementation Plan

## Scope

Implement Route C Phase 11 as optional EVM/DApp service contracts layered on Phase 10 Web3. The phase defines provider-neutral contracts, unavailable behavior, mock adapter semantics, DApp capability metadata, SDK facade shape, and trace/audit events. It must not run a real node, implement a custom EVM, integrate live Substrate/Frontier/RPC providers, or execute real contracts.

## Architecture Choice

Use a protocol-first optional submodule:

- `macaca-proto`: provider-neutral EVM/DApp value objects and commands.
- `macaca-kernel` or optional service boundary: availability, policy, mock facade, trace/audit events.
- `macaca-app`: DApp capability declarations as metadata/policy input.
- `macaca-sdk`: facade type for future developer calls without provider coupling.
- docs: Substrate/Frontier/EVM adapter boundary design.

## Required Design Patterns

- Null Object for absent EVM module.
- Adapter/Bridge for Substrate/Frontier/EVM RPC providers.
- Command for deploy/call/read/subscribe/estimate/receipt operations.
- Strategy for gas, network, signing, payment, and compliance policy.
- Facade for SDK/application-facing DApp operations.
- Observer for contract command lifecycle and event streams.
- Memento for receipts, gas estimates, read results, and subscription checkpoints.
- Specification for ABI references, policy constraints, capability declarations, and optional-module availability.

## Implementation Slices

### Slice 11.1: EVM protocol contracts

- Add `macaca/crates/macaca-proto/src/evm.rs`.
- Define `EvmChainId`, `ContractAddress`, `ContractAbiRef`, `ContractCallRequest`, `ContractCallResult`, `ContractDeployRequest`, `ContractDeployResult`, `ContractReadRequest`, `ContractReadResult`, `GasPolicy`, `GasEstimate`, `ContractEvent`, `ContractEventSubscription`, `EvmAvailability`, and `EvmError`.
- Tests prove serde roundtrip and custom chain/gas metadata preservation.

### Slice 11.2: EVM unavailable behavior

- Add optional EVM facade or service contract with Null Object unavailable implementation.
- Unavailable EVM returns structured unavailable for deploy/call/read/subscribe/estimate/receipt.
- Ordinary apps and Web3 absence-safe behavior remain unaffected.

### Slice 11.3: Mock EVM adapter

- Add mock adapter for deterministic no-network tests.
- Mock deploy returns a simulated contract address.
- Mock call returns a simulated transaction/call result.
- Mock read returns bounded state digest.
- Mock gas estimate returns deterministic gas estimate.
- Every command emits trace/audit-compatible events.

### Slice 11.4: DApp capability metadata and SDK facade

- Add application/package DApp capability metadata for `web3.evm` without instantiating providers.
- Add SDK-facing facade surface that constructs commands and delegates to service/facade boundaries.
- Ensure signing/payment/gas policy is represented as input and never bypassed.

### Slice 11.5: Substrate/Frontier adapter boundary doc

- Add documentation describing future adapter ownership:
  - provider adapter owns Substrate/Frontier/RPC mapping,
  - kernel owns policy/registry/trace coordination only,
  - application/SDK owns command construction only,
  - web shell owns display/approval surfaces only.

## Files Expected

- New: `macaca/crates/macaca-proto/src/evm.rs`
- Possible new: `macaca/crates/macaca-proto/src/evm_tests.rs`
- Possible new: `macaca/crates/macaca-kernel/src/evm.rs`
- Possible new: `macaca/crates/macaca-kernel/src/evm_event.rs`
- Possible new: `macaca/crates/macaca-app/src/dapp_capability.rs`
- Possible new: `macaca/crates/macaca-sdk/src/evm.rs`
- New doc: `macaca/docs/optional-evm-substrate-frontier-adapter-boundary.md`

## Mandatory Constraints

- Do not implement a custom EVM.
- Do not add real EVM/Substrate/Frontier/RPC dependencies in Phase 11.
- Do not default-install or default-enable EVM.
- Do not make base OS, ordinary applications, trace replay, or Web3 v0 depend on EVM.
- Do not let contract deploy/call bypass signing/payment/gas/compliance policy.
- Do not hardcode chain names, provider names, token names, application names, workflow names, driver names, gateway names, model names, or business routing.
- Do not treat mock adapter output as real chain evidence.
- Keep Rust files below 500 lines; split tests/modules when needed.
- All new Rust code must include detailed English comments and structured `tracing` logs at key execution nodes.

## Verification

- `openspec validate add-optional-evm-dapp-v0 --strict`
- `cargo test -p macaca-proto evm`
- `cargo test -p macaca-kernel evm`
- `cargo test -p macaca-app dapp`
- `cargo test -p macaca-sdk evm`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cargo check --workspace`
- hardcode/secrets scan over new EVM/DApp files
- `npx gitnexus detect-changes --repo agent`

## Commit Plan

After approval and implementation:

- Commit 1: EVM proto and tests.
- Commit 2: optional EVM facade, unavailable behavior, mock adapter, trace events.
- Commit 3: DApp metadata, SDK facade, adapter boundary doc.

Do not commit unrelated dirty files.
