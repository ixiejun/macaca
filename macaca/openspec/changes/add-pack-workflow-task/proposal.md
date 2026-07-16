# Change: Add Industrial Workflow Task Pack

## Why

Macaca applications need `pack.workflow.task.v1` for durable, auditable task creation and execution coordination: task descriptors, dependencies, queues, assignment, leases, attempts, retries, cancellation, progress, artifacts, checkpoints, terminal outcomes, and replay evidence. This pack is the foundation under workflow schedule, approval, delegation, review, and recovery.

The current template is not industrial-grade because it does not define a durable state machine, retry/attempt semantics, dependency handling, concurrency, idempotency, lease expiry, or replayable event history. Macaca must support 24/7 autonomous work without application-specific task boards or shell-owned orchestration.

## Supplier/API Baseline

- Temporal Tasks and Task Queues: durable task queues, workers polling, activity/workflow tasks, retries, timeouts, and event history. Official docs: https://docs.temporal.io/tasks, https://docs.temporal.io/task-queue, and https://docs.temporal.io/encyclopedia/retry-policies
- Camunda service/user tasks: BPMN task types, job creation, worker completion, assignments, due dates, incidents, and task updates. Official docs: https://docs.camunda.io/docs/components/modeler/bpmn/service-tasks/ and https://docs.camunda.io/docs/components/modeler/bpmn/user-tasks/
- Apache Airflow tasks/DAGs: task as execution unit, dependencies, retries, task states, scheduling, pause/resume, and sensors. Official docs: https://airflow.apache.org/docs/apache-airflow/stable/core-concepts/tasks.html and https://airflow.apache.org/docs/apache-airflow/stable/core-concepts/dags.html
- GitHub Actions jobs/concurrency: workflow jobs, cancellation, concurrency groups, queued runs, and deterministic job control. Official docs: https://docs.github.com/actions/using-workflows/workflow-syntax-for-github-actions and https://docs.github.com/actions/writing-workflows/choosing-what-your-workflow-does/control-the-concurrency-of-workflows-and-jobs

## Macaca Provider-Neutral Mapping

Macaca SHALL expose workflow tasks as serviceized autonomy primitives:

- Task creation/update becomes `workflow_task.create`, `workflow_task.update`, and `workflow_task.patch_metadata`.
- Queueing/assignment/leasing becomes `workflow_task.enqueue`, `workflow_task.claim`, `workflow_task.heartbeat`, and `workflow_task.release`.
- Execution progress becomes `workflow_task.record_progress`, `workflow_task.record_checkpoint`, and `workflow_task.attach_artifact`.
- Outcomes become `workflow_task.complete`, `workflow_task.fail`, `workflow_task.cancel`, and `workflow_task.skip`.
- Inspection and replay become `workflow_task.get`, `workflow_task.list`, `workflow_task.get_history`, and `workflow_task.snapshot`.

## What Changes

- Add `pack.workflow.task.v1` as a service-backed industrial pack under the workflow family.
- Define command DTOs for task CRUD, dependencies, queues, claims, leases, attempts, retries, cancellation, progress, checkpoints, artifacts, history, and snapshots.
- Define DTOs for `WorkflowTask`, `WorkflowTaskSpec`, `WorkflowTaskState`, `TaskDependency`, `TaskQueueRef`, `TaskLease`, `TaskAttempt`, `RetryPolicy`, `ConcurrencyPolicy`, `TaskProgress`, `TaskCheckpoint`, `TaskArtifactRef`, and structured errors.
- Define permission scopes, policy/resource/entitlement rules, idempotency keys, queue fairness, concurrency groups, trace/audit events, and unavailable diagnostics.
- Require detailed developer documentation under `docs/developer-packs/workflow/task.md`.

## Impact

- Affected specs: `pack-workflow-task`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Later affected code: protocol DTOs, descriptor/admission validators, SDK pack client, task service provider contract, queue/lease/retry adapters, mock/unavailable providers, trace/audit schemas, and boundary gates.
- Validation: `openspec validate add-pack-workflow-task --strict`, state machine tests, retry/lease tests, dependency/concurrency tests, replay tests, no-direct-provider-call gates, and docs coverage checks.

## Non-Goals

- This pack does not own scheduling calendars, human approvals, delegation policy, review judgment, recovery planning, application-specific task boards, or workflow-specific business logic.
- This pack does not hardcode task types, worker names, queue names, app names, providers, or business workflows into OS-layer routing.
- This pack does not expose raw prompts, secrets, artifacts, provider payloads, unbounded logs, or application-specific payloads in traces, audits, snapshots, logs, or examples.
