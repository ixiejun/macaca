## ADDED Requirements

### Requirement: Runtime Events Persist Before SSE

The system SHALL persist session-visible runtime lifecycle events to EventLog before sending any matching Web SSE event.

#### Scenario: Live runtime event is emitted

- **WHEN** a runtime adapter emits a session-visible skill, tool, or data retrieval lifecycle event
- **THEN** the event SHALL be appended to EventLog under the session id first
- **AND** the SSE frame SHALL be sent only after the append returns
- **AND** the event SHALL remain queryable through the existing session events endpoint

### Requirement: Runtime Event Payloads Are Sanitized

The system SHALL keep runtime event payloads bounded and safe for Web, CLI, EventLog, and replay consumers.

#### Scenario: Runtime event includes service or skill metadata

- **WHEN** the event is persisted
- **THEN** the payload SHALL include only bounded metadata such as lifecycle stage, agent, service id, provider id, status, counts, hashes, trace id, or error summary
- **AND** the payload SHALL NOT include raw prompts, full skill bodies, raw provider payloads, manifests, WASM bytes, package bytes, credentials, private keys, or unbounded output
