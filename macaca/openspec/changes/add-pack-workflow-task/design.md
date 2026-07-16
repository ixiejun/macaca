# Workflow Task Pack Design

## Context

`pack.workflow.task.v1` is the durable task substrate for Macaca's autonomous workflow packs. It provides generic task state, queues, dependencies, assignment, attempts, retries, progress, checkpoints, artifacts, and history. It must be provider-neutral and reusable for all applications; application-specific task board semantics remain outside the OS layer.

Temporal, Camunda, Airflow, and GitHub Actions all show the same core pattern: durable work items, explicit states, queue/worker leasing, retries, cancellation, dependency/concurrency constraints, and event history. Macaca adapts those ideas through typed service commands and replayable evidence.

## Supplier Capability Matrix

| Platform/API | Borrowed capability | Macaca mapping |
| --- | --- | --- |
| Temporal | task queues, worker polling, activity attempts, retry policy, event history | task queues, claims/leases, attempts, retry policy, history |
| Camunda | service/user tasks, assignments, due dates, incidents, worker completion | task specs, assignment hints, due times, failure/incidents |
| Airflow | tasks in DAGs, dependencies, retries, paused/scheduled/running states | dependency graph, retry/backoff, state transitions |
| GitHub Actions | jobs, concurrency groups, cancellation, queued runs | concurrency policy, cancel/replace/queue behavior |

## Goals

- Provide durable task creation, update, queueing, claiming, heartbeat, release, progress, checkpoint, artifact, completion, failure, cancellation, skip, listing, history, and snapshot commands.
- Model task lifecycle with explicit nonterminal and terminal states.
- Support dependencies, concurrency groups, retry policies, timeouts, idempotency, lease expiry, and worker assignment hints.
- Emit replayable sanitized trace/audit evidence for every state transition.
- Provide detailed developer documentation and provider conformance guidance.

## Non-Goals

- Do not own workflow schedule, approval decisions, delegation assignment policy, review judgment, recovery planning, or application-specific task boards.
- Do not encode application task names, worker names, queue names, provider names, or business workflows in OS-layer code.
- Do not store raw prompts, secrets, unbounded logs, raw artifacts, or provider payloads in generic observability.

## Ownership And Boundaries

- Pack id: `pack.workflow.task.v1`.
- Capability family: `workflow`.
- Backing service: workflow task service.
- SDK surface: `sdk.packs.workflow.task`.
- Command namespace: `workflow_task.*`.
- Application framework owns manifest declaration and app-scoped permission projection.
- Service runtime owns typed dispatch, decorators, task lifecycle, queue/lease state, retries, health, snapshots, and unavailable behavior.
- Runtime host owns concrete provider adapters through approved composition roots.
- Shells render task state and diagnostics only from service events.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `workflow_task.create` | Create a task spec | Requires idempotency key, owner scope, payload reference, policy, queue/dependency metadata |
| `workflow_task.update` | Update mutable task fields | Validates state, version, permissions, and optimistic concurrency |
| `workflow_task.patch_metadata` | Patch labels, hints, SLA, priority, or external refs | Bounded metadata only |
| `workflow_task.enqueue` | Place task into queue | Validates dependencies, readiness, queue policy, and concurrency policy |
| `workflow_task.claim` | Worker/agent claims task lease | Creates attempt and lease with expiry and heartbeat requirements |
| `workflow_task.heartbeat` | Extend active lease and report liveness | Updates lease heartbeat and emits bounded counters |
| `workflow_task.release` | Release task without terminal outcome | Returns to queued/blocked state according to reason |
| `workflow_task.record_progress` | Record bounded progress | Updates percentage/stage/message references without raw logs |
| `workflow_task.record_checkpoint` | Record resumable checkpoint reference | Captures memento for recovery and replay |
| `workflow_task.attach_artifact` | Attach artifact reference | Stores bounded references and scan/redaction metadata |
| `workflow_task.complete` | Mark task completed | Requires active lease or authorized terminal transition |
| `workflow_task.fail` | Mark attempt or task failed | Applies retry policy and failure classification |
| `workflow_task.cancel` | Cancel task | Propagates cancellation to leases and dependents by policy |
| `workflow_task.skip` | Skip task with reason | Requires policy allowance and dependency handling |
| `workflow_task.get` | Inspect one task | Returns current state, attempt, queue, dependency, progress, and history pointers |
| `workflow_task.list` | Query tasks | Supports scoped filtering, pagination, and redacted summaries |
| `workflow_task.get_history` | Read replayable state history | Returns bounded events and evidence ids |
| `workflow_task.snapshot` | Record task service snapshot | Captures queue/state/lease summaries |

## DTO Model

