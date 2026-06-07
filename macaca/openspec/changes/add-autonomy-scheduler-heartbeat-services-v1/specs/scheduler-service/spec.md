# scheduler-service Specification

## ADDED Requirements

### Requirement: Scheduler Service Ownership

The system SHALL expose scheduled autonomous execution as a provider-neutral
`service.scheduler` system service, not as microkernel logic, shell logic,
frontend logic, or application-specific code.

#### Scenario: Runtime host registers a scheduler provider

Given the runtime host bootstraps service providers
When it registers scheduler capability
Then the provider is registered through the service runtime as `service.scheduler`
And the microkernel does not construct the concrete scheduler provider
And Web, CLI, frontend, and application code do not own scheduler semantics

#### Scenario: Scheduler capability is absent

Given no scheduler provider is installed or enabled
When a caller invokes scheduler commands
Then the service runtime returns a structured unavailable result
And the result includes trace and audit correlation
And the system does not silently fake successful scheduling

### Requirement: Scheduler Service Descriptor and Lifecycle

The Scheduler Service SHALL provide a service descriptor, lifecycle state,
health command, and snapshot command through the standard service runtime.

#### Scenario: Caller inspects scheduler health

Given a scheduler provider is registered
When a caller requests scheduler health
Then the response includes provider-neutral status, lifecycle state, capability
flags, and safe diagnostic summaries
And the response does not reveal provider secrets, raw payloads, or application
business data

### Requirement: Scheduler Typed Command Contract

The Scheduler Service SHALL accept typed command DTOs and return typed result
DTOs for job registration, updates, pause, resume, deletion, manual trigger,
job lookup, run lookup, run listing, health, and snapshots.

#### Scenario: Shell creates a scheduled job through the facade

Given a shell caller has a scheduler facade client
When it registers a scheduled job
Then the facade sends a typed scheduler command through the service runtime
And the scheduler returns a typed result with job identity, lifecycle state,
trace correlation, and audit correlation
And the shell does not construct providers, timers, stores, or queues

### Requirement: Provider-Neutral Schedule Definitions

The Scheduler Service SHALL support provider-neutral schedule definitions for
one-shot, interval, and cron-expression schedules without binding the OS
contract to a specific cron library or storage backend.

#### Scenario: Caller registers an interval schedule

Given a caller submits an `Every` schedule with a bounded interval
When the scheduler validates the job definition
Then it stores a provider-neutral schedule descriptor
And it records time-zone, clock, missed-run, retry, lease, and audit metadata
And it rejects invalid or unbounded schedule definitions with structured errors

#### Scenario: Caller registers a cron-expression schedule

Given a caller submits a cron-expression schedule
When the scheduler validates the job definition
Then it records the expression as provider-neutral schedule metadata
And provider-specific parser details remain hidden behind the scheduler provider

### Requirement: Missed-Run and Stagger Policy

The Scheduler Service SHALL define missed-run handling and deterministic
stagger policy as explicit strategies on job definitions.

#### Scenario: Provider restarts after missed ticks

Given an active job missed one or more scheduled ticks during provider downtime
When the scheduler recalculates due work
Then it applies the job's missed-run policy
And it records whether runs were skipped, fired once, or caught up within a
bounded catch-up limit
And every decision is traceable and auditable

### Requirement: Scheduler Job Lifecycle State Machine

The Scheduler Service SHALL model job definitions with an explicit lifecycle
state machine including draft, active, paused, disabled, and deleted states.

#### Scenario: Caller pauses an active job

Given a job is active
When a caller pauses the job through a typed command
Then the scheduler transitions the job to paused state
And no new due runs are materialized for that job while it remains paused
And the transition is recorded in sanitized audit evidence

### Requirement: Scheduler Run Lifecycle State Machine

The Scheduler Service SHALL model each materialized run separately from the job
definition with explicit queued, leased, running, succeeded, failed, cancelled,
skipped, and expired states.

#### Scenario: Due run completes successfully

Given an active job has a due run
When a provider leases the run and dispatches the generic target command
Then the run transitions through leased and running states
And the run transitions to succeeded after the target command reports success
And each transition records trace, audit, and bounded diagnostic evidence

### Requirement: Lease-Based Dispatch

The Scheduler Service SHALL use lease-based dispatch and explicit concurrency
policy instead of relying on shell polling or hidden in-memory loops.

#### Scenario: Worker loses its lease

Given a provider instance acquired a run lease
When the lease expires before completion
Then the scheduler marks the run expired or recoverable according to policy
And it prevents duplicate active dispatch beyond the declared concurrency policy
And it records lease acquisition, expiry, and recovery evidence

### Requirement: Retry and Backoff Policy

The Scheduler Service SHALL make retry and backoff behavior explicit, bounded,
and auditable for each scheduled job.

#### Scenario: Target command fails

Given a scheduled run dispatches a target command
When the target command fails with a retryable error
Then the scheduler applies the job's bounded retry policy
And it records the next retry time, attempt count, and sanitized failure class
And it transitions the run to failed when retry policy is exhausted

### Requirement: Provider-Neutral Target Commands

The Scheduler Service SHALL dispatch only provider-neutral target command
categories and SHALL NOT branch on application, workflow, provider, driver,
model, gateway, chain, payment, or business-domain names.

#### Scenario: Scheduler dispatches application work

Given a job targets an application-declared capability
When the job becomes due
Then the scheduler dispatches a generic application command through declared
capability boundaries
And it does not inspect application payloads to make business-specific routing
decisions

### Requirement: Scheduler Snapshots and Run History

The Scheduler Service SHALL provide snapshots and run-history queries that are
safe for shells, applications, diagnostics, and audit replay.

#### Scenario: Caller lists recent runs

Given a caller has permission to inspect scheduler history
When it lists recent runs for a job
Then the scheduler returns bounded run summaries, lifecycle states, timestamps,
trace identifiers, and audit identifiers
And the response omits raw secrets, raw prompts, provider payloads, and
unbounded application output

### Requirement: Scheduler Trace, Policy, and Audit

Every Scheduler Service command SHALL carry or derive trace context, evaluate
policy before side effects, and emit sanitized audit records for key execution
nodes.

#### Scenario: Policy denies job registration

Given a caller submits a job definition outside its allowed scope
When scheduler policy evaluates the request
Then the service denies the command before storing or dispatching work
And the result includes a structured denied state with trace and audit
correlation
