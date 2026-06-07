## ADDED Requirements

### Requirement: Web server startup uses builder facade

The web server SHALL provide a `WebServerBuilder` startup path and keep `start_server` as a deprecated compatibility facade.

#### Scenario: Existing CLI starts web server

- **WHEN** existing code calls `start_server(port)`
- **THEN** the function delegates to the builder-based startup path
- **AND** the HTTP routes, CORS behavior, app startup, persistence, and executor registration remain compatible

#### Scenario: New code starts web server

- **WHEN** new code starts the web server
- **THEN** it can use `WebServerBuilder::new().port(port).serve()`
- **AND** it does not need to know the internal bootstrap assembly order

### Requirement: Web event forwarding primitives are additive

The web crate SHALL expose event forwarding primitives that can represent EventLog and SSE delivery without changing existing live event behavior in the first implementation.

#### Scenario: Event forwarding primitive exists

- **WHEN** web code needs a future event forwarding boundary
- **THEN** `TraceEventForwarder` and `TraceEventNormalizer` are available as web-local primitives
- **AND** current SSE and EventLog payload behavior remains unchanged until explicitly migrated

### Requirement: Session replay primitive is additive

The web crate SHALL expose a session replay state primitive without changing current session reconstruction behavior in the first implementation.

#### Scenario: Replay state is constructed

- **WHEN** web code constructs `SessionReplayState`
- **THEN** it can track session id and optional cursor state
- **AND** existing `session.rs` replay behavior remains unchanged until explicitly migrated

### Requirement: Chat mediator primitive is additive

The web crate SHALL expose a chat session mediator primitive without replacing `post_chat_v2` in the first implementation.

#### Scenario: Mediator shell is available

- **WHEN** future web code migrates chat orchestration
- **THEN** `ChatSessionMediator` can hold web state and serve as the migration target
- **AND** existing `post_chat_v2` behavior remains unchanged in this change

### Requirement: Deprecated web entrypoints remain discoverable

Deprecated web Rust entrypoints SHALL remain present and SHALL NOT be deleted during gradual migration.

#### Scenario: Deprecated startup entrypoint

- **WHEN** code searches for deprecated web startup APIs
- **THEN** `start_server` remains present with a deprecation marker
- **AND** the marker points to the builder-based replacement
