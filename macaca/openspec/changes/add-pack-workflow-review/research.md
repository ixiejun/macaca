# Workflow Review Pack Research

## Purpose

This note records borrowed platform patterns, Macaca provider-neutral mapping,
reuse inventory, and GitNexus memo evidence for `pack.workflow.review.v1`.
The review pack owns review requests, rounds, findings, fix requests,
re-review, approval/closure, stale-subject handling, closure gates, freshness,
and audit records. It must not own approval decision gates for protected side
effects, delegation leases, recovery repair, task execution, or
application-specific review UI.

## Source Baseline

- GitHub Pull Request reviews, requested changes, comments, and stale reviews:
  <https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/reviewing-changes-in-pull-requests/about-pull-request-reviews>
- GitLab merge request approvals:
  <https://docs.gitlab.com/user/project/merge_requests/approvals/>
- Gerrit code review labels and submit rules:
  <https://gerrit-review.googlesource.com/Documentation/intro-user.html>
- Camunda user tasks as generic human-review work:
  <https://docs.camunda.io/docs/components/modeler/bpmn/user-tasks/>
- Temporal signals/updates for review state changes:
  <https://docs.temporal.io/develop/go/message-passing>

## Borrowed Platform Pattern Mapping

- Pull-request review rounds map to `ReviewRound`, subject revision hash, stale
  review behavior, findings, requested changes, and approval closure.
- Merge-request approval rules map to closure gates and blocking finding
  requirements without embedding repository semantics.
- Gerrit labels map to provider-neutral outcome and finding severity classes.
- Human-task patterns map to assigned reviewers and decision windows.
- Temporal signals map to replayable state changes for findings, fixes, and
  re-review requests.

## Macaca-Owned Abstractions

`pack.workflow.review.v1` should define `ReviewRequest`, `ReviewRound`,
`ReviewFinding`, `FixRequest`, `ReviewOutcome`, `ReviewClosureGate`,
`ReviewState`, `FindingState`, and `ReviewAuditRecord`.

The DTOs must carry reviewed subject reference, subject revision hash,
reviewer eligibility, finding severity, finding lifecycle, fix evidence,
re-review request, closure gate, dismissal authority, bounded diagnostics,
redaction class, and replay pointers. Raw prompts, hidden findings, private
review comments, provider payloads, credentials, and application-specific
review-board fields are rejected.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides canonical Facade
  calls for SDK helpers.
- Generic policy, resource, entitlement, trace, audit, artifact, mock-provider,
  unavailable-provider, task, and approval concepts are reusable, but current
  evidence does not prove review-specific DTOs, providers, SDK helpers, tests,
  or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
