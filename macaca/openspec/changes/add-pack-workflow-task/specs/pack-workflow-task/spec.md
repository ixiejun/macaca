## ADDED Requirements

### Requirement: Macaca SHALL provide Workflow Task as a serviceized industrial pack

Macaca SHALL provide `pack.workflow.task.v1` as a provider-neutral industrial pack for durable task creation, queueing, dependencies, claiming, leases, attempts, retries, progress, checkpoints, artifacts, terminal outcomes, history, and snapshots. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.workflow.task.v1` as required and the workflow task service is registered, healthy, entitled, policy-admissible, durable, and command-compatible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, state machine, queue capabilities, retry/dependency/concurrency support, policy template, availability, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing raw prompts, secrets, provider payloads, raw artifacts, or unbounded task logs

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.workflow.task.v1` as required but provider, command support, permission, entitlement, resource, durability, queue support, or policy is absent
- **THEN** admission SHALL block readiness with structured unavailable, unsupported, denied, or degraded-durability diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to another provider, or fake durable task success

#### Scenario: Optional declaration is degraded
- **WHEN** an application declares `pack.workflow.task.v1` as optional and the pack is unavailable or command-limited
- **THEN** admission SHALL produce an explicit degraded effective capability report with bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Workflow Task SHALL expose supplier-grade provider-neutral commands

`pack.workflow.task.v1` SHALL expose typed commands for create, update, patch metadata, enqueue, claim, heartbeat, release, record progress, record checkpoint, attach artifact, complete, fail, cancel, skip, get, list, get history, and snapshot operations.

#### Scenario: Task creation is idempotent
- **WHEN** a caller invokes `workflow_task.create` with spec, owner scope, payload reference, queue/dependency metadata, and idempotency key
- **THEN** Macaca SHALL create or return the same task according to idempotency policy
- **AND** raw payloads SHALL remain external references rather than trace contents

#### Scenario: Enqueue validates dependencies and concurrency
- **WHEN** a caller invokes `workflow_task.enqueue`
- **THEN** Macaca SHALL validate dependency readiness, queue policy, priority, concurrency group, and resource budget
- **AND** blocked dependencies or concurrency constraints SHALL return typed blocked diagnostics or queued state according to policy

#### Scenario: Claim creates lease and attempt
- **WHEN** a worker or agent invokes `workflow_task.claim`
- **THEN** Macaca SHALL create a `TaskAttempt` and `TaskLease` with expiry, heartbeat deadline, resource reservation, and claimant identity
- **AND** the task SHALL transition to claimed or running state with replayable evidence

#### Scenario: Heartbeat extends lease
- **WHEN** a claimant invokes `workflow_task.heartbeat`
- **THEN** Macaca SHALL update lease heartbeat and liveness counters if the lease is valid
- **AND** expired or revoked leases SHALL return typed diagnostics without extending work

#### Scenario: Release returns task to queue or blocked state
- **WHEN** a claimant invokes `workflow_task.release`
- **THEN** Macaca SHALL release the lease and move the task according to reason, dependency state, retry policy, and queue policy
- **AND** resources SHALL be released

#### Scenario: Progress and checkpoints are bounded
- **WHEN** a caller records progress or checkpoint
- **THEN** Macaca SHALL store bounded progress metadata or memento references
- **AND** raw logs, prompts, and artifact contents SHALL not enter generic trace/audit records

#### Scenario: Failure applies retry policy
- **WHEN** a caller invokes `workflow_task.fail`
- **THEN** Macaca SHALL classify failure, close the active attempt, apply retry policy, and transition to retry_wait or failed
- **AND** retry exhaustion SHALL be explicit and replayable

#### Scenario: Completion requires valid authority
- **WHEN** a caller invokes `workflow_task.complete`
- **THEN** Macaca SHALL require an active lease or explicit administrative policy
- **AND** invalid terminal transitions SHALL return invalid-state or denied diagnostics

#### Scenario: Cancellation propagates by policy
- **WHEN** a caller invokes `workflow_task.cancel`
- **THEN** Macaca SHALL revoke active leases, record cancellation reason, and apply dependency/dependent propagation policy
- **AND** cancellation SHALL be idempotent and traceable

#### Scenario: History returns replay evidence
- **WHEN** a caller invokes `workflow_task.get_history`
- **THEN** Macaca SHALL return bounded state transition events, evidence ids, versions, and redacted diagnostics
- **AND** replay SHALL not require raw payloads or unbounded logs

### Requirement: Workflow Task DTOs SHALL model state, queues, attempts, leases, retries, dependencies, and artifacts

