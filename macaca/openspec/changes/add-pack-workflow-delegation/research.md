# Workflow Delegation Pack Research

## Purpose

This note records borrowed platform patterns, Macaca provider-neutral mapping,
reuse inventory, and GitNexus memo evidence for `pack.workflow.delegation.v1`.
The delegation pack owns delegation requests, accept/claim semantics, leases,
handoff, capacity inspection, result collection, cancellation, freshness, and
audit records. It must not own task internals, approval decisions, review
findings, recovery repair, identity membership, or application-specific
assignment logic.

## Source Baseline

- Temporal task queues, workers, heartbeats, and cancellation:
  <https://docs.temporal.io/task-queue> and
  <https://docs.temporal.io/activities>
- Camunda user task assignment and candidate groups:
  <https://docs.camunda.io/docs/components/modeler/bpmn/user-tasks/>
- GitHub Actions runners and job assignment/concurrency:
  <https://docs.github.com/en/actions/hosting-your-own-runners/managing-self-hosted-runners/about-self-hosted-runners>
  and
  <https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/control-the-concurrency-of-workflows-and-jobs>
- Kubernetes work queue and lease concepts:
  <https://kubernetes.io/docs/concepts/architecture/leases/>
- ServiceNow assignment/delegation concepts:
  <https://docs.servicenow.com/>

## Borrowed Platform Pattern Mapping

- Worker/task-queue patterns map to delegatable work queues, capacity
  snapshots, and eligible assignee classes.
- User-task assignment maps to candidate assignee sets, acceptance, conflict on
  duplicate accept, and audit-preserving handoff.
- Runner/concurrency patterns map to capacity limits and queue re-placement
  after capacity recovery.
- Lease patterns map to `DelegationLease`, renewal, expiry, deterministic clock
  evidence, and one-active-owner constraints.
- Service-management delegation maps to handoff, escalation, cancellation, and
  result collection without embedding business-specific routing rules.

## Macaca-Owned Abstractions

`pack.workflow.delegation.v1` should define `DelegationRequest`,
`DelegationClaim`, `DelegationLease`, `DelegationHandoff`,
`CapacitySnapshot`, `DelegationResult`, `DelegationState`, and
`DelegationAuditRecord`.

The DTOs must carry requested work reference, eligible assignee scope,
capacity snapshot, claim idempotency key, lease expiry, handoff checkpoint,
result artifact handle, terminal state, bounded diagnostics, redaction class,
and replay pointers. Raw prompts, raw task payloads, hidden capacity
calculations, provider payloads, credentials, and application-specific routing
logic are rejected.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides canonical Facade
  calls for SDK helpers.
- Generic policy, resource, entitlement, trace, audit, artifact, mock-provider,
  unavailable-provider, and task-state concepts are reusable, but current
  evidence does not prove delegation-specific DTOs, providers, SDK helpers,
  tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
