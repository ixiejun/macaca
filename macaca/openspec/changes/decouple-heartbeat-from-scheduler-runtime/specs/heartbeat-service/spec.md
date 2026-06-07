## ADDED Requirements

### Requirement: Heartbeat Native Cadence

The Heartbeat Service SHALL own native heartbeat cadence for provider-neutral
agent, application, system, session, and recovery scopes without requiring
Scheduler Service jobs to trigger recurring heartbeat ticks.

#### Scenario: Native heartbeat tick becomes due

- **GIVEN** local autonomy is enabled and a heartbeat profile has a due native cadence
- **WHEN** runtime-host HeartbeatLane ticks the Heartbeat Service
- **THEN** Heartbeat evaluates the profile, coalescing, policy, resource, cooldown, busy, and provider-health gates
- **AND** Heartbeat records bounded trace and audit evidence for the decision
- **AND** Scheduler due-run materialization is not required for the heartbeat tick

### Requirement: Heartbeat Profiles

The Heartbeat Service SHALL expose provider-neutral heartbeat profiles that
bind a typed scope identity to cadence policy, gate policy, safe action
declarations, last tick evidence, next eligible tick, and bounded diagnostics.

#### Scenario: Agent heartbeat profile is inspected

- **GIVEN** a caller has permission to inspect heartbeat diagnostics
- **WHEN** it requests heartbeat profiles for an agent scope
- **THEN** the service returns bounded profile summaries, cadence status, gate summaries, trace identifiers, and audit identifiers
- **AND** the response omits raw prompts, raw provider payloads, secrets, and application business data

### Requirement: Heartbeat Action Dispatch

The Heartbeat Service SHALL dispatch heartbeat work only as provider-neutral
typed commands to declared service boundaries; it SHALL NOT implement
application-specific task planning, memory consolidation, review,
notification, or business behavior directly.

#### Scenario: Heartbeat dispatches memory maintenance action

- **GIVEN** a heartbeat profile declares a generic memory maintenance action through capabilities and policy
- **WHEN** the heartbeat tick passes gates
- **THEN** Heartbeat dispatches a typed command to the memory service boundary with trace context
- **AND** Heartbeat records only a safe action summary, lifecycle state, trace identifier, and audit identifier
- **AND** Heartbeat does not branch on application name, workflow name, provider name, model name, driver name, or business-domain strings

## MODIFIED Requirements

### Requirement: Scheduler Integration

Scheduler integration with Heartbeat SHALL be compatibility-only or explicit
cross-service signaling. Native recurring heartbeat cadence SHALL NOT be
represented as application-facing Scheduler jobs. If a Scheduler target can
still request a heartbeat wake during migration, it SHALL be internal/runtime
compatibility and Heartbeat SHALL remain the owner of coalescing, gates,
cadence, profiles, and wake lifecycle.

#### Scenario: Scheduler compatibility wake is received

- **GIVEN** an internal runtime compatibility path emits a Scheduler-targeted heartbeat wake
- **WHEN** the wake reaches Heartbeat
- **THEN** Heartbeat treats it as a typed wake intent and evaluates coalescing and gates
- **AND** native heartbeat cadence remains available without Scheduler due-run materialization

#### Scenario: Application schedule UI creates a recurring job

- **GIVEN** an application-facing schedule-management caller creates a recurring job
- **WHEN** it selects target kind
- **THEN** the caller cannot select Heartbeat native cadence as a normal Scheduler target
- **AND** Heartbeat profile management remains separate from Scheduler job management
