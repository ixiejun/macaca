# Change: Add Workflow Recovery Pack

## Why

Developers need `pack.workflow.recovery.v1` as a real industrial capability for checkpoint discovery, failure classification, retry, repair, resume, and replay diagnostics. The pack must not be a catalog label only: if an application declares it and the provider is installed, the SDK must expose callable typed commands; if policy, entitlement, provider, or host support is absent, Macaca must return explicit unavailable or denied diagnostics.

This child proposal intentionally narrows the broad industrial catalog into one implementable service-backed pack. It records the research basis, capability boundary, command surface, permission model, observability model, and acceptance gates needed before this pack can be called production-ready.

## Research And Borrowed Platform Patterns

The proposal borrows stable ideas from mature application platforms without copying their product-specific APIs:

- OS background task and notification patterns: scheduling and lifecycle are platform services, not app loops.
- Android foreground/background execution limits: long-running work must expose state and user-sensitive approval.
- Windows app lifecycle and notification activation: work resumption needs identity and manifest-visible behavior.
- Apple background task/privacy patterns: autonomous work needs explicit capability and observable lifecycle.

The Macaca interpretation is: app manifests declare pack intent; admission validates capability and permission metadata; runtime permission and approval gates happen before sensitive side effects; concrete providers stay behind service descriptors; traces and audits prove the execution path.

## What Changes

- Add the provider-neutral `pack.workflow.recovery.v1` contract under the `workflow` family.
- Define a concrete command namespace, initial industrial command set, result/error DTO requirements, permission scopes, policy defaults, resource/entitlement behavior, and unavailable diagnostics.
- Bind implementation ownership to recovery service provider; the kernel, SDK, shells, and generic application framework remain provider-neutral.
- Add SDK discovery metadata and examples for classify failure, list recovery points, retry, repair state, resume.
- Add trace, audit, health, snapshot, replay, and boundary gates proving the pack uses the canonical service path.

## Impact

- Affected specs: `pack-workflow-recovery`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs in protocol crates, descriptor/admission validators, SDK discovery/command helpers, recovery service provider, optional provider registration, trace/audit tests, and dependency-boundary gates.
- Non-goals: no application-specific workflow, no provider-name routing in OS layers, no concrete provider construction in SDK/shell/kernel, and no fake success for absent providers.

## Supplier/API Baseline To Match

Recovery must be modeled after durable workflow and orchestration systems:

- Temporal retries, event history, reset, and continue-as-new:
  `https://docs.temporal.io/` establish deterministic replay, retry policies,
  workflow reset points, failure classification, and history compaction.
- Apache Airflow retry, clear, backfill, and task instance state:
  `https://airflow.apache.org/docs/` establishes explicit task instance
  states, retries, backfills, manual repair, and scheduler-visible recovery.
- Kubernetes Job/Pod restart policy and backoff:
  `https://kubernetes.io/docs/` establishes restart policy, backoff limits,
  terminal failure, controller reconciliation, and status conditions.
- Saga/compensation patterns:
  `https://microservices.io/patterns/data/saga.html` establishes compensating
  actions and durable recovery plans for partially completed workflows.

Macaca's target is provider-neutral failure classification, checkpoint
discovery, recovery point selection, retry/backoff, repair, resume,
compensation reference, and replay diagnostics. It must repair generic
workflow/task state without application-specific business branches.

## Industrial Acceptance Bar

This proposal is not complete until implementation tasks produce:

- a developer guide at `docs/developer-packs/workflow/recovery.md`;
- typed failure, checkpoint, retry policy, recovery plan, repair action, resume,
  compensation reference, and replay export DTOs;
- deterministic tests for transient/permanent failure classification, retry
  budget exhaustion, corrupted checkpoint rejection, resume after restart,
  compensation reference preservation, and replay export redaction;
- audit replay proving recovery decisions are explainable from sanitized event
  history and cannot silently skip policy or resource gates.
