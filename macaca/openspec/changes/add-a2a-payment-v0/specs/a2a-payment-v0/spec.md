## ADDED Requirements

### Requirement: Macaca SHALL define provider-neutral A2A payment contracts

Macaca SHALL define provider-neutral protocol contracts for A2A agent identity, remote capability descriptors, quote requests/responses, payment terms, payment intents, budget/approval policy inputs, receipts, execution proof, and structured A2A/payment errors.

#### Scenario: A2A quote, intent, and receipt fixtures round trip through serde

- **WHEN** quote, intent, terms, receipt, state, and execution proof fixtures are serialized and deserialized
- **THEN** decoded contracts SHALL preserve requester, provider, capability, amount, asset, rail, terms, lifecycle, proof, receipt, and metadata fields
- **AND** unknown/custom rails, asset codes, billing units, and metadata SHALL remain structured without panic

#### Scenario: A2A contracts remain provider-neutral

- **WHEN** A2A contracts are consumed by kernel policy, persistence, task context, or local simulated adapters
- **THEN** the contracts SHALL NOT depend on concrete payment provider names, chain names, app names, workflow names, gateway names, driver names, model names, or business-specific routing

### Requirement: Macaca SHALL enforce budget and approval policy before payment execution

Macaca SHALL evaluate budget and approval policy before any A2A payment adapter is allowed to execute.

#### Scenario: Over-budget intent is rejected before adapter execution

- **WHEN** a payment intent amount exceeds the applicable budget policy
- **THEN** the intent SHALL be rejected with a structured over-budget reason
- **AND** no payment adapter SHALL execute
- **AND** the rejection SHALL be logged and auditable

#### Scenario: Real payment adapter requires explicit approval

- **WHEN** a payment intent targets a non-simulated real adapter and explicit approval is missing
- **THEN** the intent SHALL enter or remain in an approval-required state
- **AND** adapter execution SHALL NOT occur

#### Scenario: Local simulated payment may auto-approve under threshold

- **WHEN** a local simulated payment intent is under the configured auto-approval threshold
- **THEN** policy MAY approve the intent for deterministic no-network testing
- **AND** the approval SHALL be traceable as simulation-only

#### Scenario: Missing real adapter is structured unavailable

- **WHEN** a real payment adapter is requested but not configured
- **THEN** Macaca SHALL return structured unavailable
- **AND** existing local applications and non-payment task flows SHALL continue unaffected

### Requirement: Macaca SHALL provide A2A coordinator facade with pluggable adapter strategy

Macaca SHALL provide a kernel-level A2A coordinator facade that coordinates quote, payment intent, budget policy, approval policy, adapter execution, settlement status, receipt recording, and audit without embedding concrete payment provider behavior in the kernel.

#### Scenario: Local A2A simulation produces a receipt

- **WHEN** a requester asks a local simulated provider for a quote and the approved intent executes
- **THEN** the coordinator SHALL produce a structured payment receipt
- **AND** the receipt SHALL reference the quote, intent, requester, provider, capability, amount, asset, session/task scope when available, and timestamp

#### Scenario: Invalid intent transition is rejected

- **WHEN** a caller attempts an invalid payment intent lifecycle transition
- **THEN** the coordinator or state validator SHALL reject the transition with a structured error
- **AND** the invalid transition SHALL be logged without mutating the canonical state

### Requirement: Macaca SHALL persist payment mementos and receipts

Macaca SHALL provide a payment persistence contract that stores immutable quote snapshots, intent state transitions, receipts, and execution proofs, and supports query by session id, task id, and payment intent id.

#### Scenario: Receipt can be queried by session and task

- **WHEN** a payment receipt is recorded with session and task scope
- **THEN** persistence queries by session id or task id SHALL return the receipt
- **AND** the receipt SHALL include enough identity context for audit and replay

#### Scenario: Intent transitions are ordered and auditable

- **WHEN** an intent moves through quote, approval, execution, settlement, receipt, failure, or dispute states
- **THEN** each transition SHALL be stored in chronological order
- **AND** each transition SHALL include operation, status, timestamp, and reason/error metadata when present

### Requirement: Macaca SHALL expose task-level A2A request context without changing existing task execution

Macaca SHALL provide task-level A2A request/context contracts that can carry requester, provider, remote capability, quote, intent, session, task, and trace metadata while preserving existing task execution behavior until explicit migration.

#### Scenario: Existing no-network task pipeline remains compatible

- **WHEN** Phase 09 contracts are added
- **THEN** existing no-network goal/task baseline SHALL continue to pass
- **AND** existing task execution SHALL NOT require a payment provider, Web3 module, EVM module, wallet, browser, frontend server, or external network

#### Scenario: A2A task context is serializable

- **WHEN** an A2A task request context is serialized and deserialized
- **THEN** requester, provider, capability, quote reference, intent reference, session scope, task scope, and trace metadata SHALL be preserved

### Requirement: Macaca SHALL emit trace and audit events for A2A payment lifecycle

Macaca SHALL emit structured logs and trace/audit-compatible events for quote, budget decision, approval decision, payment intent transition, adapter execution, settlement simulation, receipt recording, failure, and dispute-possible states.

#### Scenario: Payment event contains auditable identity context

- **WHEN** a payment lifecycle event is emitted
- **THEN** the event payload SHALL include requester id, provider id, capability id, quote id, intent id, amount, asset, operation, status, session/task scope when available, timestamp, and error code when present
- **AND** the event SHALL be compatible with existing trace/event log paths

#### Scenario: Payment logs exclude sensitive material

- **WHEN** payment policy, adapter, settlement, receipt, or failure logs are emitted
- **THEN** logs SHALL NOT include secrets, private keys, credentials, raw encrypted payloads, or full provider credentials
- **AND** logs SHALL retain bounded identifiers needed for audit

### Requirement: Macaca SHALL preserve Route C regressions for Phase 09

A2A Payment v0 SHALL be additive and SHALL preserve baseline goal/task and trace behavior.

#### Scenario: Phase 09 regression checks pass

- **WHEN** Phase 09 verification runs
- **THEN** the implementation SHALL preserve `RC-GOAL-001` and `RC-TRACE-001`
- **AND** existing YAML applications, task flows, and trace/event flows SHALL continue to compile and run through current paths until explicitly migrated

### Requirement: Macaca SHALL provide detailed English comments and structured logs for A2A Payment v0 code

All new Phase 09 Rust code SHALL include detailed English comments and structured logs for key execution nodes.

#### Scenario: Maintainer can audit A2A payment decisions from code and logs

- **WHEN** a maintainer reads new A2A/payment modules and observes runtime logs/events
- **THEN** comments SHALL explain public type/function purpose, state transitions, policy rules, adapter boundaries, and runtime behavior
- **AND** logs SHALL capture quote, budget, approval, execution, settlement, receipt, failure, and dispute nodes without sensitive material
