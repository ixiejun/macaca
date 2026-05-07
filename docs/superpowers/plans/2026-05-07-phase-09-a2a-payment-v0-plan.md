# Phase 09: A2A Payment v0 Implementation Plan

## Goal

Implement the provider-neutral A2A collaboration and payment baseline described by `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-09-a2a-payment-v0.md`.

This phase SHALL model agent identity, remote capability discovery, quote negotiation, payment intent lifecycle, budget policy, approval policy, local simulated payment, receipts, trace, and persistence. It SHALL NOT integrate real chain settlement, real payment providers, autonomous spending without approval, or provider-specific business routing.

## Design Patterns

- **Value Object:** `AgentIdentity`, `PaymentAmount`, `PaymentAsset`, `ReceiptId`, `QuoteId`, and `PaymentIntentId` should be typed, serializable values.
- **Command:** `QuoteRequest`, `PaymentIntentCommand`, `ApprovalCommand`, `A2AServiceRequest`, and settlement commands carry explicit trace and scope.
- **State:** `PaymentIntentState` models lifecycle transitions from created to quoted, approval, execution, settlement, receipt, failure, or dispute-possible states.
- **Strategy:** payment adapters, budget policies, and approval policies remain replaceable traits.
- **Facade:** kernel/runtime-facing A2A payment facade coordinates quote, budget, approval, adapter execution, persistence, and trace.
- **Mediator:** A2A coordinator coordinates requester, provider capability, task context, payment adapter, policy, and audit.
- **Memento:** quote, terms, receipt, and execution proof are persisted immutable artifacts.
- **Observer:** EventLog/payment audit receives every meaningful transition.

## Affected Crates / Files

Expected implementation files:

- `macaca/crates/macaca-proto/src/a2a.rs`
- `macaca/crates/macaca-kernel/src/a2a.rs`
- `macaca/crates/macaca-kernel/src/payment_policy.rs`
- `macaca/crates/macaca-persist/src/payment_store.rs`
- `macaca/crates/macaca-task/src/a2a_task.rs`
- `macaca/crates/macaca-kernel/tests/a2a_payment.rs`

Expected modified exports:

- `macaca/crates/macaca-proto/src/lib.rs`
- `macaca/crates/macaca-kernel/src/lib.rs`
- `macaca/crates/macaca-persist/src/lib.rs`
- `macaca/crates/macaca-task/src/lib.rs`

## Slices

### Slice 9.1: A2A protocol contracts

Add `macaca-proto/src/a2a.rs` with provider-neutral contracts:

- `AgentIdentity`
- `RemoteCapabilityDescriptor`
- `QuoteRequest`
- `QuoteResponse`
- `PaymentAmount`
- `PaymentTerms`
- `PaymentIntent`
- `PaymentIntentState`
- `BudgetPolicy`
- `ApprovalPolicy`
- `PaymentReceipt`
- `A2AProtocolAdapter` input/output data contracts where appropriate
- `A2AError`

Validation:

- `cargo test -p macaca-proto a2a`
- serde roundtrip for quote, intent, terms, receipt, unknown/custom rails/assets.

### Slice 9.2: Budget and approval policy

Add policy rules in `macaca-kernel/src/payment_policy.rs`:

- default deny for real payment adapter without explicit approval
- over-budget rejection
- auto-approval only for local simulated payment under configured threshold
- structured unavailable when adapter is not configured

Validation:

- `cargo test -p macaca-kernel a2a_payment`
- tests for over-budget, auto-approved simulated intent, and unavailable real adapter.

### Slice 9.3: Local A2A adapter and coordinator facade

Add `macaca-kernel/src/a2a.rs`:

- `A2APaymentFacade`
- `A2ACoordinator`
- `A2AProtocolAdapter` trait
- local simulated adapter
- quote -> intent -> approval -> execution -> receipt flow

Validation:

- local requester can request local provider quote
- approved intent produces receipt
- failed adapter returns structured error and dispute-possible evidence.

### Slice 9.4: Payment persistence and trace

Add `macaca-persist/src/payment_store.rs`:

- store quote snapshots
- store intent state transitions
- store receipts and execution proof
- query receipts by session/task/intent

Integrate trace:

- every quote, approval, execution, settlement, failure, and receipt transition emits structured logs
- optional EventLog bridge emits session-scoped payment events with trace context

Validation:

- `cargo test -p macaca-persist payment_store`
- receipt can be queried by session/task
- EventLog-compatible payload contains requester, provider, capability, intent, amount, status, and timestamp.

### Slice 9.5: Task integration

Add `macaca-task/src/a2a_task.rs`:

- task-level A2A request model
- attach task/session context to quote and payment intent
- keep current goal/task pipeline additive and unchanged until consumers migrate

Validation:

- no-network route C baseline still passes
- A2A task request produces a traceable intent without invoking real payment.

## Prohibited Work

- Do not integrate real payment providers, wallets, chains, enterprise billing networks, or settlement services in Phase 09.
- Do not let any agent autonomously spend real money without explicit approval.
- Do not hardcode application names, workflow names, provider names, gateway names, driver names, model names, chain names, business names, or payment provider names.
- Do not put concrete payment provider behavior into `macaca-kernel`.
- Do not emit payment state without trace/audit.
- Do not require Web3/EVM modules for ordinary A2A local simulation.

## Verification Gates

- `openspec validate add-a2a-payment-v0 --strict`
- `cargo test -p macaca-proto a2a`
- `cargo test -p macaca-kernel a2a_payment`
- `cargo test -p macaca-persist payment_store`
- `cargo test -p macaca-task a2a_task`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cargo check --workspace`
- hardcode scan over new A2A/payment files
- `npx gitnexus detect-changes --repo agent`

## GitNexus

Before implementation edits, run impact analysis for each existing symbol selected for modification. If any impact result is HIGH or CRITICAL, report the blast radius before editing. Before committing, run `npx gitnexus detect-changes --repo agent`.

## Rollback

All implementation is additive. Rollback removes new A2A/payment modules, exports, tests, and OpenSpec change artifacts. Existing Store/Entitlement v0, YAML application loading, goal pipeline, trace, and task execution should remain unaffected.