The pack SHALL define provider-neutral DTOs for workflow tasks, task specs, task states, dependencies, queues, leases, attempts, retry policies, concurrency policies, progress, checkpoints, artifact references, and structured errors.

#### Scenario: State machine is explicit
- **WHEN** a task is inspected
- **THEN** `WorkflowTaskState` SHALL identify draft, ready, queued, blocked, claimed, running, waiting, paused, retry_wait, completed, failed, cancelled, skipped, expired, or unavailable
- **AND** terminal and nonterminal states SHALL have legal transition rules

#### Scenario: Attempt records retry evidence
- **WHEN** a task attempt starts or ends
- **THEN** `TaskAttempt` SHALL record attempt number, start/end time, worker identity, retry source, failure class, timeout class, and history pointer
- **AND** duplicate side-effect prevention SHALL use idempotency/checkpoint evidence

#### Scenario: Lease expiry is explicit
- **WHEN** a claimed task misses heartbeat beyond deadline
- **THEN** `TaskLease` SHALL transition to expired and the task SHALL follow retry/release policy
- **AND** expiry SHALL be visible in history and audit events

#### Scenario: Artifact references are bounded
- **WHEN** an artifact is attached
- **THEN** `TaskArtifactRef` SHALL include artifact id, kind, size class, checksum policy, scan status, retention class, and provenance
- **AND** raw artifact contents SHALL not be stored in generic observability

#### Scenario: Structured errors are stable across providers
- **WHEN** providers return invalid state, dependency blocked, lease expired, retry exhausted, concurrency blocked, quota, artifact blocked, provider failure, or version mismatch states
- **THEN** Macaca SHALL map them to stable `WorkflowTaskError` variants
- **AND** provider-specific diagnostics SHALL be sanitized and bounded

### Requirement: Workflow Task SHALL enforce permission, policy, resource, entitlement, approval, and idempotency

Every command in `pack.workflow.task.v1` SHALL run through permission, policy, resource, entitlement, approval, metering, idempotency, and redaction decorators before provider side effects.

#### Scenario: Missing permission denies before provider dispatch
- **WHEN** an application invokes a command without required scope such as `workflow.task.read`, `workflow.task.write`, `workflow.task.queue`, `workflow.task.claim`, `workflow.task.progress`, `workflow.task.complete`, or `workflow.task.admin`
- **THEN** Macaca SHALL return a typed denied result before invoking the concrete provider
- **AND** the audit event SHALL include the bounded missing-scope code

#### Scenario: Mutating command requires version or idempotency
- **WHEN** a caller invokes a mutating task command
- **THEN** Macaca SHALL require idempotency key, expected version, or active lease evidence according to command semantics
- **AND** missing or mismatched evidence SHALL return conflict or version-mismatch diagnostics

#### Scenario: Terminal transition requires authority
- **WHEN** a caller attempts complete, fail, cancel, or skip
- **THEN** Macaca SHALL require active lease, assignment authority, or administrative policy
- **AND** unauthorized terminal transitions SHALL be denied before provider mutation

#### Scenario: Queue quota blocks overproduction
- **WHEN** active tasks, queued tasks, leases, attempts, retries, checkpoints, artifacts, history size, or retained snapshots exceed resource budget
- **THEN** Macaca SHALL return quota-exceeded diagnostics before provider dispatch
- **AND** resource counters SHALL be emitted in sanitized trace evidence

### Requirement: Workflow Task SHALL preserve canonical service runtime execution

All callable operations SHALL traverse the canonical Macaca service path: application declaration, admission/effective capability projection, SDK/facade command construction, service runtime dispatch, decorators, provider adapter, structured result, task lifecycle events, trace/audit evidence, and replayable snapshot. SDK helpers SHALL NOT construct providers or create alternate execution paths.

#### Scenario: Command succeeds through the canonical path
- **WHEN** a declared and policy-allowed command is invoked
- **THEN** Macaca SHALL route it through SDK/facade helpers into service runtime dispatch and the active task provider adapter
- **AND** trace evidence SHALL show declaration, admission, policy, entitlement, resource, provider selection, state transition, command result, and replay pointer events

#### Scenario: Provider is absent
- **WHEN** no provider is registered for `pack.workflow.task.v1`
- **THEN** the unavailable provider SHALL return structured unavailable diagnostics
- **AND** SDK discovery SHALL report unavailable state while preserving the same provider-neutral command/result contract

#### Scenario: Provider supports only a subset
- **WHEN** the active provider supports task CRUD but not durable queues, retries, or concurrency groups
- **THEN** SDK discovery SHALL mark unsupported commands/features as non-callable
- **AND** direct invocation SHALL return typed unsupported diagnostics without falling through to application-specific logic

