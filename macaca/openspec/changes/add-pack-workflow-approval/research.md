# Workflow Approval Pack Research

## Purpose

This note records borrowed platform patterns, Macaca provider-neutral mapping,
reuse inventory, and GitNexus memo evidence for `pack.workflow.approval.v1`.
The approval pack owns approval requests, assignments, decisions, escalation,
cancellation, evidence inspection, eligibility checks, decision consumption, and
audit records. It must not own task execution, delegation leases, review
findings, recovery repair, identity account/profile mutation, or
application-specific approval workflows.

## Source Baseline

- Camunda user tasks as a human decision/work item pattern:
  <https://docs.camunda.io/docs/components/modeler/bpmn/user-tasks/>
- GitHub Actions environments and required reviewers:
  <https://docs.github.com/en/actions/deployment/targeting-different-environments/using-environments-for-deployment>
- ServiceNow approval and delegation concepts:
  <https://docs.servicenow.com/>
- Temporal human-in-the-loop workflow patterns and signals:
  <https://docs.temporal.io/develop/go/message-passing>
- GitHub Pull Request review states as decision evidence:
  <https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/reviewing-changes-in-pull-requests/about-pull-request-reviews>

## Borrowed Platform Pattern Mapping

- Human task assignment maps to `ApprovalAssignment`, eligibility, claim, and
  escalation metadata.
- Required reviewer/environment gates map to `ApprovalDecisionGate`,
  approval-required results, expiry, and decision consumption checks.
- Service-management approval patterns map to requested, pending, claimed,
  escalated, decided, expired, cancelled, and consumed state transitions.
- Temporal signals map to provider-neutral decision recording and replayable
  evidence; the pack should not block OS execution on hidden provider callbacks.
- Pull-request reviews map to decision evidence, comments/findings references,
  dismissals, and audit trails without importing code-review semantics.

## Macaca-Owned Abstractions

`pack.workflow.approval.v1` should define `ApprovalRequest`,
`ApprovalAssignment`, `ApprovalDecision`, `ApprovalEvidenceBundle`,
`ApprovalDecisionGate`, `ApprovalState`, `ApprovalEscalation`,
`ApprovalConsumption`, and `ApprovalAuditRecord`.

The DTOs must carry request subject, protected side-effect reference, approver
eligibility, policy hash, trace lineage, decision reason, expiry, consumption
mode, idempotency key, redaction class, and replay pointers. Raw prompts,
credentials, provider payloads, hidden approver lists, and application-specific
approval forms are rejected.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides canonical Facade
  calls for SDK helpers.
- Generic policy, approval, entitlement, resource, trace, audit, artifact,
  mock-provider, and unavailable-provider concepts are reusable, but current
  evidence does not prove approval-specific DTOs, providers, SDK helpers, tests,
  or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
