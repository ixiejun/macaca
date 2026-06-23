## MODIFIED Requirements

### Requirement: Macaca SHALL keep trace and SSE as Observer presentation behavior

Macaca Web SHALL subscribe to trace/event sources, forward or render bounded events, and preserve replay cursors without redefining core trace semantics. Application execution history rendered by an application-owned UI SHALL come from the application execution replay/current-state projection, not from a second generic session-event replay path.

#### Scenario: Real-time trace remains live
- **WHEN** agent, task, service, driver, skill, MCP, plugin, payment, Web3, EVM, or UI events occur during an active session
- **THEN** Web SHALL forward live trace events through SSE or equivalent shell transport
- **AND** `RC-TRACE-001` SHALL remain valid

#### Scenario: Historical trace replay remains complete and non-duplicated
- **WHEN** a user refreshes or reloads a session
- **THEN** Web SHALL replay historical trace from EventLog or equivalent trace source using session-scoped cursors
- **AND** replay SHALL avoid duplicate historical/live events
- **AND** `RC-TRACE-002` SHALL remain valid

#### Scenario: Application execution UI history has one replay source
- **WHEN** an application-owned UI refreshes a running or completed application execution
- **THEN** execution timeline history SHALL be loaded from the application execution replay/current-state projection
- **AND** the UI SHALL NOT fall back to generic session-event history as an alternate execution timeline
