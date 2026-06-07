## ADDED Requirements

### Requirement: Heartbeat Agent Bridge Dependency Direction
Dependency-boundary gates SHALL preserve the layer direction for heartbeat
agent execution.

#### Scenario: Runtime-host bridge uses service boundaries
- **WHEN** Runtime Host dispatches heartbeat agent work
- **THEN** it SHALL depend on provider-neutral DTOs and ServiceRuntime calls
- **AND** it SHALL NOT import Web, frontend, concrete app-specific modules, or
  concrete provider implementations outside approved composition roots.
