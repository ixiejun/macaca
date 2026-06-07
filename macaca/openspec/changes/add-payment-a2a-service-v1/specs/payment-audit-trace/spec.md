## ADDED Requirements

### Requirement: Payment Service SHALL emit bounded trace and audit records

Payment Service SHALL emit structured logs and trace/audit-compatible events for provider lifecycle, quote, intent creation, policy evaluation, approval, transition, adapter execution, settlement, receipt recording, proof recording, read queries, snapshot, and failure nodes.

#### Scenario: Payment lifecycle event contains bounded identity context

- **WHEN** Payment Service emits a lifecycle event
- **THEN** the event SHALL include bounded service id, command name, trace id, requester id, provider id, capability id, quote id when present, intent id when present, operation, status, reason code when present, session/task scope when present, and timestamp
- **AND** it SHALL be compatible with existing trace/event log consumers

#### Scenario: Provider lifecycle is auditable

- **WHEN** Payment Service provider starts, stops, accepts a command, rejects a command, or reports a snapshot
- **THEN** it SHALL log bounded service id, command when present, trace id when present, health/status, and sanitized diagnostics
- **AND** the log SHALL be sufficient to understand the lifecycle without reading raw provider payloads

### Requirement: Payment trace and audit payloads SHALL be redacted

Payment trace, audit, logs, receipts, proofs, snapshots, and diagnostics SHALL exclude sensitive or unbounded material.

#### Scenario: Secrets are not emitted

- **WHEN** Payment Service processes quote, approval, settlement, receipt, proof, failure, or snapshot data
- **THEN** emitted trace/audit/log payloads SHALL NOT include private keys, wallet secrets, provider credentials, API keys, raw signed payloads, raw provider responses, raw prompt bodies, raw package bytes, encrypted payload, raw tool payload, or unbounded user input
- **AND** payloads SHALL retain bounded identifiers needed for audit

#### Scenario: Suspicious metadata is rejected or redacted

- **WHEN** payment command metadata contains keys indicating secrets or credentials
- **THEN** Payment Service admission SHALL reject or redact those fields before trace/audit/log emission
- **AND** the decision SHALL be structured and auditable without copying the secret value

### Requirement: Payment mementos SHALL support replay and dispute evidence

Payment Service SHALL persist quote snapshots, ordered payment intent transitions, receipts, and execution proofs as mementos that can be replayed by session, task, or intent scope.

#### Scenario: Receipt and proof are queryable after settlement

- **WHEN** a payment intent settles successfully
- **THEN** Payment Service SHALL persist a receipt and execution proof
- **AND** callers SHALL be able to query them by payment intent id

#### Scenario: Ordered transitions reconstruct lifecycle

- **WHEN** a payment intent moves through creation, quote, approval, execution, settlement, receipt, failure, or dispute-possible states
- **THEN** Payment Service SHALL persist ordered transitions with operation, status, timestamp, reason/error metadata when present, and bounded identity context
- **AND** replay SHALL not require provider credentials or raw adapter payloads

### Requirement: Payment Service SHALL provide sanitized snapshots

Payment Service SHALL provide snapshots that expose service health, configured capability counts, adapter availability status, and sanitized diagnostics without leaking secrets.

#### Scenario: Snapshot omits sensitive details

- **WHEN** a caller requests a Payment Service snapshot
- **THEN** the snapshot SHALL include service id, health/status, adapter availability, counts, timestamps, and diagnostics
- **AND** it SHALL NOT expose provider credentials, private keys, wallet data, raw receipts beyond redacted receipt views, raw proofs beyond redacted proof views, or raw adapter configuration

