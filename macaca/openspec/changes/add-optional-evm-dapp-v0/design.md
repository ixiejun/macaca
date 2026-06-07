# Design: Optional EVM / DApp Module v0

## Context

Phase 10 introduced provider-neutral optional Web3 contracts and unavailable behavior. Phase 11 builds on that boundary by defining EVM/DApp contracts as an optional Web3 submodule. The implementation must make future contract operations programmable while keeping base OS, application loading, task execution, trace replay, and ordinary Web3 absence behavior independent of EVM.

Macaca is an agent operating system, not a specific blockchain product. Therefore EVM/DApp support must be a pluggable capability fabric: applications describe intent and policy, SDKs build commands, kernel/service boundaries coordinate policy and trace, and adapters translate commands to concrete providers only when installed.

## Goals

- Define provider-neutral EVM/DApp protocol contracts in `macaca-proto`.
- Represent absent EVM through a Null Object unavailable service.
- Provide adapter/facade boundaries for future Substrate, Frontier, EVM RPC, or other compatible providers.
- Enforce signing, payment, gas, module availability, and compliance policy before deploy/call execution.
- Emit trace/audit-compatible events for every meaningful EVM lifecycle action.
- Add deterministic mock adapter behavior for no-network contract tests.
- Add DApp capability metadata and SDK facade shape without provider coupling.
- Keep all new Rust files below 500 lines with detailed English comments and structured logs.

## Non-Goals

- No real EVM execution, bytecode validation, node runtime, Substrate/Frontier integration, remote RPC transport, browser wallet, private-key handling, token economics, or real contract settlement.
- No default EVM module installation or base OS dependency.
- No migration of existing applications to EVM.
- No provider-specific schema or hardcoded chain/provider/contract names.

## Design Decisions

### 1. Protocol-first EVM contracts

Add provider-neutral EVM/DApp contracts, expected in `macaca-proto/src/evm.rs` during implementation:

- `EvmChainId`
- `ContractAddress`
- `ContractAbiRef`
- `ContractFunctionRef`
- `ContractDeployRequest`
- `ContractDeployResult`
- `ContractCallRequest`
- `ContractCallResult`
- `ContractReadRequest`
- `ContractReadResult`
- `ContractEventSubscription`
- `ContractEvent`
- `GasPolicy`
- `GasEstimate`
- `TransactionReceiptQuery`
- `EvmAvailability`
- `EvmError`

Pattern: Value Object + Command + Memento.

All chain ids, ABI refs, function names, contract identifiers, and metadata fields should be string-backed and extensible. The protocol must preserve unknown/custom values without binding to one chain, provider, or DApp stack.

### 2. Optional service facade and Null Object

Kernel/service-facing code should expose an optional EVM facade. When EVM is absent, disabled, unavailable, or blocked by policy, the facade returns structured `unavailable` or policy-denied errors for deploy, call, read, subscribe, gas estimate, and receipt lookup.

Pattern: Null Object + Facade.

The unavailable service must be safe during base OS startup and tests. It must not require Web3 node installation, RPC configuration, wallet presence, browser context, frontend server, or external network.

### 3. Adapter/Bridge boundary for providers

Future providers may include Substrate/Frontier adapters, EVM RPC gateways, local sandbox nodes, enterprise proxy modules, or third-party plugin runtimes. Phase 11 should define the service contract and mock adapter shape without importing provider dependencies.

Pattern: Adapter / Bridge + Proxy.

Provider adapters own transport mapping, provider-specific errors, ABI invocation encoding, receipt normalization, and subscription transport. Kernel-owned code coordinates policy, registry, availability, trace, and audit only.

### 4. Command model for contract operations

Deploy, call, read, subscribe, estimate gas, and receipt lookup must be represented as commands with explicit request ids and bounded metadata.

Pattern: Command + Specification.

Commands should include enough identity for audit and replay-safe inspection: chain id, optional wallet/account id, contract address or deploy artifact ref, ABI ref, function ref, arguments as bounded structured payloads, gas policy, value policy, session/task scope when present, and metadata.

### 5. Policy before execution

Deploy and state-changing call commands must pass signing, payment, gas, module availability, permission scope, and compliance policy before any adapter executes. Read, estimate, receipt, and subscription commands must still pass availability, permission, and compliance policy.

Pattern: Strategy + Specification.

Policy strategies must be replaceable. The default behavior is conservative: absent modules are unavailable, missing approvals are denied, disabled or region-blocked scopes are denied, and mock adapters execute only when tests provide approving policy.

### 6. Trace and audit lifecycle

Every availability check, deploy request/result, call request/result, read request/result, subscription request/event, gas estimate, receipt lookup, unavailable result, policy denial, and failure must produce structured logs and trace/audit-compatible events.

Pattern: Observer + Memento.

Events must include chain id, operation, status, request id, contract address when available, transaction id or receipt id when available, session/task scope when available, timestamp, and error code when present. Events must not include secrets, private keys, credentials, seed phrases, raw encrypted payloads, provider secrets, raw unbounded ABI arguments, or unredacted signatures.

### 7. DApp capability metadata and SDK facade

Application/package metadata may declare optional DApp/EVM capability requirements such as `web3.evm`. SDK code may expose a facade that builds commands and delegates them to the optional service surface.

Pattern: Facade + Dependency Inversion.

Applications and SDKs must not instantiate providers directly. They should supply capability intent and command data, then receive structured availability, denial, or mock results from the service boundary.

## Alternatives Considered

### Implement a real Substrate/Frontier adapter now

Rejected for Phase 11 because it would add provider dependencies, network/runtime complexity, and security concerns before the Macaca contract boundary is stable.

### Put EVM directly into Web3 v0

Rejected because Web3 is broader than EVM. Keeping EVM as an optional submodule protects ordinary Web3 signing/query flows and preserves modular replacement.

### Let SDK call providers directly

Rejected because direct provider calls bypass policy, payment/gas controls, trace, audit, and optional-module unavailable behavior.

### Treat mock outputs as receipts

Rejected because mock outputs are deterministic test artifacts only. They must be clearly marked and must not be accepted as real chain evidence.

## Risks and Mitigations

- Risk: EVM accidentally becomes a base OS dependency.
  - Mitigation: absence tests, no real dependencies, Null Object service, and Route C baseline verification.
- Risk: contract calls bypass signing/payment/gas/compliance policy.
  - Mitigation: service facade enforces policy before adapter execution and tests cover denied paths.
- Risk: provider hardcoding leaks into protocol contracts.
  - Mitigation: string-backed value objects, extensible metadata, and hardcode scans.
- Risk: trace/audit events leak sensitive ABI or signing data.
  - Mitigation: bounded/redacted event payloads and tests for sensitive-material exclusion.
- Risk: mock behavior is mistaken for real chain execution.
  - Mitigation: mock result metadata must explicitly identify simulated provenance.
- Risk: adapter boundary becomes too abstract to implement.
  - Mitigation: keep Phase 11 contracts aligned with deploy/call/read/subscribe/estimate/receipt lifecycle and document Substrate/Frontier mapping responsibilities.

## Verification Plan

- `openspec validate add-optional-evm-dapp-v0 --strict`
- `cargo test -p macaca-proto evm`
- `cargo test -p macaca-kernel evm`
- `cargo test -p macaca-app dapp`
- `cargo test -p macaca-sdk evm`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cargo check --workspace`
- hardcode/secrets scan over new EVM/DApp files
- `npx gitnexus detect-changes --repo agent`
