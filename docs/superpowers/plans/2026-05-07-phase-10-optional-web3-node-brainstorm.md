# Phase 10 Optional Web3 Node v0 Brainstorm

## Current Problem

Macaca Route C needs Web3 node, wallet, signing, transaction, chain query, and future payment-adapter capabilities without making Web3 part of the base OS. If Web3 is hardwired into the kernel, web shell, app runtime, or A2A payment path, ordinary applications would fail when a node, wallet, region policy, or chain adapter is missing.

Phase 10 must establish the optional module boundary before any EVM/DApp work. The base OS must remain fully usable when Web3 is absent, and installed Web3 modules must be discovered, policy-checked, traced, and replaceable.

## Why This Phase Must Solve It

Route C Phase 11 depends on an optional Web3 foundation. Phase 09 A2A Payment v0 already models payment and receipt flows without real settlement; Phase 10 provides the optional node/signing/transaction contract that future settlement or DApp modules can depend on without coupling the kernel to one chain, wallet, RPC provider, or app workflow.

## Design Pattern Candidates

- Null Object: represent missing Web3 as a first-class unavailable service instead of panic, hang, or optional dependency failure.
- Adapter / Bridge: normalize different node, wallet, signing, and chain-query implementations behind protocol-level contracts.
- Proxy: allow local nodes and remote RPC endpoints to share the same service surface while keeping transport replaceable.
- Strategy: isolate signing policy, fee policy, network policy, compliance policy, and availability policy.
- Facade: expose a small Web3 facade to applications and system services so they never touch private keys or provider internals.
- Observer: emit trace/audit-compatible events for availability checks, signing decisions, transactions, query results, denials, and failures.
- Memento: persist transaction receipts, signing decisions, and bounded audit evidence for replay and compliance.
- Specification: validate manifest permissions, region/compliance state, signing policy, and transaction requests before execution.

## Options

### Option A: Protocol-first optional Web3 service contracts (recommended)

Add provider-neutral Web3 contracts in `macaca-proto`, kernel/service facade boundaries for availability and policy, in-memory/mock adapters for tests, and optional module registration semantics. No real chain, wallet, RPC, key management, or EVM execution is implemented.

Pros:
- Additive and regression-safe.
- Preserves base OS when Web3 is absent.
- Keeps kernel provider-neutral.
- Gives Phase 11 a stable dependency.
- Enables trace/audit and policy gates from day one.

Cons:
- Requires multiple crates to gain protocol/facade/test surfaces.
- Real Web3 functionality remains unavailable until future adapters.

### Option B: Implement a concrete Web3 crate immediately

Create a real `macaca-web3` crate with a concrete node/RPC/wallet implementation.

Pros:
- Faster path to an end-to-end demo.

Cons:
- High coupling risk.
- Introduces external dependencies and security exposure too early.
- Harder to preserve optional-module behavior.
- Violates the plan's "optional module first" intent.

### Option C: Put Web3 support inside A2A payment runtime

Extend Phase 09 payment adapters with Web3 concepts.

Pros:
- Smaller short-term surface.

Cons:
- Conflates payment with Web3 infrastructure.
- Makes non-payment DApp/chain-query use cases awkward.
- Risks payment-provider or chain hardcoding.

## Recommended Plan

Choose Option A. Establish provider-neutral optional Web3 contracts, unavailable/null behavior, policy and trace boundaries, and mock-only tests. Defer real node/RPC/wallet/private-key handling, concrete chain support, and EVM/DApp capability execution to later proposals.

## Risks

- Risk: optional module becomes implicitly required by app or payment paths.
  - Mitigation: tests must prove absent Web3 returns structured unavailable and Route C baseline still passes.
- Risk: contracts accidentally encode one chain/provider.
  - Mitigation: use string-backed identifiers and metadata, hardcode scan for chain/provider/app/workflow constants.
- Risk: signing surface exposes private keys.
  - Mitigation: protocol contains signing requests and bounded signatures/proofs only; private key material is a forbidden field and test scan target.
- Risk: kernel becomes a Web3 provider.
  - Mitigation: kernel only owns service registry/policy/trace facade; adapters live behind replaceable service traits.
- Risk: policy is bypassed by convenience helpers.
  - Mitigation: signing and transaction commands must require explicit policy decisions before execution.

## Rollback

Because the phase is additive-first, rollback is limited to removing new Web3 contracts, facade exports, tests, and OpenSpec change files. No existing app/session/task/trace behavior should depend on Web3 in this phase.
