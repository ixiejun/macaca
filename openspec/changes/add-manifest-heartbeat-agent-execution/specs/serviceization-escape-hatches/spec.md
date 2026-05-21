## ADDED Requirements

### Requirement: Heartbeat Agent Execution Ownership Gate
Serviceization gates SHALL prevent presentation shells, Scheduler, or Heartbeat
providers from owning direct heartbeat agent execution semantics.

#### Scenario: Forbidden ownership regression
- **WHEN** production code introduces heartbeat-agent execution dispatch
- **THEN** the dispatch owner SHALL be runtime-host strategy code that calls
  typed services
- **AND** Web, frontend, Scheduler providers, and Heartbeat providers SHALL NOT
  define application-specific heartbeat execution semantics.
