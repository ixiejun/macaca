## Context

Scheduler and Heartbeat were introduced as provider-neutral autonomy services.
The current contract permits recurring heartbeat behavior to be represented as
Scheduler jobs that enqueue heartbeat wake commands. That design keeps cron
logic out of Web/CLI shells, but it still couples natural heartbeat cadence to
Scheduler due-run materialization and makes the schedule-management UI expose
heartbeat as if it were normal application recurring work.

Macaca OS needs true 7x24 autonomous operation. Agents need independent
heartbeat cadence for health, memory consolidation, resumable work inspection,
fixed agent duties, and recovery checks. Those actions must be generic typed
service commands, not app-specific code inside Heartbeat.

## Goals / Non-Goals

- Goal: make Scheduler and Heartbeat sibling services coordinated by
  runtime-host.
- Goal: give Heartbeat native cadence/profile ownership without moving timers
  into the microkernel or shells.
- Goal: keep all heartbeat work traceable, auditable, policy-gated, and
  provider-neutral.
- Goal: prevent application schedule-management UI from presenting heartbeat as
  a Scheduler target.
- Non-goal: implement application-specific memory, task, or notification logic.
- Non-goal: build a heartbeat management UI in this change.
- Non-goal: remove Scheduler or application schedule APIs.

## Decisions

### Decision: Use runtime-host dual autonomy lanes

`AutonomySupervisor` SHALL own two independent lanes:

- `SchedulerLane` ticks `service.scheduler`, materializes due runs, leases
  scheduled runs, and dispatches provider-neutral scheduled targets.
- `HeartbeatLane` ticks `service.heartbeat`, resolves heartbeat scopes,
  evaluates coalescing and gates, dispatches provider-neutral heartbeat actions,
  and records heartbeat mementos.

The lanes share lifecycle, shutdown, sanitized observability, and provider
composition through runtime-host, but they do not own each other's semantics.

### Decision: Heartbeat owns native cadence and profiles

Heartbeat SHALL expose provider-neutral heartbeat profiles. A profile binds a
typed scope identity to cadence policy, gates, safe action declarations, last
tick evidence, next eligible tick, and bounded diagnostics. Profiles may target
agent, application, system, session, or recovery scopes through typed
identities. Implementations must not hardcode app names, agent roles, workflow
names, provider names, model names, driver names, gateway names, chain names,
payment names, or business-domain strings.

### Decision: Heartbeat actions are typed service commands

Heartbeat SHALL NOT implement memory consolidation, task execution, review, or
business behavior directly. When a heartbeat passes gates, it dispatches a
provider-neutral command to another service boundary, such as memory, task,
execution, or diagnostics. Heartbeat records only safe action summaries,
lifecycle states, trace identifiers, audit identifiers, and gate outcomes.

### Decision: Scheduler-driven heartbeat wake is compatibility-only

If `HeartbeatWake` Scheduler targets remain in DTOs during migration, they SHALL
be documented as internal/runtime compatibility and SHALL NOT be exposed as a
normal application schedule-management target. Native heartbeat cadence must not
depend on Scheduler jobs.

### Decision: Frontend schedule UI remains Scheduler-only

The schedule-management UI SHALL browse and manage Scheduler jobs. It SHALL NOT
offer `Heartbeat wake` as a normal target kind. A future heartbeat UI must use
focused Heartbeat routes and show heartbeat profiles/runs separately.

## Alternatives Considered

- Keep recurring heartbeat as Scheduler jobs. Rejected because it preserves the
  semantic coupling and keeps UI/application users thinking heartbeat is a
  schedule target.
- Use Scheduler as an internal timer adapter hidden behind Heartbeat. Rejected
  as the long-term model because Heartbeat liveness would still depend on
  Scheduler availability, though a temporary compatibility path may exist.
- Add a microkernel timer primitive now. Deferred because it risks moving
  concrete runtime behavior into the kernel before multiple services require a
  shared lower-level primitive.

## Trace, Audit, and Logging

Every lane tick, profile evaluation, gate result, action dispatch, run state
transition, lane degradation, shutdown, and recovery decision SHALL emit
sanitized logs and replayable trace/audit evidence. Observability must not
include raw prompts, raw provider payloads, secrets, manifests, package bytes,
WASM bytes, credentials, private keys, raw signatures, or unbounded output.

## Migration Plan

1. Update OpenSpec contracts to define native Heartbeat cadence and dual
   runtime-host lanes.
2. Add tests for Heartbeat cadence without Scheduler jobs and Scheduler due-run
   processing without Heartbeat cadence.
3. Implement provider-neutral heartbeat profile DTOs and local provider cadence
   state.
4. Split runtime-host supervisor behavior into Scheduler and Heartbeat lane
   modules or narrow lane structs.
5. Remove heartbeat target exposure from schedule-management UI.
6. Strengthen serviceization gates and documentation.

## Risks / Trade-Offs

- Risk: Heartbeat becomes a hidden task runner.
  Mitigation: heartbeat actions are typed commands routed to service
  boundaries and audited as safe summaries.
- Risk: supervisor code grows too large.
  Mitigation: lane modules must stay focused and separately testable.
- Risk: existing compatibility tests depend on Scheduler-targeted heartbeat
  wakes.
  Mitigation: retain an internal compatibility path while native cadence becomes
  the required runtime behavior.
- Risk: users lose an easy manual heartbeat test.
  Mitigation: use future heartbeat diagnostics rather than schedule creation to
  test heartbeat.
