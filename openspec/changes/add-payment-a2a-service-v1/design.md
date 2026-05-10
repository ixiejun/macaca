# Design: Payment / A2A Service v1

## Context

`add-a2a-payment-v0` established provider-neutral A2A payment contracts, a kernel-level compatibility coordinator, policy evaluation, local simulated adapter behavior, payment memento persistence, and trace/audit-compatible lifecycle events.

Route C now requires S10 to move replaceable Payment / A2A behavior out of kernel ownership. Kernel may retain policy primitives and service registry invariants, but quote adapter execution, payment intent orchestration, settlement, receipt/proof persistence, and production service calls belong to Payment Service.

This design follows:

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `docs/superpowers/plans/2026-05-10-s10-payment-a2a-serviceization-plan.md`

## Goals

- Provide a provider-neutral Payment Service contract in `macaca-proto`.
- Register a runtime-host Payment Service provider through `ServiceRuntime`.
- Keep budget/approval checks as replaceable policy strategy while preventing adapter execution before policy allows it.
- Persist quote, transition, receipt, and proof mementos through the existing `PaymentStore` contract.
- Expose SDK `SystemPaymentClient` so Web/CLI/Gateway/Application/Agent consumers do not construct kernel coordinators or runtime-host providers.
- Preserve A2A Payment v0 compatibility by deprecating old helper paths instead of deleting them.
- Ensure every mutating command is trace-required, auditable, logged, and redacted.

## Non-Goals

- No real external payment rail or provider integration.
- No Web3/EVM optional module integration.
- No wallet signing, private key management, chain transaction, or smart contract execution.
- No Store/Entitlement authorization behavior changes.
- No marketplace billing UI.
- No removal of existing A2A Payment v0 API.

## Decisions

### Decision: Payment Service Owns Lifecycle Orchestration

Payment Service owns quote, intent creation, policy evaluation command handling, approval state, settlement adapter dispatch, receipt/proof persistence, receipt queries, transition queries, proof queries, and snapshots.

Kernel no longer owns new production payment orchestration. Existing kernel coordinator code becomes a deprecated compatibility anchor that remains searchable for later migration.

### Decision: Keep Payment Policy As Strategy

`PaymentPolicyEngine` remains a Strategy boundary. The runtime-host provider composes it before settlement adapter execution. This keeps budget, approval, region, optional module, and future compliance checks replaceable without hardcoding provider names or business workflows.

### Decision: Use Command + State + Memento

All service operations enter as typed commands, then flow through `ServiceCommand`. `PaymentIntentState` remains the canonical State guard. Quote snapshots, intent transitions, receipts, execution proofs, and service snapshots are Mementos used for replay, audit, and dispute evidence.

### Decision: Runtime-Host Provider Is A Mediator, Not A Macro-Service

The Payment Service provider coordinates policy, adapter, store, state validation, trace, and logging. It must not own Store/Entitlement rules, Web3/EVM execution, A2A message formatting, gateway routing, or application workflow.

### Decision: SDK Is The Upper-Consumer Boundary

Web, CLI, Gateway, Application Framework, and future agent-facing APIs must use `SystemPaymentClient` or `SystemFacade`. They must not construct `PaymentSystemServiceProvider`, `A2ACoordinator`, `PaymentStore`, or concrete adapter strategies.

### Decision: Built-In Local Simulation Is The Only S10 Adapter

S10 may provide a deterministic local simulated payment adapter for no-network tests and local development. It must be clearly marked simulation-only in metadata and logs. External providers, enterprise billing, Web3, EVM, wallet signing, and chain settlement remain future adapter/plugin/optional-module work.

### Decision: Trace And Redaction Are Mandatory

Mutating commands require `TraceContext`. Logs and trace/audit events include bounded identifiers, operation, status, reason code, quote id, intent id, session/task scope, amount, asset code, and timestamps.

They must not include private keys, wallet secrets, provider credentials, raw signed payloads, API keys, raw provider responses, prompt bodies, raw package bytes, encrypted payloads, or unbounded user input.

## Patterns

- Facade: Payment Service and SDK `SystemPaymentClient` hide provider/runtime details.
- Mediator: runtime-host provider coordinates adapter, policy, store, observer, and state transitions.
- Strategy: payment adapter, policy engine, approval policy, receipt/proof issuance, and future optional-module availability checks are replaceable.
- Command: every service call has a typed command and command name.
- State: `PaymentIntentState` guards lifecycle transitions.
- Memento: quote, transition, receipt, proof, and snapshot artifacts support replay and audit.
- Observer: structured trace/audit/log records are emitted for lifecycle nodes.
- Adapter / Bridge: local simulated adapter and future providers sit behind provider-neutral service contracts.
- Null Object: unavailable SDK client/provider fails closed for payment-required commands and returns structured unavailable snapshots for read-only calls.
- Specification: trace, scope, amount, transition, adapter availability, and redaction rules are centralized before provider dispatch.

## Risks / Trade-Offs

- Risk: Runtime-host provider accidentally reuses kernel `A2ACoordinator`, preserving macro-kernel ownership.
  Mitigation: implement provider orchestration directly over policy/store/adapter strategies; mark kernel coordinator deprecated.

- Risk: Payment Service becomes a broad commerce service.
  Mitigation: keep Store/Entitlement, marketplace, billing UI, Web3, and EVM out of S10 scope.

- Risk: Payment trace leaks secrets.
  Mitigation: require redacted DTOs and bounded logs; reject suspicious metadata keys in admission specs.

- Risk: Payment unavailable blocks ordinary applications.
  Mitigation: only payment-required commands fail closed; ordinary task flows and free/open local applications must remain unaffected.

- Risk: Duplicate payment paths confuse future consumers.
  Mitigation: SDK clients become the new production path; deprecated kernel APIs remain only as migration anchors.

## Migration Plan

1. Add OpenSpec and validate strict deltas.
2. Add `macaca-proto::payment_service` DTOs and tests.
3. Add runtime-host admission specifications.
4. Add runtime-host Payment Service provider with local simulated adapter.
5. Add SDK `SystemPaymentClient` and `SystemFacade` accessor.
6. Register built-in Payment Service in Web startup as composition root only.
7. Mark kernel coordinator/adapter helpers deprecated.
8. Update Route C governance and allowlist.
9. Run focused tests, dependency boundary tests, workspace check, hardcode scan, and GitNexus detect changes.

## Rollback

- Disable Payment Service registration while keeping DTOs and unavailable SDK client.
- Keep deprecated kernel A2A coordinator as compatibility fallback.
- Preserve existing payment store artifacts.
- Revert Web composition-root registration without affecting ordinary task/session/trace flows.

