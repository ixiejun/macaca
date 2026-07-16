# Change: Add Workflow Approval Pack

## Why

Developers need `pack.workflow.approval.v1` as a real industrial capability for approval request, decision capture, policy binding, escalation, and evidence replay. The pack must not be a catalog label only: if an application declares it and the provider is installed, the SDK must expose callable typed commands; if policy, entitlement, provider, or host support is absent, Macaca must return explicit unavailable or denied diagnostics.

This child proposal intentionally narrows the broad industrial catalog into one implementable service-backed pack. It records the research basis, capability boundary, command surface, permission model, observability model, and acceptance gates needed before this pack can be called production-ready.

## Research And Borrowed Platform Patterns

The proposal borrows stable ideas from mature application platforms without copying their product-specific APIs:

- OS background task and notification patterns: scheduling and lifecycle are platform services, not app loops.
- Android foreground/background execution limits: long-running work must expose state and user-sensitive approval.
- Windows app lifecycle and notification activation: work resumption needs identity and manifest-visible behavior.
- Apple background task/privacy patterns: autonomous work needs explicit capability and observable lifecycle.

The Macaca interpretation is: app manifests declare pack intent; admission validates capability and permission metadata; runtime permission and approval gates happen before sensitive side effects; concrete providers stay behind service descriptors; traces and audits prove the execution path.

## What Changes

- Add the provider-neutral `pack.workflow.approval.v1` contract under the `workflow` family.
- Define a concrete command namespace, initial industrial command set, result/error DTO requirements, permission scopes, policy defaults, resource/entitlement behavior, and unavailable diagnostics.
- Bind implementation ownership to approval service provider; the kernel, SDK, shells, and generic application framework remain provider-neutral.
- Add SDK discovery metadata and examples for request approval, record decision, escalate, cancel approval, inspect evidence.
- Add trace, audit, health, snapshot, replay, and boundary gates proving the pack uses the canonical service path.

## Impact

- Affected specs: `pack-workflow-approval`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs in protocol crates, descriptor/admission validators, SDK discovery/command helpers, approval service provider, optional provider registration, trace/audit tests, and dependency-boundary gates.
- Non-goals: no application-specific workflow, no provider-name routing in OS layers, no concrete provider construction in SDK/shell/kernel, and no fake success for absent providers.

## Supplier/API Baseline To Match

The pack must be designed against vendor-grade approval systems rather than a
single demo form:

- GitHub Actions Environments required reviewers:
  `https://docs.github.com/actions/deployment/targeting-different-environments/using-environments-for-deployment`
  establishes protected resources, wait timers, reviewer identity, bypass
  control, and auditability before a deployment side effect proceeds.
- ServiceNow Flow Designer approvals:
  `https://docs.servicenow.com/` establishes explicit approver assignment,
  delegation, escalation, rejection, cancellation, and business-rule-backed
  evidence without exposing provider internals to callers.
- Camunda user tasks and authorization:
  `https://docs.camunda.io/` establish claim/complete semantics, candidate
  users/groups, form/evidence payloads, due dates, and process-state linkage.
- Temporal workflow signals and queries:
  `https://docs.temporal.io/` establish durable human-in-the-loop decisions,
  replay-safe event history, timeout handling, cancellation, and deterministic
  workflow continuation.

Macaca's target is the common contract behind those systems: approval requests
are durable resources with policy-bound subjects, approver constraints,
deadline/escalation rules, decision records, and replayable evidence. The
service may adapt to many providers later, but the OS-visible contract remains
provider-neutral.

## Industrial Acceptance Bar

This proposal is not complete until implementation tasks produce:

- a developer guide at `docs/developer-packs/workflow/approval.md`;
- provider-neutral request, decision, escalation, cancellation, and evidence
  DTOs with stable schema versions;
- deterministic tests for duplicate decisions, revoked approver permission,
  timeout escalation, cancellation races, and provider absence;
- audit replay proving an approval-gated side effect cannot proceed without a
  valid decision record linked to the same trace, task, session, application,
  tenant, and policy template hash.
