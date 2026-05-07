# Phase 09 A2A Payment v0 Brainstorm

## Context

Route C Phase 09 builds on Phase 08 Store/Entitlement v0. Macaca now has provider-neutral commerce metadata, entitlement decisions, metering events, encrypted package hooks, and audit-friendly runtime guards. The next missing layer is a provider-neutral Agent-to-Agent collaboration and payment protocol that can express quote negotiation, budget checks, explicit approval, execution, settlement, receipts, and dispute evidence without coupling the OS to a concrete payment provider or chain.

This phase must obey:

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-regression-matrix.md`
- `macaca/docs/route-c-phase-template.md`
- `macaca/docs/route-c-architecture-governance.md`
- `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-09-a2a-payment-v0.md`

## Current Problem

A2A work is not a normal tool call. It requires service discovery, remote capability descriptors, quote requests, payment intent creation, budget policy, approval policy, execution proof, receipt storage, and traceable status transitions. Without a shared contract, every caller would invent its own payment state, making it hard to audit whether agents spent within budget, whether approval happened, and whether receipts can be tied back to session/task trace.

## Why This Phase

Phase 09 is the bridge between Store/Entitlement v0 and future optional Web3/EVM/payment adapters. It needs to establish the payment control plane before any real money, chain settlement, enterprise billing, or remote A2A provider is allowed into the runtime. The phase must preserve `RC-GOAL-001` and `RC-TRACE-001`.

## Design Pattern Candidates

### Option 1: Central A2A Payment Facade with pluggable adapter strategies

Use protocol contracts in `macaca-proto`, a kernel-facing coordinator facade in `macaca-kernel`, persistence contracts in `macaca-persist`, and task integration in `macaca-task`. Payment adapters are trait-based strategies.

Patterns:

- Facade for the runtime-facing A2A payment entry point.
- Strategy for payment adapters and approval policies.
- Command for quote, intent, approval, settlement, and service requests.
- State for payment intent lifecycle.
- Memento for quote, receipt, terms, and execution proof.
- Observer for EventLog/payment audit emission.

Pros:

- Provider-neutral and compatible with future enterprise billing, MCP A2A, Web3, and EVM adapters.
- Auditable because all state transitions pass through one facade.
- Additive because local simulation can validate protocol without real payment.
- Keeps payment business logic out of the microkernel.

Cons:

- Requires contracts across multiple crates.
- Needs clear boundaries so kernel does not become a payment provider.

### Option 2: Implement A2A payment directly inside task execution

Task execution would create quotes, intents, approval decisions, and receipts inline.

Pros:

- Fastest local path for a demo.
- Minimal new surface area initially.

Cons:

- Duplicates payment state if skill/MCP/plugin paths need A2A later.
- Harder to enforce budget and approval consistently.
- Poor fit for Route C because payment is a replaceable system service.

### Option 3: Treat Phase 09 as a Store-only extension

Extend Phase 08 entitlement objects to represent quote, payment intent, and receipt.

Pros:

- Reuses existing commerce types and audit concepts.
- Reduces the number of new modules.

Cons:

- Conflates entitlement state with transactional payment state.
- Makes future A2A collaboration harder to express.
- Risks overloading Store/Entitlement contracts with payment provider concerns.

## Recommendation

Choose Option 1.

The A2A payment layer should be a provider-neutral service facade with trait-based adapters. The kernel may expose policy and audit boundaries, but it must not contain concrete payment provider logic. Local simulation is the default adapter for Phase 09 so tests can validate quote -> approval -> execution -> receipt without real settlement.

## Key Risks

- **Risk: agents spend without explicit budget approval.**
  Mitigation: default policy denies real payment unless explicit approval exists; tests cover over-budget and approval-required states.

- **Risk: kernel gains payment provider logic.**
  Mitigation: kernel owns facade/policy coordination only; provider-specific behavior is behind adapter traits.

- **Risk: future chain or enterprise billing hardcodes leak into contracts.**
  Mitigation: amounts, asset codes, settlement rails, and billing units are string/value objects with provider-neutral metadata.

- **Risk: trace gaps make disputes unauditable.**
  Mitigation: every quote, approval, settlement, failure, and receipt transition emits structured logs and persistent audit records.

- **Risk: local simulation becomes a toy shortcut.**
  Mitigation: local adapter must exercise the same protocol contracts and policy pipeline used by future real adapters.

## Rollback

Because the phase is additive, rollback can remove the new A2A contracts, payment store, local adapter, and task integration without changing existing YAML application, goal, trace, or entitlement behavior. Legacy execution paths remain available until explicit migration.
