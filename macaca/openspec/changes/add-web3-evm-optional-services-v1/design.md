# Design: Web3 / EVM Optional Services v1

## Context

`add-optional-web3-node-v0` and `add-optional-evm-dapp-v0` established provider-neutral Web3/EVM value objects, kernel compatibility facades, null/mock adapters, SDK/Web status surfaces, and absent-safe behavior.

Route C S11 now requires Web3/EVM to become real optional services owned by `macaca-runtime-host`. Kernel may keep service registry primitives, trace/policy primitives, and deprecated compatibility anchors, but Web3/EVM provider lifecycle, command admission, mock/unavailable behavior, and production service calls belong behind `ServiceRuntime`.

This design follows:

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `docs/superpowers/plans/2026-05-10-s11-web3-evm-optional-module-realization-plan.md`

## Goals

- Provide provider-neutral Web3/EVM service contracts in `macaca-proto`.
- Register runtime-host Web3/EVM optional service providers through `ServiceRuntime`.
- Preserve absent-safe base OS behavior when Web3/EVM are unavailable, disabled, or policy-denied.
- Expose SDK `SystemWeb3Client` and `SystemEvmClient` so Web/CLI/Gateway/Application/Agent consumers do not construct kernel facades or runtime-host providers.
- Preserve Web3/EVM v0 compatibility by deprecating old helper paths instead of deleting them.
- Ensure mutating Web3/EVM commands are trace-required, policy-admitted, auditable, logged, bounded, and redacted.
- Make mock/dev providers explicit, diagnosable, and impossible to confuse with real chain execution.

## Non-Goals

- No real chain node, RPC provider, wallet private key, mnemonic, keystore, signing secret, gas payment, or chain transaction broadcast.
- No self-built EVM, Substrate/Frontier adapter, real DApp runtime, chain event indexing, or real chain proof.
- No chain payment settlement, Payment Service adapter, Store/Entitlement rule migration, or marketplace billing UI.
- No new optional-module crates in this phase.
- No removal of existing Web3/EVM v0 compatibility APIs.

## Decisions

### Decision: Web3 And EVM Are Separate Focused Services

Web3 Service owns Web3 availability, wallet list, signing request admission, transaction preparation/admission, chain query, provider descriptor, and service snapshot.

EVM Service owns EVM availability, contract deploy/call/read admission, gas estimate, receipt query, event subscription admission, provider descriptor, and service snapshot.

They are separate service identifiers and provider modules. They may share admission primitives and redaction rules, but neither service becomes a Web3 macro-service that absorbs payment, entitlement, application lifecycle, or chain execution business logic.

### Decision: Runtime-Host Owns Provider Lifecycle

Runtime-host owns unavailable providers, mock/dev providers, provider descriptors, admission specification, command dispatch, service snapshots, structured logs, and trace/audit emission.

Kernel no longer owns new production Web3/EVM provider execution. Existing kernel Web3/EVM code remains as deprecated compatibility anchors so later migrations can find old semantics.

### Decision: SDK Is The Upper-Consumer Boundary

Web, CLI, Gateway, Application Framework, and future agent-facing APIs must use `SystemWeb3Client`, `SystemEvmClient`, or `SystemFacade` accessors. They must not construct runtime-host providers, kernel Web3/EVM facades, chain clients, wallet clients, RPC clients, or EVM adapters directly.

### Decision: Unavailable Is The Default Safe Provider

The built-in default provider is unavailable/null. It returns structured unavailable diagnostics for availability/snapshot/read-only calls and fails closed for mutating commands. Mock/dev providers require explicit test/dev construction and must identify themselves as non-real-chain in descriptors, logs, snapshots, and results.

### Decision: Command DTOs Are Provider-Neutral And Redacted

All operations enter through typed service command DTOs before `ServiceRuntime` dispatch. DTOs may include bounded identifiers, requested operation, capability scope, redacted metadata, and artifact references.

DTOs, logs, trace events, snapshots, and mementos must not include private keys, mnemonics, raw signatures, wallet secrets, provider credentials, raw signed transactions, raw RPC credentials, raw contract bytecode, raw ABI payload, prompt bodies, package bytes, encrypted payload, or unbounded user input.

