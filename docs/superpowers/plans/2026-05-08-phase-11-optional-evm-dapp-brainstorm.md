# Phase 11 Optional EVM / DApp v0 Brainstorm

## Current Problem

Macaca needs a safe way for future AI + Web3 applications to declare and use EVM/DApp capabilities without turning EVM into a base OS dependency or binding the system to one concrete chain/runtime/provider. Phase 10 created optional Web3 node, wallet, signing, transaction, and query contracts. Phase 11 must layer EVM/DApp contracts on top of that optional Web3 foundation.

If EVM contract calls are treated as normal tools, Macaca loses signing/payment policy, gas policy, traceability, and optional-module absence semantics. If EVM logic enters the kernel or web shell, it violates Route C microkernel boundaries.

## Why This Phase Must Solve It

Route C Phase 11 is the boundary before any real DApp support. It must define contract deployment, contract call, read-only state query, event subscription, gas estimation, and receipt lookup as provider-neutral service commands. Future Substrate/Frontier/EVM adapters can implement the contract later without changing application semantics.

## Design Pattern Candidates

- Adapter: normalize Substrate, Frontier, EVM RPC, or other runtime providers behind one EVM service contract.
- Bridge: separate protocol/application commands from concrete transport/runtime providers.
- Command: represent deploy, call, read, subscribe, estimate gas, and receipt lookup as explicit auditable commands.
- Strategy: isolate gas policy, network selection, signing policy, payment policy, and read/write execution policy.
- Facade: expose a small DApp/EVM facade to SDK/application code without exposing provider internals.
- Observer: map contract events and command lifecycle events into trace/audit streams.
- Memento: store contract call receipts, gas estimates, read results, and event checkpoints for replay.
- Null Object: absent EVM module returns structured unavailable and never breaks ordinary applications.
- Specification: validate ABI references, capability declarations, gas policy, signing/payment requirements, and optional-module availability before execution.

## Options

### Option A: Protocol-first optional EVM/DApp contracts with mock adapter (recommended)

Define EVM/DApp contracts in `macaca-proto`, add optional facade/adapter boundaries, model unavailable behavior, and provide deterministic mock adapter tests. Real Substrate/Frontier/EVM integration remains a future adapter.

Pros:
- Keeps EVM optional and absent-safe.
- Preserves Route C regression guarantees.
- Establishes stable contracts for SDK/application capabilities.
- Avoids concrete provider dependencies.
- Keeps signing/payment/gas policy explicit and traceable.

Cons:
- Does not execute real contracts yet.
- Requires future adapter implementation before production DApp execution.

### Option B: Add a concrete `macaca-evm` crate with a runtime dependency now

Create a new crate and integrate a real EVM/RPC library immediately.

Pros:
- Faster path to a demo.

Cons:
- Adds dependency and security risk too early.
- Makes optional-module guarantees harder to validate.
- Risks chain/provider hardcoding and overcoupling.

### Option C: Add EVM as a thin extension of Web3 transaction only

Use generic Web3 transaction requests with metadata for contract calls.

Pros:
- Smaller surface.

Cons:
- Loses explicit ABI, gas, event subscription, read-only call, receipt, and DApp capability semantics.
- Makes SDK and audit surfaces ambiguous.

## Recommended Plan

Choose Option A. Establish explicit EVM/DApp contracts and a mock-only optional service path. Real EVM providers, Substrate/Frontier adapters, and live node integration remain future work behind adapter traits.

## Risks

- Risk: EVM becomes a required base OS dependency.
  - Mitigation: Null Object unavailable behavior and Route C baseline tests.
- Risk: contract call bypasses signing/payment/gas policy.
  - Mitigation: every mutating command must require policy approval before adapter execution.
- Risk: provider or chain hardcoding leaks into contracts.
  - Mitigation: string-backed value objects, metadata, and hardcode scans.
- Risk: event subscriptions become unbounded or leak provider payloads.
  - Mitigation: bounded event DTOs with digests/checkpoints and trace/audit payload rules.
- Risk: mock adapter is mistaken for real chain execution.
  - Mitigation: mock status and metadata must explicitly mark simulation-only behavior.

## Rollback

The phase is additive-first. Rollback removes new EVM/DApp contracts, facade/adapter/mock modules, tests, and OpenSpec change files. Existing Web3, A2A, application, task, and trace flows should not depend on EVM in this phase.
