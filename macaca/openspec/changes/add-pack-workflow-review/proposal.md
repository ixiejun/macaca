# Change: Add Workflow Review Pack

## Why

Developers need `pack.workflow.review.v1` as a real industrial capability for review request, finding capture, fix loop, re-review, approval, and terminal-state closure. The pack must not be a catalog label only: if an application declares it and the provider is installed, the SDK must expose callable typed commands; if policy, entitlement, provider, or host support is absent, Macaca must return explicit unavailable or denied diagnostics.

This child proposal intentionally narrows the broad industrial catalog into one implementable service-backed pack. It records the research basis, capability boundary, command surface, permission model, observability model, and acceptance gates needed before this pack can be called production-ready.

## Research And Borrowed Platform Patterns

The proposal borrows stable ideas from mature application platforms without copying their product-specific APIs:

- OS background task and notification patterns: scheduling and lifecycle are platform services, not app loops.
- Android foreground/background execution limits: long-running work must expose state and user-sensitive approval.
- Windows app lifecycle and notification activation: work resumption needs identity and manifest-visible behavior.
- Apple background task/privacy patterns: autonomous work needs explicit capability and observable lifecycle.

The Macaca interpretation is: app manifests declare pack intent; admission validates capability and permission metadata; runtime permission and approval gates happen before sensitive side effects; concrete providers stay behind service descriptors; traces and audits prove the execution path.

## What Changes

- Add the provider-neutral `pack.workflow.review.v1` contract under the `workflow` family.
- Define a concrete command namespace, initial industrial command set, result/error DTO requirements, permission scopes, policy defaults, resource/entitlement behavior, and unavailable diagnostics.
- Bind implementation ownership to review service provider; the kernel, SDK, shells, and generic application framework remain provider-neutral.
- Add SDK discovery metadata and examples for request review, record finding, request fix, request rereview, approve.
- Add trace, audit, health, snapshot, replay, and boundary gates proving the pack uses the canonical service path.

## Impact

- Affected specs: `pack-workflow-review`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs in protocol crates, descriptor/admission validators, SDK discovery/command helpers, review service provider, optional provider registration, trace/audit tests, and dependency-boundary gates.
- Non-goals: no application-specific workflow, no provider-name routing in OS layers, no concrete provider construction in SDK/shell/kernel, and no fake success for absent providers.

## Supplier/API Baseline To Match

The review pack must match industrial review systems rather than a simple
boolean approval:

- GitHub pull request reviews and Checks:
  `https://docs.github.com/` establish review requests, comments/findings,
  requested changes, approvals, dismissed reviews, check runs, and merge gates.
- Gerrit review labels and submit rules:
  `https://gerrit-review.googlesource.com/Documentation/` establish label
  votes, reviewer identity, patch-set re-review, blocking findings, and submit
  requirements.
- GitLab merge request approvals:
  `https://docs.gitlab.com/` establish approval rules, required reviewers,
  code-owner-like constraints, re-review after changes, and audit history.
- Camunda user tasks:
  `https://docs.camunda.io/` establish review as a claimable human or automated
  task with completion evidence and process linkage.

Macaca's provider-neutral review contract must cover review request, finding
capture, fix request, re-review, approval, dismissal/closure, terminal-state
repair, and evidence replay without embedding code-review-specific or
application-specific business rules in the OS.

## Industrial Acceptance Bar

This proposal is not complete until implementation tasks produce:

- a developer guide at `docs/developer-packs/workflow/review.md`;
- typed review request, finding, review round, fix request, approval, dismissal,
  and closure DTOs;
- deterministic tests for repeated review rounds, stale findings after input
  revision changes, review dismissal, blocked terminal states, and re-review
  after fix evidence;
- audit replay proving a task cannot close as reviewed while unresolved blocking
  findings remain visible under the same policy scope.
