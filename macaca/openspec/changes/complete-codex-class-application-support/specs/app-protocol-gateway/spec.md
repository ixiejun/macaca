## ADDED Requirements

### Requirement: Bidirectional App Protocol Gateway
The system SHALL provide `service.app_protocol` as a shell/gateway adapter for
bidirectional clients such as IDE, desktop, terminal, websocket, stdio, or unix
socket clients.

#### Scenario: Initialize protocol connection
- **WHEN** a client opens a protocol connection
- **THEN** the gateway SHALL require initialization with client metadata before
  accepting other requests
- **AND** it SHALL return sanitized platform and capability metadata without
  exposing provider secrets

#### Scenario: Backpressure and overload
- **WHEN** protocol ingress or outbound notification queues are saturated
- **THEN** the gateway SHALL reject new work with a structured retryable error
- **AND** it SHALL log queue saturation with trace and connection metadata

### Requirement: Gateway Calls Focused Clients Only
The app protocol gateway SHALL adapt transport messages into focused SDK or
SystemFacade calls and SHALL NOT own interaction, file, process, sandbox,
approval, plugin, MCP, skill, tool, Git, review, or diagnostics semantics.

#### Scenario: Thread start through protocol
- **WHEN** a client calls a thread start method through the gateway
- **THEN** the gateway SHALL call the interaction focused client
- **AND** all thread state SHALL be owned by `service.interaction`

#### Scenario: Tool event notification
- **WHEN** a service emits a tool, process, approval, item, or diagnostics event
- **THEN** the gateway SHALL translate it into the client protocol format
- **AND** it SHALL preserve trace and audit refs while bounding payloads

### Requirement: Protocol Health and Subscription Lifecycle
The system SHALL expose protocol health, subscription creation, subscription
closure, and notification delivery lifecycle with traceable state.

#### Scenario: Last subscriber disconnects
- **WHEN** the last protocol subscriber leaves a thread
- **THEN** the gateway SHALL close only the subscription state
- **AND** it SHALL leave thread lifecycle decisions to `service.interaction`
