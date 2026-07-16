## ADDED Requirements

### Requirement: Macaca SHALL provide Workflow Schedule as a serviceized industrial pack

Macaca SHALL provide `pack.workflow.schedule.v1` as a provider-neutral industrial pack for durable schedules, recurrence, one-shot timers, intervals, cron-like schedules, RFC 5545-style recurrence, exclusions, timezone/DST policy, preview, due firing, misfire handling, catch-up, backfill, pause/resume, deletion, trigger history, and snapshots. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.workflow.schedule.v1` as required and the workflow schedule service is registered, healthy, entitled, policy-admissible, recurrence-capable, timezone-compatible, and command-compatible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, recurrence support, timezone/DST support, misfire/catch-up/overlap policies, policy template, availability, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing raw payloads, prompts, secrets, provider payloads, or unbounded schedule history

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.workflow.schedule.v1` as required but provider, command support, permission, entitlement, resource, recurrence support, timezone support, or policy is absent
- **THEN** admission SHALL block readiness with structured unavailable, unsupported, denied, or degraded-recurrence diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to another provider, or fake scheduled trigger success

#### Scenario: Optional declaration is degraded
- **WHEN** an application declares `pack.workflow.schedule.v1` as optional and the pack is unavailable or command-limited
- **THEN** admission SHALL produce an explicit degraded effective capability report with bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Workflow Schedule SHALL expose supplier-grade provider-neutral commands

`pack.workflow.schedule.v1` SHALL expose typed commands for create, update, pause, resume, delete, inspect, preview, next occurrences, fire due, backfill, cancel trigger, get history, and snapshot operations.

#### Scenario: Schedule creation validates recurrence
- **WHEN** a caller invokes `workflow_schedule.create` with schedule spec, action reference, owner scope, and idempotency key
- **THEN** Macaca SHALL validate recurrence, timezone, action target, policy, and idempotency key
- **AND** it SHALL create a durable schedule without firing work directly

#### Scenario: Preview computes bounded occurrences
- **WHEN** a caller invokes `workflow_schedule.preview`
- **THEN** Macaca SHALL compute bounded occurrences for the requested range/count using recurrence, exclusions, timezone, and DST policy
- **AND** preview SHALL NOT create tasks or service commands

#### Scenario: Next occurrences enforce count limits
- **WHEN** a caller invokes `workflow_schedule.next_occurrences`
- **THEN** Macaca SHALL enforce max count and time range limits
- **AND** it SHALL return scheduled time, logical time, data interval, timezone evidence, and recurrence diagnostics

#### Scenario: Fire due generates idempotent triggers
- **WHEN** a caller invokes `workflow_schedule.fire_due`
- **THEN** Macaca SHALL compute due occurrences, apply misfire/catch-up/overlap policy, and create idempotent `ScheduleTriggerRecord` entries
- **AND** trigger actions SHALL call declared task/service capabilities through the canonical service path

#### Scenario: Misfire policy is explicit
- **WHEN** a scheduled occurrence is missed beyond the configured threshold
- **THEN** Macaca SHALL apply explicit misfire policy such as fire now, skip, reschedule next, coalesce, bounded catch-up, or fail schedule
- **AND** the outcome SHALL be traceable in trigger history

#### Scenario: Backfill is bounded
- **WHEN** a caller invokes `workflow_schedule.backfill`
- **THEN** Macaca SHALL require bounded range, max trigger count, catch-up mode, overlap policy, resource budget, and approval when configured
- **AND** excessive backfill SHALL return backfill-too-large or quota diagnostics before creating triggers

#### Scenario: Pause and resume preserve definition
- **WHEN** a caller invokes pause or resume
- **THEN** Macaca SHALL preserve schedule definition, record lifecycle event, and apply resume catch-up policy
- **AND** paused schedules SHALL not fire due triggers unless explicitly backfilled

#### Scenario: Delete cancels future triggers
- **WHEN** a caller invokes `workflow_schedule.delete`
- **THEN** Macaca SHALL mark schedule deleted and cancel future pending triggers according to policy
- **AND** history SHALL remain available within retention policy

#### Scenario: History returns replay evidence
- **WHEN** a caller invokes `workflow_schedule.get_history`
- **THEN** Macaca SHALL return bounded schedule lifecycle and trigger events with evidence ids
- **AND** replay SHALL not require raw action payloads or unbounded logs

### Requirement: Workflow Schedule DTOs SHALL model recurrence, timezone, trigger policy, and generated records

The pack SHALL define provider-neutral DTOs for schedules, schedule specs, recurrence, timezone policy, misfire policy, overlap policy, trigger records, backfill requests, and structured errors.

