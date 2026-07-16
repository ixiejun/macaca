# Workflow Recovery Pack Research

## Purpose

This note records borrowed platform patterns, Macaca provider-neutral mapping,
reuse inventory, and GitNexus memo evidence for `pack.workflow.recovery.v1`.
The recovery pack owns failure classification, recovery point listing, retry,
state repair, resume, replay export, checkpoint integrity, retry budget,
compensation references, freshness, and audit records. It must not own normal
task execution, scheduling, approval decisions, delegation, review closure, or
application-specific incident workflows.

## Source Baseline

- Temporal retries, failures, workflow replay, and continue-as-new:
  <https://docs.temporal.io/encyclopedia/retry-policies>,
  <https://docs.temporal.io/develop/go/failure-detection>, and
  <https://docs.temporal.io/workflows#event-history>
- AWS prescriptive guidance for saga patterns and compensation:
  <https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/saga.html>
- Camunda incidents and retries:
  <https://docs.camunda.io/docs/components/concepts/incidents/>
- Airflow task retries and clearing/backfill:
  <https://airflow.apache.org/docs/apache-airflow/stable/core-concepts/tasks.html>
- Kubernetes Jobs retry/backoff and failure handling:
  <https://kubernetes.io/docs/concepts/workloads/controllers/job/>

## Borrowed Platform Pattern Mapping

- Temporal failure/replay patterns map to recovery points, event lineage,
  replay export, retry policies, and deterministic resume requirements.
- Saga compensation maps to `CompensationRef` and repair plans without
  embedding domain-specific compensation behavior.
- Camunda incidents map to failure classification and operator-visible repair
  state.
- Airflow clearing/backfill patterns map to retry/resume after task state
  changes, with policy-controlled replay.
- Kubernetes Job backoff maps to retry budgets, terminalization, and bounded
  retry loops.

## Macaca-Owned Abstractions

`pack.workflow.recovery.v1` should define `FailureRecord`,
`RecoveryPoint`, `RetryPolicy`, `RecoveryPlan`, `RepairAction`,
`CompensationRef`, `ResumePlan`, `ReplayExport`, `RecoveryState`, and
`RecoveryAuditRecord`.

The DTOs must carry failed subject reference, failure class, checkpoint handle,
checkpoint integrity hash, compatibility version, retry budget, backoff policy,
repair action class, compensation reference, resume target, replay export
handle, terminal state, redaction class, and replay pointers. Raw checkpoint
bytes, raw prompts, provider payloads, credentials, manifests, package bytes,
and unbounded replay exports are rejected.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides canonical Facade
  calls for SDK helpers.
- Generic policy, resource, entitlement, trace, audit, artifact, mock-provider,
  unavailable-provider, task, schedule, and recovery/autonomy concepts are
  reusable, but current evidence does not prove recovery-specific DTOs,
  providers, SDK helpers, tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
