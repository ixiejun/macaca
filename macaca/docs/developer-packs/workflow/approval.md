# Workflow Approval Pack

`pack.workflow.approval.v1` describes provider-neutral approval request,
decision, evidence, escalation, and gate capabilities. The pack is
descriptor-only until an approval provider is registered through the runtime
composition root.

## Manifest Declaration

Declare the pack as required only when approval gates are mandatory for
readiness. Optional declarations degrade with structured unavailable
diagnostics.

```toml
[service_contract]
optional_packs = ["pack.workflow.approval.v1"]
```

## Permissions

Use the narrowest scope: `workflow.approval.request`,
`workflow.approval.decide`, `workflow.approval.escalate`,
`workflow.approval.read`, and `workflow.approval.admin`.

## Capability Model

Macaca models approvals as requests, assignments, eligible principal
references, decision records, evidence bundles, escalation links, cancellation,
and consumable decision gates. Raw evidence, prompts, identity payloads,
provider payloads, credentials, private comments, and unbounded assignment
history stay behind provider adapters and must not appear in traces, snapshots,
or SDK diagnostics.

## Platform Comparison

macOS authorization prompts, Windows UAC consent, Android permission prompts,
OpenHarmony permission dialogs, ITSM approval systems, GitHub protected
environment approvals, and workflow-engine human tasks map to approval requests,
assignments, decisions, evidence bundles, and decision gates. UI rendering and
provider-native approval forms remain shell or provider concerns.

## Commands

`approval.request_approval`, `approval.record_decision`,
`approval.escalate`, `approval.cancel_approval`,
`approval.inspect_evidence`, `approval.list_pending`,
`approval.evaluate_gate`, and `approval.inspect_provider` are descriptor-owned
schema names. SDK helpers build canonical traced service calls; providers
execute behind the service runtime.

## App-Facing Examples

- Request approval with subject, policy hash, requester, deadline, schema, and
  redaction references.
- Assign or escalate approval eligibility without leaking raw identity payloads.
- Record a decision once and treat duplicate or revoked eligibility as explicit
  conflict states.
- Link protected side effects to approval gates and evaluate those gates before
  executing privileged commands.
- Cancel stale approvals and keep shell approval surfaces as rendering adapters.

## Trace And Audit

Traces should record declaration, admission decision, command name, request ref,
assignment ref, decision ref, gate ref, evidence ref, policy hash, provider
class, capability hash, result status, and consumption state. They must not
record raw evidence, prompts, credentials, provider payloads, private comments,
or raw identity documents.

## Provider Authors

Descriptors must report assignment semantics, decision immutability, escalation
rules, gate consumption modes, eligibility recheck behavior, evidence redaction,
health, and snapshot metadata. Providers must return structured denied,
unavailable, unsupported, conflict, expired, cancelled, eligibility-revoked,
duplicate-decision, quota, timeout, and failure results.

Conformance tests should cover descriptor completeness, request admission,
assignment eligibility, decision idempotency, escalation, cancellation, evidence
redaction, gate evaluation and consumption, policy hooks, trace and audit
events, unavailable behavior, snapshot/replay, and restart recovery.
