## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, the umbrella industrial catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API comparison notes for Temporal tasks/task queues/retry policies, Camunda service/user tasks, Airflow tasks/DAGs/retries, and GitHub Actions jobs/concurrency.
- [x] 1.3 Confirm boundaries with workflow schedule, approval, delegation, review, recovery, application task boards, shell rendering, and existing Macaca task/autonomy services.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits, per the current refactor instruction.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define provider-neutral commands for create, update, patch metadata, enqueue, claim, heartbeat, release, record progress, record checkpoint, attach artifact, complete, fail, cancel, skip, get, list, get history, and snapshot.
- [x] 2.2 Define `WorkflowTask`, `WorkflowTaskSpec`, `WorkflowTaskState`, `TaskDependency`, `TaskQueueRef`, `TaskLease`, `TaskAttempt`, `RetryPolicy`, `ConcurrencyPolicy`, `TaskProgress`, `TaskCheckpoint`, `TaskArtifactRef`, and `WorkflowTaskError`.
- [x] 2.3 Define typed success, partial, denied, unavailable, unsupported, invalid-state, conflict, dependency-blocked, lease-expired, lease-revoked, retry-exhausted, concurrency-blocked, quota-exceeded, artifact-blocked, provider-failure, and version-mismatch results.
- [x] 2.4 Define descriptor metadata for pack id, family, lifecycle, command schemas, state machine, queue capabilities, lease rules, retry support, dependency support, concurrency support, permission scopes, policy template, resource budgets, SDK metadata, compatibility, diagnostics, and documentation URL.
- [x] 2.5 Add stable descriptor hashing, version compatibility checks, DTO snapshot fixtures, state transition fixtures, retry fixtures, lease fixtures, dependency/concurrency fixtures, and schema migration tests.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for `workflow.task.read`, `workflow.task.write`, `workflow.task.queue`, `workflow.task.claim`, `workflow.task.progress`, `workflow.task.complete`, and `workflow.task.admin`.
- [x] 3.2 Enforce state transition, idempotency, optimistic concurrency, dependency, queue, concurrency group, retry, timeout, artifact, terminal transition, and redaction policies before dispatch.
- [x] 3.3 Require mutating commands to carry idempotency keys or expected versions.
- [x] 3.4 Add resource reservation and quota checks for active tasks, queued tasks, active leases, attempts, retries, checkpoints, artifacts, history size, retained snapshots, and replay metadata.
- [x] 3.5 Add approval behavior for force transitions, cancellation propagation, administrative repair, high-priority queues, and external side-effect artifact attachments.
- [x] 3.6 Add tests proving denied, unavailable, invalid-state, dependency-blocked, lease-expired, lease-revoked, retry-exhausted, concurrency-blocked, artifact-blocked, and quota paths do not call concrete providers incorrectly or leak resources.

## 4. Service Provider And Task State Strategy

- [x] 4.1 Implement the workflow task service provider contract behind the service runtime; do not construct providers from kernel, SDK, shells, or generic application-framework code.
- [x] 4.2 Add provider descriptor support for built-in durable, remote workflow-engine, plugin, mock, and unavailable provider classes.
- [x] 4.3 Add task, queue, lease, attempt, retry, dependency, concurrency, cancellation, and terminal outcome state machines.
- [x] 4.4 Add mock and unavailable providers for deterministic tests; external workflow adapters must remain optional providers or plugin/remote modules.
- [x] 4.5 Add provider conformance tests for creation, enqueue, claim, heartbeat, release, progress, checkpoint, artifact, complete, fail, retry, cancel, skip, dependencies, concurrency, history, snapshot, redaction, and unsupported-command reporting.
- [x] 4.6 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, lease expiry, retry scheduling, resource cleanup, and bounded output behavior.

## 5. SDK, Admission, Examples, And ABI

- [x] 5.1 Extend SDK discovery for `pack.workflow.task.v1` with command schemas, DTO schemas, permission scopes, examples, availability, state machine, queue capabilities, retry/dependency/concurrency support, diagnostics, compatibility, and documentation URL.
- [x] 5.2 Extend application admission so required declarations block when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls and never construct providers or branch on task/worker/queue/application names.
- [x] 5.4 Add WASM/application ABI exposure for task commands using provider-neutral DTO schemas and canonical service-call dispatch.
- [x] 5.5 Add generic examples for create, enqueue, claim, heartbeat, progress, checkpoint, complete, fail/retry, cancel, dependencies, concurrency, history, and unavailable-provider diagnostics.

## 6. Trace, Audit, Replay, And Boundary Gates

- [x] 6.1 Emit sanitized task lifecycle events for declaration, admission, creation, enqueue, claim, heartbeat, progress, checkpoint, artifact, completion, failure, retry scheduling, cancellation, skip, lease revocation, and snapshot recording.
- [x] 6.2 Add replay tests proving every state transition and terminal outcome is trace-addressable through the canonical service path after refresh/restart.
- [x] 6.3 Add dependency-boundary gates proving microkernel, SDK, shells, and generic application framework do not import concrete task providers or workflow engines.
- [x] 6.4 Add no-direct-provider-call gates proving all task commands enter through descriptor-owned service registrations and typed service runtime dispatch.
- [x] 6.5 Add redaction tests for raw prompts, input payloads, artifacts, provider payloads, worker diagnostics, history events, snapshots, and logs.
- [x] 6.6 Run `openspec validate add-pack-workflow-task --strict`, DTO compatibility tests, state machine tests, retry/lease tests, dependency/concurrency tests, boundary gates, file-size gates, and audit replay checks before marking implementation tasks complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/workflow/task.md` with purpose, manifest declarations, scopes, command DTOs, result DTOs, lifecycle states, queues, claims, leases, attempts, retries, dependencies, concurrency, progress, checkpoints, artifacts, cancellation, history, unavailable diagnostics, and trace/audit behavior.
- [x] 7.2 Add provider author documentation covering descriptor fields, task/queue/lease/retry state machines, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy.
- [x] 7.3 Add minimal app-facing examples for task create/enqueue/claim/heartbeat/complete/fail/retry/cancel/history using generic synthetic data.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-workflow-task` complete.
