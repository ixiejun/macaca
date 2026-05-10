## ADDED Requirements

### Requirement: Macaca SHALL expose Payment / A2A lifecycle through a Payment Service

Macaca SHALL expose provider-neutral Payment Service commands for A2A quote, payment intent creation, policy evaluation, approval, settlement, receipt query, transition query, proof query, and payment snapshot.

#### Scenario: Payment service command set is available through ServiceRuntime

- **WHEN** the built-in Payment Service is registered and started
- **THEN** callers SHALL be able to dispatch quote, intent creation, policy evaluation, approval, settlement, receipt query, transition query, proof query, and snapshot commands through the system service path
- **AND** the service descriptor SHALL remain independent of concrete payment provider names, chain names, wallet names, app names, workflow names, gateway names, driver names, model names, or business-specific routing

#### Scenario: Payment service does not replace Store or Entitlement

- **WHEN** Payment Service settles an A2A quote or records a receipt
- **THEN** it SHALL NOT define Store package install semantics or Entitlement authorization semantics
- **AND** Store / Entitlement services MAY call Payment Service in future phases through provider-neutral client contracts

### Requirement: Macaca SHALL require trace and scope before mutating payment commands execute

Payment Service SHALL validate trace context, requester identity, provider identity, capability scope, amount, and lifecycle transition before any mutating payment command reaches adapter execution.

#### Scenario: Mutating payment command without trace is rejected

- **WHEN** quote, intent creation, policy evaluation, approval, or settlement is requested without `TraceContext`
- **THEN** Payment Service SHALL reject the command before adapter execution
- **AND** the rejection SHALL be structured and logged with bounded service and command identifiers

#### Scenario: Missing A2A scope is rejected

- **WHEN** quote or intent processing lacks requester, provider, or capability identity
- **THEN** Payment Service SHALL reject the command before adapter execution
- **AND** the rejection SHALL NOT mutate payment store state

### Requirement: Macaca SHALL mediate payment lifecycle with replaceable strategies

Payment Service SHALL mediate policy, adapter execution, persistence, lifecycle transition, and trace/audit emission through replaceable Strategy, Adapter, State, Memento, and Observer boundaries.

#### Scenario: Local simulated adapter produces auditable receipt

- **WHEN** a local simulated quote is approved and settled under policy
- **THEN** Payment Service SHALL produce a structured payment receipt and execution proof
- **AND** it SHALL persist quote, transition, receipt, and proof mementos
- **AND** it SHALL mark simulation-only behavior in bounded metadata or diagnostics

#### Scenario: Invalid transition is rejected

- **WHEN** a payment command attempts a lifecycle transition not allowed by `PaymentIntentState`
- **THEN** Payment Service SHALL reject the transition with structured error information
- **AND** it SHALL NOT append the invalid transition or execute an adapter

### Requirement: Macaca SHALL provide structured unavailable behavior for Payment Service

Payment Service and its unavailable implementation SHALL fail closed for payment-required commands while preserving non-payment task and application flows.

#### Scenario: Payment service missing fails closed for settlement

- **WHEN** a caller attempts to settle or approve a payment while Payment Service is unavailable
- **THEN** the system SHALL return structured unavailable or denied status
- **AND** ordinary no-payment application, session, task, and trace flows SHALL continue unaffected

#### Scenario: Read-only unavailable snapshot is safe

- **WHEN** a caller asks for a Payment Service snapshot while the service is unavailable
- **THEN** the system MAY return an unavailable snapshot with diagnostics
- **AND** it SHALL NOT fake successful payment capability

