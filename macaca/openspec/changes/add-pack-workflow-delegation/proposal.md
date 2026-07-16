# Change: Add Workflow Delegation Pack

## Why

Developers need `pack.workflow.delegation.v1` as a real industrial capability for agent delegation, role assignment, handoff, capacity, and result collection. The pack must not be a catalog label only: if an application declares it and the provider is installed, the SDK must expose callable typed commands; if policy, entitlement, provider, or host support is absent, Macaca must return explicit unavailable or denied diagnostics.

This child proposal intentionally narrows the broad industrial catalog into one implementable service-backed pack. It records the research basis, capability boundary, command surface, permission model, observability model, and acceptance gates needed before this pack can be called production-ready.

## Research And Borrowed Platform Patterns

The proposal borrows stable ideas from mature application platforms without copying their product-specific APIs:

- OS background task and notification patterns: scheduling and lifecycle are platform services, not app loops.
- Android foreground/background execution limits: long-running work must expose state and user-sensitive approval.
- Windows app lifecycle and notification activation: work resumption needs identity and manifest-visible behavior.
- Apple background task/privacy patterns: autonomous work needs explicit capability and observable lifecycle.

The Macaca interpretation is: app manifests declare pack intent; admission validates capability and permission metadata; runtime permission and approval gates happen before sensitive side effects; concrete providers stay behind service descriptors; traces and audits prove the execution path.

## What Changes

- Add the provider-neutral `pack.workflow.delegation.v1` contract under the `workflow` family.
- Define a concrete command namespace, initial industrial command set, result/error DTO requirements, permission scopes, policy defaults, resource/entitlement behavior, and unavailable diagnostics.
- Bind implementation ownership to delegation service provider; the kernel, SDK, shells, and generic application framework remain provider-neutral.
- Add SDK discovery metadata and examples for delegate, accept delegation, handoff, inspect capacity, collect result.
- Add trace, audit, health, snapshot, replay, and boundary gates proving the pack uses the canonical service path.

## Impact

- Affected specs: `pack-workflow-delegation`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs in protocol crates, descriptor/admission validators, SDK discovery/command helpers, delegation service provider, optional provider registration, trace/audit tests, and dependency-boundary gates.
- Non-goals: no application-specific workflow, no provider-name routing in OS layers, no concrete provider construction in SDK/shell/kernel, and no fake success for absent providers.

## Supplier/API Baseline To Match

The delegation pack must be comparable to mature orchestration and work
assignment systems:

- Temporal task queues and worker versioning:
  `https://docs.temporal.io/` establish durable task routing, worker capacity,
  leases, heartbeats, cancellation, retry visibility, and replay-safe handoff.
- Camunda task assignment:
  `https://docs.camunda.io/` establishes assignee, candidate users/groups,
  claim/unclaim, delegation-like reassignment, due dates, and task history.
- Kubernetes scheduler and Lease coordination:
  `https://kubernetes.io/docs/` establishes placement constraints, leases,
  heartbeat expiry, fairness, and controller-driven reconciliation.
- GitHub Actions job concurrency/runners:
  `https://docs.github.com/actions/` establishes runner labels, concurrency
  groups, cancellation, queue visibility, and result artifacts.

Macaca should expose the common abstraction: a delegation is a durable work
placement request with assignee constraints, capacity evidence, lease/heartbeat
state, handoff semantics, result collection, cancellation, and replayable
assignment history.

## Industrial Acceptance Bar

This proposal is not complete until implementation tasks produce:

- a developer guide at `docs/developer-packs/workflow/delegation.md`;
- typed placement, claim, lease, handoff, capacity, result, and cancellation DTOs;
- deterministic tests for lease expiry, reassignment, duplicate accepts,
  assignee capacity exhaustion, cancellation races, and result collection after
  restart;
- audit replay proving every delegated unit has one active owner at a time,
  bounded handoff history, and structured terminal state.