#### Scenario: Provider is replaced
- **WHEN** a built-in, remote workflow-engine, plugin, mock, or unavailable provider is selected
- **THEN** callers SHALL observe the same provider-neutral DTO contract
- **AND** OS-layer code SHALL identify only provider class, descriptor version, state support, and capability metadata in traces rather than branching on provider names

### Requirement: Workflow Task SHALL expose industrial SDK discovery and developer documentation

SDK discovery for `pack.workflow.task.v1` SHALL expose pack metadata, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, state machine, queue capabilities, lease rules, retry support, dependency support, concurrency support, policy templates, examples, diagnostics, compatibility, and documentation links. The implementation SHALL provide detailed developer documentation under `docs/developer-packs/workflow/task.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.workflow.task.v1`
- **THEN** it SHALL return command namespace `workflow_task.*`, supported commands, required scopes, state machine, queue/retry/dependency/concurrency support, examples, lifecycle, health, diagnostics, compatibility metadata, and documentation URL
- **AND** examples SHALL use generic synthetic task data rather than application-specific workflows or provider-name routing

#### Scenario: Documentation covers app developer usage
- **WHEN** a developer opens `docs/developer-packs/workflow/task.md`
- **THEN** the guide SHALL explain manifest declarations, scopes, command DTOs, result DTOs, lifecycle states, queues, claims, leases, attempts, retries, dependencies, concurrency, progress, checkpoints, artifacts, cancellation, history, unavailable diagnostics, trace/audit behavior, and replay workflow
- **AND** it SHALL include minimal app-facing examples using canonical SDK calls

#### Scenario: Documentation covers provider authors
- **WHEN** a provider author reads the guide
- **THEN** it SHALL document descriptor fields, task/queue/lease/retry state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy
- **AND** it SHALL forbid application-specific business routing in provider-neutral layers

### Requirement: Workflow Task observability SHALL be sanitized, replayable, and auditable

The pack SHALL emit sanitized trace, audit, health, task lifecycle, queue, lease, attempt, retry, artifact, snapshot, and replay evidence for declaration, admission, creation, queueing, claiming, heartbeat, progress, checkpoint, artifact, completion, failure, retry scheduling, cancellation, skip, lease revocation, unavailable state, and snapshot recording.

#### Scenario: Successful command emits bounded evidence
- **WHEN** a task command succeeds
- **THEN** Macaca SHALL emit sanitized events containing pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when available, task id, version, state, queue id hash, attempt number, lease id hash, policy decision, failure class, retry class, latency, and resource counters
- **AND** it SHALL exclude raw prompts, raw payloads, raw artifacts, credentials, secrets, provider payloads, and unbounded logs

#### Scenario: Snapshot records queue and lease summaries
- **WHEN** the service runtime records a task snapshot
- **THEN** the snapshot SHALL include queue summaries, state counts, active lease summaries, retry wait summaries, dependency blocked summaries, policy hash, unavailable diagnostics, and sanitized replay pointers
- **AND** it SHALL exclude raw payloads, artifacts, prompts, credentials, and unbounded output

#### Scenario: Replay verifies task lifecycle
- **WHEN** a session or task is replayed after refresh or restart
- **THEN** Macaca SHALL reconstruct the task command, state transition, lease, attempt, retry, and terminal outcome chain from bounded trace/audit evidence
- **AND** replay diagnostics SHALL prove the commands used the canonical service runtime path

### Requirement: Workflow Task implementation SHALL preserve Macaca architecture boundaries

The `pack.workflow.task.v1` implementation SHALL keep concrete task providers behind service/runtime provider adapters. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, provider-specific, worker-specific, queue-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan imports
- **WHEN** dependency-boundary gates scan the implementation
- **THEN** they SHALL find no concrete workflow engine, task provider, queue backend, worker implementation, or shell task-board semantic owner in the microkernel, SDK, shells, or generic application framework
- **AND** provider construction SHALL appear only in approved runtime composition roots or plugin/remote provider registration paths

#### Scenario: No-direct-provider-call gate scans commands
- **WHEN** no-direct-provider-call gates scan task commands
- **THEN** every callable operation SHALL be reachable only through descriptor-owned service registrations and typed service runtime dispatch
- **AND** SDK helpers SHALL only build canonical service commands

#### Scenario: Pack remains separate from neighboring workflow packs
- **WHEN** architecture review compares workflow packs
- **THEN** task SHALL own durable task state, queues, dependencies, claims, leases, attempts, retries, progress, checkpoints, artifacts, terminal outcomes, history, and snapshots
- **AND** schedule, approval, delegation, review, recovery, and application-specific task boards SHALL remain owned by their respective packs or services
