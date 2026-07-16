# Workflow Delegation Pack

`pack.workflow.delegation.v1` describes provider-neutral delegation,
handoff, capacity, lease, and result collection capabilities. The pack is
descriptor-only until a delegation provider is registered through the runtime
composition root.

## Manifest Declaration

Declare the pack as required only when delegation is mandatory for readiness.
Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.workflow.delegation.v1"]
```

## Permissions

Use the narrowest scope: `workflow.delegation.create`,
`workflow.delegation.accept`, `workflow.delegation.cancel`,
`workflow.delegation.read`, and `workflow.delegation.admin`.

## Capability Model

Macaca models delegation as requests, candidate pools, claims, capacity
snapshots, leases, handoffs, result references, cancellation, and terminal
outcomes. Raw work payloads, agent private state, provider payloads,
credentials, raw results, and unbounded worker logs stay behind provider
adapters and must not appear in traces, snapshots, or SDK diagnostics.

## Platform Comparison

Work queues, actor mailboxes, Temporal activity assignment, Kubernetes
controllers, cloud task queues, Android foreground services, and OpenHarmony
background ability delegation map to delegation requests, atomic claims,
capacity snapshots, leases, handoffs, and result references. Agent selection and
business-specific work semantics remain provider or application concerns.

## Commands

`delegation.delegate`, `delegation.accept_delegation`,
`delegation.handoff`, `delegation.inspect_capacity`,
`delegation.collect_result`, `delegation.cancel_delegation`,
`delegation.renew_lease`, and `delegation.inspect_provider` are
descriptor-owned schema names. SDK helpers build canonical traced service calls;
providers execute behind the service runtime.

## App-Facing Examples

- Delegate work with a work reference, requester reference, candidate-pool
  reference, schema version, and redaction profile.
- Inspect capacity before assignment and accept delegation through an atomic
  claim.
- Renew, revoke, or expire leases without inventing shell-side agent state.
- Handoff work through checkpoint references and collect terminal result
  references.
- Cancel delegations explicitly when capacity or eligibility changes.

## Trace And Audit

Traces should record declaration, admission decision, command name, request ref,
candidate pool ref, claim ref, capacity ref, lease ref, handoff ref, result ref,
provider class, capability hash, result status, and terminal state. They must
not record raw work payloads, agent private state, raw results, credentials,
provider payloads, or unbounded logs.

## Provider Authors

Descriptors must report atomic claim semantics, lease behavior, capacity
dimensions, handoff support, result reference support, cancellation semantics,
health, and snapshot metadata. Providers must return structured denied,
unavailable, unsupported, conflict, lease-expired, capacity-exhausted,
ineligible-assignee, cancelled, quota, timeout, and failure results.

Conformance tests should cover descriptor completeness, capacity inspection,
atomic accept, lease expiry and renewal, handoff checkpoint safety, result
reference redaction, cancellation, policy hooks, trace and audit events,
unavailable behavior, snapshot/replay, and restart recovery.
