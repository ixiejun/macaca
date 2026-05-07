# Change: Add A2A Payment v0

## Why

Route C Phase 09 needs a provider-neutral Agent-to-Agent collaboration and payment control plane before Macaca can safely support paid cross-agent services, future MCP A2A adapters, enterprise billing, or optional Web3/EVM settlement.

Without A2A Payment v0, quote negotiation, budget checks, approval, execution proof, receipt storage, and dispute evidence would fragment across task, tool, skill, and future payment integrations. That would make agent spending hard to audit and would violate Route C's payment-service boundary.

## What Changes

- Add provider-neutral A2A/payment protocol contracts in `macaca-proto` for agent identity, remote capability descriptors, quote request/response, payment terms, payment intent lifecycle, budget/approval policy inputs, receipts, and structured errors.
- Add kernel-level A2A coordination facade and payment policy primitives without embedding concrete payment provider logic in the kernel.
- Add payment persistence contract in `macaca-persist` for quotes, intent state transitions, receipts, execution proof, and session/task-scoped queries.
- Add task-level A2A request integration in `macaca-task` so goal/task flows can carry traceable quote and payment intent context without changing existing task execution behavior.
- Add local simulated A2A adapter for protocol verification only; real payment providers remain future work and must be unavailable unless explicitly configured and approved.
- Add structured logs and trace/audit events for quote, budget decision, approval decision, intent transition, settlement simulation, receipt recording, failure, and dispute-possible states.
- Add detailed English comments for new public contracts, state transitions, policy rules, non-goals, and key runtime execution nodes.

## Impact

- Affected specs: `a2a-payment-v0`
- Affected crates: `macaca-proto`, `macaca-kernel`, `macaca-persist`, `macaca-task`, and integration tests
- Affected code areas: protocol contracts, payment policy, A2A coordinator facade, payment persistence, task A2A context, trace/audit emission, and local simulated adapter tests
- Regression matrix references: `RC-GOAL-001`, `RC-TRACE-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: Payment/A2A is a replaceable system service; kernel only coordinates policy and audit boundaries.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 09 explicitly preserves goal/task pipeline and trace behavior.
- Follows `macaca/docs/route-c-phase-template.md`: includes Superpowers brainstorm/write-plan, OpenSpec proposal/design/tasks/spec, GitNexus impact, additive implementation, targeted tests, integration smoke, detect_changes, and commit gates.
- Follows `macaca/docs/route-c-architecture-governance.md`: uses Facade, Strategy, Command, State, Mediator, Memento, and Observer patterns; every payment action must be traceable and policy-checked.

## Non-Goals

- Do not integrate real payment providers, wallets, chains, enterprise billing networks, or settlement systems in Phase 09.
- Do not bind Macaca kernel or protocol contracts to any concrete chain, payment provider, currency, application, workflow, gateway, driver, or model.
- Do not allow agents to autonomously spend real money without explicit approval.
- Do not require Web3/EVM modules for local A2A simulation or existing applications.
- Do not migrate all task/tool/skill consumers to A2A in this phase.