#### Scenario: Recurrence records include inclusions and exclusions
- **WHEN** a schedule uses calendar recurrence
- **THEN** `ScheduleRecurrence` SHALL model RRULE-like rules, RDATE/EXDATE inclusions/exclusions, DTSTART, count/until, and validation diagnostics
- **AND** unsupported recurrence features SHALL be explicit

#### Scenario: Timezone policy records DST behavior
- **WHEN** a schedule uses local time
- **THEN** `ScheduleTimezonePolicy` SHALL include IANA zone id, tzdb version, DST gap/fold strategy, local-time resolution, stale-database behavior, and provenance
- **AND** DST-unresolved cases SHALL not silently choose an occurrence

#### Scenario: Trigger record is idempotent
- **WHEN** a due occurrence is fired
- **THEN** `ScheduleTriggerRecord` SHALL include schedule id, scheduled time, actual fire time, logical time, data interval, misfire class, jitter, idempotency key, action reference, and state
- **AND** duplicate firing SHALL be prevented by idempotency policy

#### Scenario: Structured errors are stable across providers
- **WHEN** providers return invalid recurrence, invalid timezone, DST unresolved, misfire blocked, overlap blocked, backfill too large, schedule paused, trigger conflict, quota, provider failure, or version mismatch states
- **THEN** Macaca SHALL map them to stable `WorkflowScheduleError` variants
- **AND** provider-specific diagnostics SHALL be sanitized and bounded

### Requirement: Workflow Schedule SHALL enforce permission, policy, resource, entitlement, approval, and idempotency

Every command in `pack.workflow.schedule.v1` SHALL run through permission, policy, resource, entitlement, approval, metering, idempotency, and redaction decorators before provider side effects.

#### Scenario: Missing permission denies before provider dispatch
- **WHEN** an application invokes a command without required scope such as `workflow.schedule.read`, `workflow.schedule.write`, `workflow.schedule.control`, `workflow.schedule.fire`, `workflow.schedule.backfill`, or `workflow.schedule.admin`
- **THEN** Macaca SHALL return a typed denied result before invoking the concrete provider
- **AND** the audit event SHALL include the bounded missing-scope code

#### Scenario: Trigger action requires declared capability
- **WHEN** a schedule fires an action target
- **THEN** Macaca SHALL verify the target task/service capability is declared, available, policy-allowed, and idempotent
- **AND** missing target capability SHALL return denied or unavailable diagnostics without fake firing

#### Scenario: High-frequency schedule is quota bounded
- **WHEN** recurrence frequency, trigger count, backfill count, history size, active schedules, or retained snapshots exceed policy
- **THEN** Macaca SHALL return quota-exceeded diagnostics before provider mutation
- **AND** resource counters SHALL be emitted in sanitized trace evidence

#### Scenario: Backfill requires approval when configured
- **WHEN** a backfill could create many triggers or invoke sensitive actions
- **THEN** Macaca SHALL require approval evidence according to policy
- **AND** missing approval SHALL deny before creating triggers

### Requirement: Workflow Schedule SHALL preserve canonical service runtime execution

All callable operations SHALL traverse the canonical Macaca service path: application declaration, admission/effective capability projection, SDK/facade command construction, service runtime dispatch, decorators, provider adapter, structured result, schedule lifecycle events, trigger action dispatch through declared capabilities, trace/audit evidence, and replayable snapshot. SDK helpers SHALL NOT construct providers or create alternate execution paths.

#### Scenario: Command succeeds through the canonical path
- **WHEN** a declared and policy-allowed command is invoked
- **THEN** Macaca SHALL route it through SDK/facade helpers into service runtime dispatch and the active schedule provider adapter
- **AND** trace evidence SHALL show declaration, admission, policy, entitlement, resource, provider selection, schedule/trigger state, command result, and replay pointer events

#### Scenario: Provider is absent
- **WHEN** no provider is registered for `pack.workflow.schedule.v1`
- **THEN** the unavailable provider SHALL return structured unavailable diagnostics
- **AND** SDK discovery SHALL report unavailable state while preserving the same provider-neutral command/result contract

#### Scenario: Provider supports only a subset
- **WHEN** the active provider supports interval schedules but not RRULE, backfill, or coalescing misfires
- **THEN** SDK discovery SHALL mark unsupported commands/features as non-callable
- **AND** direct invocation SHALL return typed unsupported diagnostics without falling through to application-specific logic

#### Scenario: Provider is replaced
- **WHEN** a built-in, remote workflow-engine, plugin, mock, or unavailable provider is selected
- **THEN** callers SHALL observe the same provider-neutral DTO contract
- **AND** OS-layer code SHALL identify only provider class, descriptor version, recurrence support, and capability metadata in traces rather than branching on provider names

