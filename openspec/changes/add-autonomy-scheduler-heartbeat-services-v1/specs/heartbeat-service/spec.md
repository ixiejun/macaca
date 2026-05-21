# heartbeat-service Specification

## ADDED Requirements

### Requirement: Heartbeat Service Ownership

The system SHALL expose wake-loop coordination as a provider-neutral
`service.heartbeat` system service, not as microkernel logic, shell logic,
frontend logic, or application-specific code.

#### Scenario: Runtime host registers a heartbeat provider

Given the runtime host bootstraps service providers
When it registers heartbeat capability
Then the provider is registered through the service runtime as
`service.heartbeat`
And the microkernel does not construct the concrete heartbeat provider
And Web, CLI, frontend, and application code do not own heartbeat semantics

#### Scenario: Heartbeat capability is absent

Given no heartbeat provider is installed or enabled
When a caller invokes heartbeat commands
Then the service runtime returns a structured unavailable result
And the result includes trace and audit correlation
And the system does not silently fake wake-loop progress

### Requirement: Heartbeat Service Descriptor and Lifecycle

The Heartbeat Service SHALL provide a service descriptor, lifecycle state,
health command, and snapshot command through the standard service runtime.

#### Scenario: Caller inspects heartbeat snapshot

Given a heartbeat provider is registered
When a caller requests a heartbeat snapshot
Then the response includes provider-neutral lifecycle state, recent wake
summary, active gate summary, last run summary, and safe diagnostic evidence
And the response does not reveal provider secrets, raw prompts, raw payloads, or
application business data

### Requirement: Heartbeat Typed Command Contract

The Heartbeat Service SHALL accept typed command DTOs and return typed result
DTOs for wake requests, wake cancellation, run lookup, run listing, health, and
snapshots.

#### Scenario: Application requests a heartbeat wake

Given an application has declared access to heartbeat capability
When it requests a wake through the facade
Then the facade sends a typed heartbeat command through the service runtime
And the heartbeat service returns a typed wake result with lifecycle state,
trace correlation, and audit correlation
And the application does not construct timers, queues, providers, or OS wake
loops

### Requirement: Provider-Neutral Wake Intents

The Heartbeat Service SHALL support provider-neutral wake intents for scheduled
ticks, event signals, immediate requests, manual requests, and recovery
requests.

#### Scenario: Scheduler emits a heartbeat tick

Given a scheduler job targets a heartbeat wake command
When the scheduled job becomes due
Then the heartbeat service receives a `ScheduledTick` wake intent
And it evaluates coalescing and gates before dispatching any side effects

#### Scenario: Runtime recovery emits a wake

Given runtime-host recovers after a restart
When a provider requests recovery processing
Then the heartbeat service receives a `Recovery` wake intent
And the result explains whether the wake was accepted, coalesced, gated,
skipped, or dispatched

### Requirement: Wake Coalescing

The Heartbeat Service SHALL coalesce wake requests by provider-neutral scope so
repeated signals do not create duplicate or runaway autonomous loops.

#### Scenario: Duplicate wake arrives while one is pending

Given a wake request is already pending for a scope
When another compatible wake request arrives for the same scope
Then the heartbeat service coalesces the request
And it returns a structured coalesced result with trace and audit correlation
And it records the latest wake reason without creating duplicate active work

### Requirement: Heartbeat Gate Evaluation

The Heartbeat Service SHALL evaluate active-hours, cooldown, busy, resource,
budget, policy, provider-health, and scheduler-active gates before dispatching
heartbeat work.

#### Scenario: Cooldown gate blocks a wake

Given a wake request arrives within the scope's cooldown window
When the heartbeat service evaluates gates
Then it returns a gated result
And the result includes the safe reason class and next eligible time when
available
And no target command is dispatched

### Requirement: Heartbeat Run Lifecycle State Machine

The Heartbeat Service SHALL model heartbeat runs with explicit requested,
coalesced, gated, running, succeeded, failed, and skipped states.

#### Scenario: Accepted wake completes

Given a wake request passes all gates
When the heartbeat provider dispatches generic work
Then the run transitions to running state
And it transitions to succeeded after the generic target command completes
And each state transition records bounded trace and audit evidence

### Requirement: Heartbeat Does Not Own Task Semantics

The Heartbeat Service SHALL coordinate wake decisions and dispatch only generic
service commands through declared capabilities; it SHALL NOT implement
application-specific task planning, review, execution, notification, or business
logic.

#### Scenario: Heartbeat dispatches agent work

Given a wake request passes gates and targets agent execution
When heartbeat dispatches the work
Then it sends a provider-neutral service command to the appropriate task or
execution service boundary
And it does not inspect workflow names, model names, driver names, application
business payloads, or provider-specific data to decide behavior

### Requirement: Scheduler Integration

Recurring heartbeat ticks SHALL be represented through Scheduler Service jobs
that enqueue provider-neutral heartbeat wake commands.

#### Scenario: Recurring heartbeat is enabled

Given a caller configures recurring heartbeat behavior
When the recurring schedule is registered
Then `service.scheduler` owns schedule calculation and due-run materialization
And `service.heartbeat` owns wake coalescing, gates, and wake run lifecycle
And no shell-owned cron loop is required for heartbeat operation

### Requirement: Heartbeat Snapshots and History

The Heartbeat Service SHALL provide snapshots and bounded run-history queries
that are safe for diagnostics, shells, applications, and audit replay.

#### Scenario: Caller lists heartbeat runs

Given a caller has permission to inspect heartbeat history
When it lists heartbeat runs for a scope
Then the service returns bounded run summaries, wake intent classes, gate
summaries, lifecycle states, trace identifiers, and audit identifiers
And the response omits raw secrets, raw prompts, provider payloads, and
unbounded application output

### Requirement: Heartbeat Trace, Policy, and Audit

Every Heartbeat Service command SHALL carry or derive trace context, evaluate
policy before side effects, and emit sanitized audit records for key execution
nodes.

#### Scenario: Policy denies wake request

Given a caller submits a wake request outside its allowed scope
When heartbeat policy evaluates the request
Then the service denies the command before enqueuing or dispatching work
And the result includes a structured denied state with trace and audit
correlation
