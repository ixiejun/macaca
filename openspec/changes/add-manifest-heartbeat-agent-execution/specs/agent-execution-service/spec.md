## ADDED Requirements

### Requirement: Heartbeat Execution Intent
Agent Execution Service SHALL support a provider-neutral heartbeat execution
intent for manifest-declared heartbeat work.

#### Scenario: Heartbeat context includes HEARTBEAT source
- **WHEN** Agent Execution receives a heartbeat intent command
- **AND** Agent Context returns source evidence for `HEARTBEAT.md`
- **THEN** Agent Execution MAY run the target agent through the normal service
  execution path.

#### Scenario: Heartbeat context lacks HEARTBEAT source
- **WHEN** Agent Execution receives a heartbeat intent command
- **AND** Agent Context does not return source evidence for `HEARTBEAT.md`
- **THEN** Agent Execution SHALL return a structured skipped result with reason
  `heartbeat_profile_missing`
- **AND** it SHALL NOT invoke a model or tool.
