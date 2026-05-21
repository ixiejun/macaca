# Change: Add Autonomy Scheduler and Heartbeat Services V1

## Summary

Add provider-neutral Scheduler and Heartbeat service contracts so Macaca can run
scheduled, recurring, and wake-driven autonomous work without moving cron logic
into the microkernel, Web shell, CLI shell, or application-specific code.

This change is intentionally OpenSpec-first. It defines the service boundaries,
DTO expectations, lifecycle states, audit/trace requirements, SDK facade shape,
and serviceization gates before any Rust implementation lands.

## Why

Macaca is a 24/7 autonomous Agent OS. Upper-layer applications need basic
scheduled execution and heartbeat capabilities, but those capabilities must
remain generic infrastructure services rather than application-owned loops or
shell-owned cron scripts.

The Hermes Agent and OpenClaw research shows that useful autonomous systems need
both:

- Durable scheduled jobs with retry, missed-run, lease, and history semantics.
- Heartbeat wake loops that keep agents alive, coalesce wake signals, and avoid
  starting duplicate work when another run is active.

Macaca needs the same class of capability, but under the existing microkernel
constitution:

- Kernel code may own only primitives, identifiers, policy, trace, audit, and
  registry invariants.
- Runtime-host composition may register built-in providers, but concrete
  providers remain replaceable services.
- Web, CLI, frontend, and applications may request or observe scheduled work
  through facade clients, but must not own OS scheduling semantics.
- No application, workflow, provider, driver, chain, model, gateway, or business
  name may be hardcoded into OS-layer contracts.

## What Changes

- Add a new `scheduler-service` capability contract for durable scheduled jobs,
  schedule descriptors, run leasing, missed-run handling, retry/backoff,
  run-history snapshots, health, lifecycle, trace, policy, and sanitized audit.
- Add a new `heartbeat-service` capability contract for coalesced wake requests,
  heartbeat run lifecycle, active-hours/cooldown/busy gates, scheduler
  integration, recovery wake intents, trace, policy, and sanitized audit.
- Extend `sdk-system-facade` with focused Scheduler and Heartbeat clients that
  expose typed commands/results without constructing providers or runtimes.
- Extend `service-runtime` expectations so runtime-host can register built-in,
  plugin, remote, mock, and unavailable providers for the two services.
- Extend `serviceization-escape-hatches` so cron/heartbeat logic cannot be
  reintroduced into kernel code, shells, frontend code, or app-specific branches.

## Scope

### In Scope

- Service-level contracts for `service.scheduler` and `service.heartbeat`.
- Provider-neutral DTO concepts and command/result boundaries.
- Runtime-host composition-root ownership for provider registration.
- Null Object / unavailable provider behavior for absent scheduler or heartbeat
  providers.
- Trace, policy, audit, health, lifecycle, snapshot, and redaction guarantees.
- Generic design-pattern alignment for extensibility and auditability.

### Out of Scope

- Rust implementation of providers, DTOs, facade clients, or tests.
- Migration of current task execution, `/api/chat/v2`, Web UI, or CLI command
  flows.
- Application-specific workflows, reminders, delivery channels, notification
  products, cron names, provider names, driver names, model names, or business
  branches.
- Defining `HEARTBEAT.md` or any concrete file format as an OS requirement. A
  provider may use an adapter strategy later, but the OS contract remains
  provider-neutral.

## Governance Alignment

This proposal follows the Macaca OS constitutions:

- Architecture governance: scheduler and heartbeat become replaceable system
  services with descriptors, lifecycle, health, snapshots, trace, audit, policy,
  and structured errors.
- Microkernel boundaries: kernel owns only primitives and registries; concrete
  timing loops, wake queues, and execution policies live in service providers.
- Serviceization allowlist: the new services are generic infrastructure
  capabilities and explicitly reject application-specific business semantics.

## Design Patterns Considered

- Facade: focused SDK clients expose stable scheduler and heartbeat operations
  to shells and applications.
- Command: scheduled jobs and heartbeat wakes carry typed, provider-neutral
  command/result DTOs across service boundaries.
- Strategy: schedule calculation, missed-run handling, retry/backoff,
  coalescing, and provider selection remain replaceable strategies.
- Decorator: trace, policy, resource, audit, and metering wrap service calls
  without contaminating provider logic.
- State: job lifecycle, run lifecycle, and heartbeat lifecycle are explicit
  state machines.
- Observer: service events and audit records expose replayable evidence for
  autonomous operation.
- Memento: job definitions, run histories, heartbeat queues, and snapshots are
  persistable without binding the OS to one database.
- Abstract Factory: runtime-host remains the approved composition root for
  constructing service providers.
- Null Object: unavailable providers fail closed with structured responses.
- Specification: boundary gates reject forbidden cron and heartbeat ownership.

## Risks and Mitigations

- Risk: scheduler semantics could drift into shell polling or app-specific code.
  Mitigation: add explicit serviceization escape-hatch requirements that fail
  closed when cron/heartbeat logic appears outside the service boundary.
- Risk: heartbeat could become a hidden task execution engine. Mitigation:
  heartbeat may only coalesce and dispatch provider-neutral wake commands through
  declared services; task planning/execution remains owned by task/execution
  services.
- Risk: durable scheduling could expose raw prompts, payloads, secrets, or app
  internals through audit logs. Mitigation: specs require payload references,
  sanitized audit records, bounded event output, and redaction before logs.
- Risk: absent providers could silently disable autonomy. Mitigation: Null
  Object providers must report structured unavailable states, health snapshots,
  and auditable denial reasons.

## Expected Impact

- New proposed specs: `scheduler-service`, `heartbeat-service`.
- Updated proposed specs: `sdk-system-facade`, `service-runtime`,
  `serviceization-escape-hatches`.
- Future implementation will likely touch `macaca-proto`, service crates,
  `macaca-runtime-host`, `macaca-sdk`, and boundary integration tests, but this
  proposal does not implement code yet.
