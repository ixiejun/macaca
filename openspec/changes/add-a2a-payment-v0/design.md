# Design: A2A Payment v0

## Context

Phase 08 established Store/Entitlement v0: paid packages and paid capability calls can be expressed, authorized, metered, and audited. Phase 09 adds the collaboration and payment control plane needed when one agent requests work from another agent or provider capability that may require quoted terms and budget-controlled settlement.

A2A Payment v0 must be provider-neutral. It cannot bake in one payment network, chain, billing provider, currency, or application workflow. Real settlement remains future work. This phase establishes contracts, policy, persistence, traceability, and a local simulated adapter that exercises the same control flow without moving real value.

## Goals

- Define provider-neutral A2A/payment protocol contracts in `macaca-proto`.
- Add kernel-level A2A payment coordination facade and policy primitives without concrete provider logic.
- Add payment persistence contracts for quote, intent state, receipt, execution proof, and audit queries.
- Add task-level A2A request context that can be attached to existing goal/task flows.
- Add a local simulated adapter for deterministic no-network tests.
- Enforce budget and approval policy before any payment adapter execution.
- Emit structured logs and trace/audit events for every meaningful payment transition.
- Preserve `RC-GOAL-001` and `RC-TRACE-001`.

## Non-Goals

- No real payment provider integration.
- No chain, wallet, EVM, or Web3 module integration.
- No autonomous real-money spending by agents.
- No full migration of existing task/tool/skill calls to A2A.
- No kernel-owned payment provider implementation.

## Superpowers Brainstorm Summary

### Problem

A2A collaboration requires discovery, quote negotiation, budget policy, approval, execution proof, settlement status, receipt recording, and dispute evidence. Treating it as a normal tool call would lose payment semantics and auditability.

### Options Considered

1. **Central A2A Payment Facade + pluggable adapter strategies (recommended)**
   - Pros: provider-neutral, auditable, additive, compatible with future payment modules.
   - Cons: requires new contracts across multiple crates.
2. **Inline payment logic inside task execution**
   - Pros: quick demo path.
   - Cons: duplicates policy, hides spending decisions in task code, weak audit.
3. **Extend Store/Entitlement contracts directly**
   - Pros: reuses Phase 08 structures.
   - Cons: conflates entitlement with transactional payment state.

### Recommended

Use Facade + Strategy + Command + State + Mediator + Memento + Observer. The kernel coordinates policy and audit boundaries, while provider-specific settlement remains behind adapter traits and is unavailable unless explicitly configured in later phases.

## Architecture Decisions

### 1. Protocol Contracts (`macaca-proto/src/a2a.rs`)

Introduce data-only contracts:

- `AgentIdentity`
- `RemoteCapabilityDescriptor`
- `QuoteRequest`
- `QuoteResponse`
- `PaymentAmount`
- `PaymentAsset`
- `PaymentTerms`
- `PaymentIntent`
- `PaymentIntentState`
- `BudgetPolicy`
- `ApprovalPolicy`
- `PaymentReceipt`
- `ExecutionProof`
- `A2AError`

Pattern: **Value Object + Command + State**

The contracts must preserve unknown/custom payment rails, asset codes, billing units, and metadata. Amount and asset fields must not assume a single chain, token, or fiat currency.

### 2. Payment Policy (`macaca-kernel/src/payment_policy.rs`)

Define policy primitives:

- budget policy evaluation
- explicit approval requirements
- local simulation auto-approval threshold
- unavailable state for unconfigured real adapters
- policy decision metadata for trace/audit

Pattern: **Strategy + Specification**

Default policy denies real payment without explicit approval. Local simulated payment may be auto-approved only when below the configured threshold and test/local adapter metadata marks it as simulation.

### 3. A2A Coordinator Facade (`macaca-kernel/src/a2a.rs`)

Add the runtime-facing facade:

- quote remote/local capability
- create payment intent
- evaluate budget
- evaluate approval
- execute local simulated adapter
- record receipt
- emit state transition logs/events

Pattern: **Facade + Mediator + State**

The facade coordinates but does not own concrete provider implementation. Provider behavior is behind `A2AProtocolAdapter` strategies.

### 4. Payment Store (`macaca-persist/src/payment_store.rs`)

Add a repository boundary for immutable payment artifacts:

- quote snapshots
- intent state transitions
- payment receipts
- execution proofs
- query by session/task/intent

Pattern: **Repository + Memento**

The initial in-memory/test adapter may be implemented first, with future persistent backends using the same contract.

### 5. Task A2A Context (`macaca-task/src/a2a_task.rs`)

Add task-level request structures that carry:

- session id
- task id
- requester identity
- provider identity
- remote capability descriptor
- quote and payment intent references
- trace context metadata

Pattern: **Command + Adapter**

Existing task execution remains unchanged until explicit consumers migrate.

### 6. Trace and Audit

Every quote, budget decision, approval decision, intent transition, adapter execution, failure, settlement simulation, receipt recording, and dispute-possible state must be observable.

Pattern: **Observer**

Trace/audit payloads must include bounded metadata only: requester id, provider id, capability id, quote id, intent id, amount, asset, operation, status, session/task scope when available, timestamp, and error code when present. Logs must not include secrets, private keys, raw credentials, or raw encrypted payloads.

## Payment Intent Lifecycle

Required canonical states:

```text
created -> quoted -> pending_approval -> approved -> executing -> settled -> receipt_recorded
created -> quoted -> rejected
approved -> failed -> dispute_possible
```

The implementation may represent states as string-backed values for forward compatibility, but helper constructors and transition validation should cover the canonical states.

## Compatibility and Regression

- Preserve goal/task no-network baseline (`RC-GOAL-001`).
- Preserve trace/event compatibility (`RC-TRACE-001`).
- Do not require real LLM, browser, frontend server, payment provider, wallet, Web3 node, or EVM module for Phase 09 tests.
- Keep local simulated adapter deterministic and provider-neutral.

## Risk and Mitigation

- **Risk:** agents spend without approval.
  - Mitigation: default policy denies real payment unless explicit approval exists; tests cover over-budget and approval-required cases.
- **Risk:** kernel becomes a concrete payment implementation.
  - Mitigation: keep provider execution behind adapter traits; kernel owns coordination and policy only.
- **Risk:** A2A contracts hardcode a payment rail.
  - Mitigation: string-backed rails/assets and metadata; hardcode scan in verification.
- **Risk:** receipt or dispute evidence cannot be audited.
  - Mitigation: payment store persists quote, terms, transition, receipt, and proof mementos with session/task indexes.
- **Risk:** local simulation hides real policy gaps.
  - Mitigation: local adapter must use the same facade, policy, state, persistence, and trace pipeline as future adapters.

## Verification Plan

- `openspec validate add-a2a-payment-v0 --strict`
- `cargo test -p macaca-proto a2a`
- `cargo test -p macaca-kernel a2a_payment`
- `cargo test -p macaca-persist payment_store`
- `cargo test -p macaca-task a2a_task`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cargo check --workspace`
- hardcode scan over new A2A/payment files
- `npx gitnexus detect-changes --repo agent`
