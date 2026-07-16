# Workflow Recovery Pack

`pack.workflow.recovery.v1` describes provider-neutral failure
classification, recovery point, retry, repair, resume, replay export,
compensation, and terminalization capabilities. The pack is descriptor-only
until a recovery provider is registered through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when recovery orchestration is mandatory for
readiness. Optional declarations degrade with structured unavailable
diagnostics.

```toml
[service_contract]
optional_packs = ["pack.workflow.recovery.v1"]
```

## Permissions

Use the narrowest scope: `workflow.recovery.read`,
`workflow.recovery.repair`, `workflow.recovery.resume`,
`workflow.recovery.retry`, `workflow.recovery.compensate`,
`workflow.recovery.export`, and `workflow.recovery.admin`.

## Capability Model

Macaca models recovery as failure records, recovery points, retry policies,
recovery plans, repair actions, compensation references, resume plans,
redacted replay exports, and terminalization evidence. Raw checkpoint bytes,
prompts, provider payloads, credentials, replay payloads, package bytes, and
unbounded logs stay behind provider adapters and must not appear in traces,
snapshots, or SDK diagnostics.

## Platform Comparison

Temporal replay and reset, Saga compensation, Kubernetes controller
reconciliation, database migration repair, message-queue dead-letter replay,
mobile background task retry, and OpenHarmony ability recovery map to failure
records, recovery points, retry budgets, repair plans, compensation references,
resume plans, and replay exports. Application-specific repair semantics remain
with the owning application or service.

## Commands

`recovery.classify_failure`, `recovery.list_recovery_points`,
`recovery.retry`, `recovery.repair_state`, `recovery.resume`,
`recovery.export_replay`, `recovery.build_plan`,
`recovery.apply_compensation`, `recovery.terminalize`, and
`recovery.inspect_provider` are descriptor-owned schema names. SDK helpers
build canonical traced service calls; providers execute behind the service
runtime.

## App-Facing Examples

- Classify failures with origin service, reason code, retryability, trace, and
  redaction references.
- List recovery points and verify integrity and compatibility before resume.
- Build a recovery plan with bounded repair actions, retry policy, and optional
  compensation references.
- Retry or resume only while retry budgets and compatibility gates allow it.
- Export redacted replay evidence for audit without exposing raw checkpoint
  bytes or provider payloads.

## Trace And Audit

Traces should record declaration, admission decision, command name, failure ref,
origin service ref, recovery point ref, plan ref, action ref, compensation ref,
resume ref, replay export ref, provider class, capability hash, result status,
and retry budget state. They must not record raw checkpoint bytes, prompts,
credentials, package bytes, provider payloads, replay payloads, or unbounded
logs.

## Provider Authors

Descriptors must report failure classes, recovery point formats, integrity
checks, compatibility rules, retry budgets, repair action types, compensation
ordering, replay redaction, health, and snapshot metadata. Providers must
return structured denied, unavailable, unsupported, conflict,
corrupted-checkpoint, retry-budget-exhausted, incompatible-checkpoint,
terminalized, quota, timeout, and failure results.

Conformance tests should cover descriptor completeness, failure classification,
recovery point integrity, retry budgets, repair plan validation, compensation
ordering, resume compatibility, redacted replay export, terminalization, policy
hooks, trace and audit events, unavailable behavior, snapshot/replay, and
restart recovery.