### Requirement: Workflow Schedule SHALL expose industrial SDK discovery and developer documentation

SDK discovery for `pack.workflow.schedule.v1` SHALL expose pack metadata, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, recurrence support matrix, timezone/DST support, misfire/catch-up/overlap policy support, backfill limits, policy templates, examples, diagnostics, compatibility, and documentation links. The implementation SHALL provide detailed developer documentation under `docs/developer-packs/workflow/schedule.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.workflow.schedule.v1`
- **THEN** it SHALL return command namespace `workflow_schedule.*`, supported commands, required scopes, recurrence/timezone/misfire/backfill support, examples, lifecycle, health, diagnostics, compatibility metadata, and documentation URL
- **AND** examples SHALL use generic synthetic schedules rather than application-specific workflows or provider-name routing

#### Scenario: Documentation covers app developer usage
- **WHEN** a developer opens `docs/developer-packs/workflow/schedule.md`
- **THEN** the guide SHALL explain manifest declarations, scopes, command DTOs, result DTOs, one-shot/interval/cron/RRULE schedules, timezone/DST behavior, preview, pause/resume, misfire/catch-up/backfill, overlap, task integration, history, unavailable diagnostics, trace/audit behavior, and replay workflow
- **AND** it SHALL include minimal app-facing examples using canonical SDK calls

#### Scenario: Documentation covers provider authors
- **WHEN** a provider author reads the guide
- **THEN** it SHALL document descriptor fields, recurrence evaluator responsibilities, schedule/trigger/backfill state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy
- **AND** it SHALL forbid application-specific business routing in provider-neutral layers

### Requirement: Workflow Schedule observability SHALL be sanitized, replayable, and auditable

The pack SHALL emit sanitized trace, audit, health, schedule lifecycle, trigger, backfill, misfire, snapshot, and replay evidence for declaration, admission, creation, update, pause, resume, preview, trigger computation, trigger fire, trigger cancellation, backfill, misfire handling, deletion, unavailable state, and snapshot recording.

#### Scenario: Successful command emits bounded evidence
- **WHEN** a schedule command succeeds
- **THEN** Macaca SHALL emit sanitized events containing pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when available, schedule id, version, recurrence hash, timezone id/hash, scheduled time, logical time, trigger id hash, misfire class, overlap policy, catch-up class, policy decision, latency, and resource counters
- **AND** it SHALL exclude raw payloads, prompts, secrets, provider payloads, and unbounded history

#### Scenario: Snapshot records schedule summaries
- **WHEN** the service runtime records a schedule snapshot
- **THEN** the snapshot SHALL include schedule state counts, next occurrence summaries, paused schedules, backfill summaries, misfire summaries, policy hash, provider health, unavailable diagnostics, and replay pointers
- **AND** it SHALL exclude raw action payloads, prompts, credentials, and unbounded output

#### Scenario: Replay verifies trigger lifecycle
- **WHEN** a session or task is replayed after refresh or restart
- **THEN** Macaca SHALL reconstruct the schedule command, trigger computation, fire, misfire, backfill, and action dispatch chain from bounded trace/audit evidence
- **AND** replay diagnostics SHALL prove the commands used the canonical service runtime path

### Requirement: Workflow Schedule implementation SHALL preserve Macaca architecture boundaries

The `pack.workflow.schedule.v1` implementation SHALL keep concrete schedule providers behind service/runtime provider adapters. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, provider-specific, schedule-specific, task-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan imports
- **WHEN** dependency-boundary gates scan the implementation
- **THEN** they SHALL find no concrete schedule provider, recurrence engine, workflow engine, task executor, or shell scheduling semantic owner in the microkernel, SDK, shells, or generic application framework
- **AND** provider construction SHALL appear only in approved runtime composition roots or plugin/remote provider registration paths

#### Scenario: No-direct-provider-call gate scans commands
- **WHEN** no-direct-provider-call gates scan schedule commands
- **THEN** every callable operation SHALL be reachable only through descriptor-owned service registrations and typed service runtime dispatch
- **AND** SDK helpers SHALL only build canonical service commands

#### Scenario: Pack remains separate from neighboring workflow packs
- **WHEN** architecture review compares workflow packs
- **THEN** schedule SHALL own durable schedule state, recurrence, trigger computation, misfire/catch-up/backfill, overlap policy, trigger history, and schedule snapshots
- **AND** task execution, approval, delegation, review, recovery, calendar invitations, and application-specific reminders SHALL remain owned by their respective packs or services
