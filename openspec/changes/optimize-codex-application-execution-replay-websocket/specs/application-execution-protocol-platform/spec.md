## MODIFIED Requirements

### Requirement: EventLog SHALL be the durable source of truth for application execution events

Macaca SHALL persist application execution events before realtime delivery. Realtime/SSE/WebSocket delivery SHALL be an Observer projection over durable EventLog/session events, not the authority for execution state. App-owned UIs and shells SHALL first reconstruct state from replay/current-state after refresh, then subscribe to WebSocket or equivalent realtime increments from the last durable cursor.

#### Scenario: Browser disconnect does not stop backend execution

- **GIVEN** an application execution has started and a frontend subscriber is receiving realtime events
- **WHEN** the browser tab closes, the iframe unloads, or the subscriber disconnects without sending a cancel control command
- **THEN** backend execution SHALL continue according to provider policy
- **AND** provider events SHALL continue to be appended to the durable session event store
- **AND** no browser-local event buffer SHALL be required for execution progress.

#### Scenario: Replay reconstructs execution after refresh

- **GIVEN** an application execution has persisted events for a session/run
- **WHEN** a shell or application UI reconnects with the same session id and replay cursor
- **THEN** Macaca SHALL replay events in deterministic order from the requested cursor
- **AND** Macaca SHALL provide current state derived from persisted events
- **AND** pending approvals, active controls, latest provider heartbeat, latest checkpoint, summarized LLM/tool steps, and terminal outcome SHALL match the event history.

#### Scenario: WebSocket increments start after durable replay

- **GIVEN** a Codex-class app-owned UI refreshes with a known session id and optional run id
- **WHEN** the UI loads
- **THEN** it SHALL call replay/current-state before opening realtime transport
- **AND** it SHALL open a WebSocket subscription using the latest durable event cursor
- **AND** each WebSocket message SHALL refer to an EventLog row that was already persisted by `service.application_execution`
- **AND** the WebSocket connection SHALL NOT own provider execution, EventLog append, current-state projection, approval semantics, or cancellation semantics.
