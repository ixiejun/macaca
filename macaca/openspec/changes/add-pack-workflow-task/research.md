# Workflow Task Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
boundary decisions, and GitNexus memo evidence for `pack.workflow.task.v1`.
The task pack owns durable workflow task records, state transitions, queues,
leases, attempts, retry metadata, dependencies, concurrency gates, progress,
checkpoints, artifacts, history, snapshots, freshness, and redaction. It must
not own schedule recurrence, approval decisions, delegation assignment policy,
review outcomes, recovery classification, application task-board UI, shell
rendering, or existing Macaca autonomy planning semantics.

## Source Baseline

- Temporal tasks, task queues, workers, retries, and activity semantics:
  <https://docs.temporal.io/tasks>,
  <https://docs.temporal.io/task-queue>, and
  <https://docs.temporal.io/encyclopedia/retry-policies>
- Camunda service tasks and user tasks:
  <https://docs.camunda.io/docs/components/modeler/bpmn/service-tasks/> and
  <https://docs.camunda.io/docs/components/modeler/bpmn/user-tasks/>
- Apache Airflow tasks, DAGs, retries, and task instances:
  <https://airflow.apache.org/docs/apache-airflow/stable/core-concepts/tasks.html>
  and
  <https://airflow.apache.org/docs/apache-airflow/stable/core-concepts/dags.html>
- GitHub Actions jobs and concurrency:
  <https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/using-jobs-in-a-workflow>
  and
  <https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/control-the-concurrency-of-workflows-and-jobs>

## Supplier API Notes

- Temporal contributes task queues, workers, activities, workflow tasks, leases
  via poll/heartbeat behavior, retry policies, timeouts, history, and durable
  replay. Macaca should normalize these into queue, lease, attempt, checkpoint,
  retry, and history DTOs without exposing Temporal event payloads.
- Camunda contributes service-task and user-task separation, BPMN state,
  assignment, completion, incidents, and history. Macaca should keep human
  approval/review semantics in adjacent workflow packs while retaining generic
  task states and audit evidence.
- Airflow contributes DAG/task dependency, retries, task instances, queued/running
  states, XCom-like artifacts, and scheduler integration. Macaca should model
  dependencies and artifacts without absorbing scheduler recurrence.
- GitHub Actions contributes jobs, matrices, concurrency groups, cancellation,
  and queued/in-progress/completed outcomes. Macaca should normalize concurrency
  gates and terminal states without hardcoding repository or CI semantics.

## Macaca-Owned Abstractions

`pack.workflow.task.v1` should define `WorkflowTask`, `WorkflowTaskSpec`,
`WorkflowTaskState`, `TaskDependency`, `TaskQueueRef`, `TaskLease`,
`TaskAttempt`, `RetryPolicy`, `ConcurrencyPolicy`, `TaskProgress`,
`TaskCheckpoint`, `TaskArtifactRef`, and `WorkflowTaskError`.

The DTOs must carry task identity, queue reference, dependency graph, lease
owner reference, attempt number, retry budget, concurrency group, progress
summary, checkpoint handle, artifact handle, state version, redaction class,
bounded diagnostics, and replay pointers. Raw prompts, unbounded input payloads,
raw provider histories, raw artifacts, worker secrets, and application-specific
task-board fields are rejected.

## Boundary Decisions And Non-Goals

- Schedule owns recurrence and trigger creation.
- Approval owns decision gates.
- Delegation owns assignment/lease transfer between eligible actors.
- Review owns finding/fix/re-review/approval closure.
- Recovery owns failure classification, retry/repair/resume plans, and replay
  export.
- Applications own product task boards, labels, swimlanes, and business
  workflow semantics.
- Shells own rendering only.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  workflow task SDK helpers should only build canonical traced service calls.
- Existing Macaca task/autonomy services provide useful state-machine and trace
  concepts, but this pack must expose developer-facing, provider-neutral domain
  pack contracts through service runtime boundaries.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