### Decision: Trace, Policy, Capability, And Bounds Are Admission Specifications

Mutating commands require `TraceContext` before provider execution. Admission specifications validate trace, capability, provider availability, policy status, command size, and redaction before dispatching to any adapter.

Read-only availability/snapshot/list commands may return unavailable diagnostics without requiring a real provider and without failing base OS startup.

## Patterns

- Facade: Web3 Service, EVM Service, `SystemWeb3Client`, and `SystemEvmClient` hide provider/runtime details from upper consumers.
- Adapter / Bridge: unavailable, mock/dev, and future real providers implement provider-neutral contracts without leaking concrete chain/RPC/vendor details.
- Strategy: network policy, signing policy, fee/gas policy, transaction admission, contract call admission, availability policy, and provider selection remain replaceable.
- Command: every service operation is a typed command before entering `ServiceRuntime`.
- Null Object: unavailable clients/providers preserve absent-safe base OS behavior and fail closed for mutating commands.
- Observer: structured trace/audit/log records are emitted for service lifecycle, provider selection, admission, denial, command completion, and snapshot.
- Specification: trace, capability, policy, command bounds, provider descriptor, and redaction rules are centralized.
- Proxy: future local/remote/plugin provider shape stays hidden behind the service contract.
- State: service lifecycle and command lifecycle use small canonical states for snapshots and diagnostics.
- Memento: snapshots and operation summaries are bounded/redacted artifacts, not raw provider payloads.

## Boundary Rules

- Kernel must not execute Web3/EVM provider logic.
- Web/CLI must not define chain, wallet, gas, signing, RPC, or contract semantics.
- Application Framework must declare capability and call SDK clients; it must not import provider implementations.
- Payment settlement must not be implemented inside Web3/EVM.
- Store/Entitlement services may later use Web3/EVM capability checks through SDK/service contracts, but S11 does not change their semantics.
- Provider selection must not branch on application name, workflow name, provider vendor name, chain name, gateway name, driver name, model name, or business-specific identifier.

## Risks / Trade-Offs

- Risk: Web3/EVM becomes a base OS dependency.
  Mitigation: default unavailable providers, absent-safe snapshots, and dependency gate checks keep base OS boot independent.

- Risk: Mock/dev provider is mistaken for real chain execution.
  Mitigation: descriptors and results must mark mock/dev, non-real-chain, and non-settlement behavior.

- Risk: Sensitive wallet or contract data leaks through traces.
  Mitigation: admission redaction specifications reject suspicious payloads; logs only include bounded identifiers, status, reason, and artifact digests.

- Risk: Two services duplicate admission logic.
  Mitigation: share small specification helpers where appropriate while keeping Web3 and EVM service IDs and provider modules separate.

- Risk: Old kernel facade and new service clients coexist for too long.
  Mitigation: mark kernel APIs deprecated, update governance/allowlist, and require new production consumers to use SDK focused clients.

## Migration Plan

1. Add OpenSpec and validate strict deltas.
2. Add `macaca-proto::web3_service` and `macaca-proto::evm_service` DTOs and tests.
3. Add runtime-host Web3 unavailable/mock providers and admission specifications.
4. Add runtime-host EVM unavailable/mock providers and admission specifications.
5. Add SDK `SystemWeb3Client`, `SystemEvmClient`, and `SystemFacade` accessors.
6. Register optional services from Web composition root without adding Web-owned semantics.
7. Mark kernel Web3/EVM facades and adapters deprecated.
8. Update Route C governance and allowlist.
9. Run focused tests, dependency boundary tests, workspace check, hardcode scan, and GitNexus detect changes.

## Rollback

- Disable Web3/EVM service registration while keeping unavailable SDK clients and DTOs.
- Keep deprecated kernel Web3/EVM facades as compatibility fallback.
- Revert Web composition-root registration without affecting ordinary task/session/trace flows.
- Preserve provider-neutral proto DTOs if the OpenSpec remains accepted because they are additive contracts.