- `WorkflowTask`: id, version, spec, state, queue, dependencies, current attempt, progress, checkpoints, artifacts, labels, timestamps, and history pointer.
- `WorkflowTaskSpec`: objective reference, input payload reference, result contract, priority, SLA, queue hint, assignment hint, retry policy, timeout policy, concurrency policy, and redaction policy.
- `WorkflowTaskState`: draft, ready, queued, blocked, claimed, running, waiting, paused, retry_wait, completed, failed, cancelled, skipped, expired, or unavailable.
- `TaskDependency`: dependency task id, condition, result requirement, failure policy, and unblock evidence.
- `TaskQueueRef`: queue id, partition, fairness class, priority class, visibility scope, and policy hash.
- `TaskLease`: lease id, claimant identity, attempt id, expiry, heartbeat deadline, revocation state, and resource reservation.
- `TaskAttempt`: attempt number, start/end time, worker identity, retry source, failure class, timeout class, and history pointer.
- `RetryPolicy`: max attempts, backoff, jitter, retryable failure classes, nonretryable classes, and retry window.
- `ConcurrencyPolicy`: group key reference, max in progress, queue/cancel/replace behavior, and ordering policy.
- `TaskProgress`: phase, percent, bounded message reference, counters, ETA class, and last update.
- `TaskCheckpoint`: checkpoint id, memento reference, schema version, redaction state, and replay pointer.
- `TaskArtifactRef`: artifact id, kind, size class, checksum policy, scan status, retention class, and provenance.
- `WorkflowTaskError`: denied, unavailable, unsupported, invalid state, conflict, dependency blocked, lease expired, lease revoked, retry exhausted, concurrency blocked, quota exceeded, artifact blocked, provider failure, or version mismatch.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `workflow.task.read`: inspect/list/history.
- `workflow.task.write`: create/update/metadata.
- `workflow.task.queue`: enqueue and dependency handling.
- `workflow.task.claim`: claim/heartbeat/release active work.
- `workflow.task.progress`: progress, checkpoints, artifacts.
- `workflow.task.complete`: terminal success/skip/fail/cancel.
- `workflow.task.admin`: snapshots, force transitions, policy repair.

Policy requirements:

- Every mutating command requires idempotency or optimistic concurrency evidence.
- Terminal transitions require active lease or explicit administrative policy.
- Dependencies and concurrency groups must be evaluated before enqueue/claim.
- Retry policy applies to attempts; retry exhaustion is explicit.
- Progress, checkpoints, and artifacts store bounded references, not raw payloads.

## Service Runtime And Provider Strategy

Provider Strategy categories:

- Built-in durable task provider.
- Remote workflow engine adapter.
- Plugin provider for specialized queue/worker implementations.
- Mock provider for deterministic tests/docs.
- Unavailable provider for absent capability.

Providers declare state machine support, queue capabilities, lease heartbeat rules, retry support, dependency/concurrency support, snapshot behavior, and health. Provider construction is allowed only in approved composition roots.

## SDK Discovery And Developer Documentation

SDK discovery returns pack metadata, command schemas, DTO schemas, permission scopes, state machine, queue capability matrix, retry/concurrency/dependency support, examples, diagnostics, compatibility, and documentation links.

The implementation SHALL create `docs/developer-packs/workflow/task.md` with manifest declarations, scopes, command reference, lifecycle diagrams, queue/lease/attempt semantics, retries, dependencies, concurrency, checkpoints, artifacts, cancellation, history/replay, unavailable diagnostics, and provider conformance checklist.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `workflow_task.pack_declared`
- `workflow_task.admission_validated`
- `workflow_task.created`
- `workflow_task.enqueued`
- `workflow_task.claimed`
- `workflow_task.heartbeat_recorded`
- `workflow_task.progress_recorded`
- `workflow_task.checkpoint_recorded`
- `workflow_task.artifact_attached`
- `workflow_task.completed`
- `workflow_task.failed`
- `workflow_task.retry_scheduled`
- `workflow_task.cancelled`
- `workflow_task.skipped`
- `workflow_task.lease_revoked`
- `workflow_task.snapshot_recorded`

Events include pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when present, task id, version, state, queue id hash, attempt number, lease id hash, policy decision, failure class, retry class, latency, and resource counters. Events exclude raw prompts, raw payloads, raw artifacts, credentials, secrets, provider payloads, and unbounded logs.

Snapshots include queue summaries, state counts, active lease summaries, retry wait summaries, dependency blocked summaries, policy hash, unavailable diagnostics, and replay pointers.

## Design Patterns

- **Facade**: SDK exposes task clients while `SystemFacade` carries canonical service calls.
- **Command**: every operation is a typed command/result DTO.
- **State**: task, attempt, lease, dependency, and terminal outcome lifecycles are explicit state machines.
- **Strategy**: provider selection, retry policy, concurrency policy, queue policy, and unavailable behavior are descriptor-driven.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **Specification**: admission validates states, scopes, dependencies, idempotency, queue policy, and concurrency policy.
- **Observer**: task lifecycle, trace, audit, health, and service events are subscribable.
- **Memento**: checkpoints, snapshots, and history pointers enable replay/recovery.
- **Abstract Factory**: providers are created only in approved composition roots.

## Risks And Mitigations

- Risk: task pack becomes application-specific board logic. Mitigation: generic state/queue/lease/attempt contract only.
- Risk: duplicate side effects after retry. Mitigation: idempotency keys, attempt records, checkpoints, retry policy, and replay history.
- Risk: stuck claimed tasks. Mitigation: lease heartbeat, expiry, revocation, and retry/release behavior.
- Risk: shell owns task transitions. Mitigation: all transitions are service commands and boundary gates enforce no shell semantics.
- Risk: observability leaks prompts/artifacts. Mitigation: references, hashes, counters, and bounded diagnostics only.
