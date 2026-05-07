## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, `docs/superpowers/plans/2026-05-07-macaca-os-route-c-microkernel-ecosystem-plan.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-09-a2a-payment-v0.md`.
- [x] 1.2 Review Phase 08 Store/Entitlement contracts and current task/trace/persist/kernel service boundaries before implementation.
- [x] 1.3 Run GitNexus impact before modifying each selected existing symbol; warn before editing any HIGH or CRITICAL impact symbol.

## 2. A2A Protocol Contracts

- [x] 2.1 Add `macaca/crates/macaca-proto/src/a2a.rs` with provider-neutral A2A/payment contracts.
- [x] 2.2 Export A2A contracts from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.3 Define `AgentIdentity`, `RemoteCapabilityDescriptor`, `QuoteRequest`, `QuoteResponse`, `PaymentAmount`, `PaymentAsset`, `PaymentTerms`, `PaymentIntent`, `PaymentIntentState`, `BudgetPolicy`, `ApprovalPolicy`, `PaymentReceipt`, `ExecutionProof`, and `A2AError`.
- [x] 2.4 Support unknown/custom payment rails, asset codes, billing units, and metadata without provider/chain hardcoding.
- [x] 2.5 Add serde roundtrip tests for quote, intent, terms, receipt, state, and custom rail/asset fixtures.

## 3. Budget and Approval Policy

- [x] 3.1 Add `macaca/crates/macaca-kernel/src/payment_policy.rs`.
- [x] 3.2 Implement explicit approval requirement for real payment adapters.
- [x] 3.3 Implement budget limit evaluation and over-budget rejection.
- [x] 3.4 Implement local simulation auto-approval under configured threshold.
- [x] 3.5 Return structured unavailable when a real payment adapter is not configured.
- [x] 3.6 Add structured logs for policy evaluation start/pass/reject/unavailable.

## 4. A2A Coordinator Facade

- [x] 4.1 Add `macaca/crates/macaca-kernel/src/a2a.rs`.
- [x] 4.2 Define `A2APaymentFacade`, `A2ACoordinator`, and `A2AProtocolAdapter` trait boundaries.
- [x] 4.3 Implement quote -> intent -> budget -> approval -> local simulated execution -> receipt pipeline.
- [x] 4.4 Implement canonical payment intent lifecycle transitions and reject invalid transitions.
- [x] 4.5 Add structured logs for quote, intent creation, approval, execution, settlement simulation, failure, and receipt recording.
- [x] 4.6 Add detailed English comments for public contracts, state rules, adapter boundaries, and non-goals.

## 5. Payment Persistence Contract

- [x] 5.1 Add `macaca/crates/macaca-persist/src/payment_store.rs` with payment persistence traits and in-memory/test adapter.
- [x] 5.2 Export payment store contract from `macaca-persist`.
- [x] 5.3 Persist quote snapshots, intent state transitions, receipts, and execution proofs.
- [x] 5.4 Support query by session id, task id, and payment intent id.
- [x] 5.5 Add tests for quote write/read, state transition ordering, receipt query, and execution proof retrieval.

## 6. Task-Level A2A Integration

- [x] 6.1 Add `macaca/crates/macaca-task/src/a2a_task.rs`.
- [x] 6.2 Export task-level A2A request/context contracts from `macaca-task`.
- [x] 6.3 Attach session/task/requester/provider/capability/quote/intent trace metadata without changing existing task execution behavior.
- [x] 6.4 Add tests proving A2A task request context is serializable and does not require real payment.

## 7. Trace and Audit Events

- [x] 7.1 Emit trace/audit-compatible payment events for quote, budget decision, approval decision, state transition, adapter execution, settlement simulation, receipt recording, failure, and dispute-possible states.
- [x] 7.2 Ensure event payloads include requester id, provider id, capability id, quote id, intent id, amount, asset, operation, status, session/task scope when available, timestamp, and error code when present.
- [x] 7.3 Ensure logs and events exclude secrets, private keys, credentials, and raw encrypted payloads.
- [x] 7.4 Add tests proving receipt and transition events are trace/audit compatible.
- [x] 7.5 Run hardcode scan over new A2A/payment files for app/workflow/provider/driver/gateway/model/chain/business constants.

## 8. Regression and Verification

- [x] 8.1 Run `openspec validate add-a2a-payment-v0 --strict`.
- [x] 8.2 Run `cargo test -p macaca-proto a2a`.
- [x] 8.3 Run `cargo test -p macaca-kernel a2a_payment`.
- [x] 8.4 Run `cargo test -p macaca-persist payment_store`.
- [x] 8.5 Run `cargo test -p macaca-task a2a_task`.
- [x] 8.6 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 8.7 Run `cargo check --workspace`.
- [x] 8.8 Run `npx gitnexus detect-changes --repo agent` before committing and verify affected flows align with Phase 09 scope.
