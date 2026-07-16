# Workflow Review Pack

`pack.workflow.review.v1` describes provider-neutral review request, finding,
fix, rereview, approval, dismissal, and closure-gate capabilities. The pack is
descriptor-only until a review provider is registered through the runtime
composition root.

## Manifest Declaration

Declare the pack as required only when review gates are mandatory for readiness.
Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.workflow.review.v1"]
```

## Permissions

Use the narrowest scope: `workflow.review.request`,
`workflow.review.write`, `workflow.review.approve`,
`workflow.review.dismiss`, `workflow.review.finding.read`, and
`workflow.review.admin`.

## Capability Model

Macaca models reviews as requests, rounds, reviewer pool references, findings,
fix requests, rereview requests, outcomes, dismissal reasons, and closure gates.
Raw subjects, raw findings, comments, prompts, provider payloads, credentials,
and unbounded logs stay behind provider adapters and must not appear in traces,
snapshots, or SDK diagnostics.

## Platform Comparison

GitHub and GitLab review flows, code-review bots, document redlining,
enterprise safety reviews, app-store review gates, and workflow-engine review
tasks map to review requests, rounds, findings, fix requests, outcomes, and
closure gates. Code review, document review, compliance review, and product
review remain provider or application-specific semantics.

## Commands

`review.request_review`, `review.record_finding`,
`review.request_fix`, `review.request_rereview`, `review.approve`,
`review.close_review`, `review.dismiss`, `review.list_findings`,
`review.evaluate_gate`, and `review.inspect_provider` are descriptor-owned
schema names. SDK helpers build canonical traced service calls; providers
execute behind the service runtime.

## App-Facing Examples

- Request review with subject, subject revision hash, requester, schema, and
  redaction references.
- Record findings with severity, status, subject-span references, evidence
  references, and blocking flags.
- Request fixes and rereviews without shell-owned retry or review state
  machines.
- Approve, dismiss, or close reviews only when the closure gate proves no
  unresolved blocking findings for the same revision.
- List findings through bounded pagination and handle stale revision results.

## Trace And Audit

Traces should record declaration, admission decision, command name, request ref,
round ref, finding ref, fix ref, outcome ref, gate ref, subject revision hash,
provider class, capability hash, result status, and blocking count. They must
not record raw reviewed subjects, raw comments, prompts, credentials, provider
payloads, or unbounded logs.

## Provider Authors

Descriptors must report review-round semantics, finding lifecycle, severity
taxonomy, fix and rereview rules, closure-gate behavior, dismissal policy,
history bounds, health, and snapshot metadata. Providers must return structured
denied, unavailable, unsupported, conflict, stale-revision,
blocking-findings, dismissal-denied, cancelled, quota, timeout, and failure
results.

Conformance tests should cover descriptor completeness, review request
admission, finding lifecycle, fix and rereview flow, approval and dismissal
policy, closure gates, stale revisions, policy hooks, trace and audit events,
unavailable behavior, snapshot/replay, and restart recovery.
