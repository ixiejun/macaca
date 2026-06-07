## ADDED Requirements

### Requirement: Heartbeat Does Not Execute Agents Directly
Heartbeat Service SHALL own cadence, gates, wake coalescing, snapshots, and
mementos, but SHALL NOT directly execute application agents.

#### Scenario: Accepted wake remains a heartbeat memento
- **WHEN** a native heartbeat wake is accepted
- **THEN** Heartbeat Service SHALL record bounded wake/run/audit evidence
- **AND** any agent execution triggered by that wake SHALL be dispatched by
  Runtime Host through Agent Execution service.
