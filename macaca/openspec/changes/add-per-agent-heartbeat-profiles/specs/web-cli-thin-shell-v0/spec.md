## MODIFIED Requirements

### Requirement: Heartbeat Diagnostics Shall Use Heartbeat Routes

Web, CLI, and frontend shells SHALL use Heartbeat-focused service routes or
facade clients for heartbeat profiles, heartbeat runs, and heartbeat
diagnostics. For manifest-declared heartbeat agents, shells SHALL render
per-agent profile identity and SHALL send profile edits as Heartbeat profile
commands rather than editing raw manifests or creating Scheduler jobs.

#### Scenario: Operator edits one agent heartbeat interval
- **GIVEN** a Heartbeat Operations UI lists multiple agent profiles for one application
- **WHEN** the operator edits one profile interval or cooldown
- **THEN** the shell sends a traced Heartbeat profile update command for that profile id
- **AND** it refreshes Heartbeat profile/run summaries from Heartbeat routes
- **AND** it does not mutate raw application manifests or Scheduler jobs
