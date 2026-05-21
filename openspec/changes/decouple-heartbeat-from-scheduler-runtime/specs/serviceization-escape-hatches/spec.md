## ADDED Requirements

### Requirement: Gates Shall Reject Scheduler-Owned Heartbeat Cadence

Serviceization gates SHALL reject new production code that makes Scheduler
Service the owner of Heartbeat native cadence, heartbeat profiles, heartbeat
coalescing, heartbeat gates, or heartbeat run lifecycle.

#### Scenario: Scheduler implements heartbeat native cadence

- **GIVEN** a change adds production Scheduler code that evaluates Heartbeat native profile cadence
- **WHEN** serviceization escape-hatch gates run
- **THEN** the gates fail with guidance to move heartbeat cadence into `service.heartbeat` and runtime-host HeartbeatLane

### Requirement: Gates Shall Reject Shell-Owned Heartbeat Timers

Serviceization gates SHALL reject Web, CLI, frontend, SDK, and application code
that constructs production heartbeat timers, heartbeat loops, heartbeat
providers, heartbeat profiles, or heartbeat coalescing logic.

#### Scenario: Frontend adds heartbeat timer

- **GIVEN** a production frontend file creates a recurring timer for heartbeat execution
- **WHEN** serviceization escape-hatch gates run
- **THEN** the gates fail with guidance to route heartbeat behavior through Heartbeat service adapters

### Requirement: Gates Shall Reject Application-Specific Heartbeat Actions

Serviceization gates SHALL reject heartbeat implementations that branch on
application names, workflow names, agent role names, provider names, model
names, driver names, gateway names, chain names, payment names, or
business-domain strings.

#### Scenario: Heartbeat special-cases a business workflow

- **GIVEN** a heartbeat implementation adds a branch for a specific application workflow or business domain
- **WHEN** serviceization escape-hatch gates run
- **THEN** the gates fail because Heartbeat must dispatch only provider-neutral typed service commands through declared capabilities
