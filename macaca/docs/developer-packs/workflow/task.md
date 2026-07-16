# Workflow Task Pack

`pack.workflow.task.v1` describes provider-neutral task lifecycle capabilities
for autonomous applications. The pack is descriptor-only until a task provider is
registered through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when task execution is mandatory for
readiness. Optional declarations degrade with structured unavailable
diagnostics.

```toml
[service_contract]
optional_packs = ["pack.workflow.task.v1"]
```

## Permissions

Use the narrowest scope: `workflow.task.read`, `workflow.task.write`,
`workflow.task.queue`, `workflow.task.claim`, `workflow.task.progress`,
`workflow.task.complete`, and `workflow.task.admin`.

## Capability Model

Macaca models tasks as specs, queues, dependencies, leases, attempts, bounded
retry policies, concurrency policies, progress references, checkpoints, artifact
references, and bounded history. Raw task payloads, prompts, worker logs,
artifacts, provider payloads, credentials, and unbounded history stay behind
provider adapters and must not appear in traces, snapshots, or SDK diagnostics.

## Platform Comparison

Temporal workflows, AWS Step Functions tasks, Kubernetes Jobs, Android
WorkManager, Windows background tasks, macOS launch services, and OpenHarmony
task primitives map to task specs, queues, leases, attempts, checkpoints, and
terminal states. Provider-native workflow names, worker identities, queue
topologies, and business payloads remain implementation details.

## Commands

`workflow_task.create`, `workflow_task.update`,
`workflow_task.patch_metadata`, `workflow_task.enqueue`,
`workflow_task.claim`, `workflow_task.heartbeat`, `workflow_task.release`,
`workflow_task.record_progress`, `workflow_task.record_checkpoint`,
`workflow_task.attach_artifact`, `workflow_task.complete`,
`workflow_task.fail`, `workflow_task.cancel`, `workflow_task.skip`,
`workflow_task.get`, `workflow_task.list`, `workflow_task.get_history`,
`workflow_task.snapshot`, and `workflow_task.inspect_provider` are
descriptor-owned schema names. SDK helpers build canonical traced service calls;
providers execute behind the service runtime.

## App-Facing Examples

- Create a task spec with queue, dependency, retry, concurrency, timeout, and
  checkpoint-policy references.
- Enqueue work with an idempotency key and inspect unavailable diagnostics when
  no provider is installed.
- Claim a task through a lease, heartbeat before the deadline, and release or
  revoke stale leases explicitly.
- Record progress, checkpoints, and artifact references without storing raw
  payloads in trace evidence.
- Complete, fail, cancel, or skip tasks through explicit terminal commands
  rather than shell-side Task Board repair.
- Model fail and retry flows through typed attempt records, retry policy
  references, and retry-exhausted diagnostics.
- Inspect dependency-blocked and concurrency-blocked states before claiming
  dependent work or additional work in a constrained group.
- Read bounded task history through cursor references and treat unavailable,
  denied, invalid-state, lease-expired, artifact-blocked, quota-exceeded, and
  version-mismatch outcomes as structured results.

## Trace And Audit

Traces should record declaration, admission decision, command name, task ref,
queue ref, lease ref, attempt index, checkpoint ref, artifact ref, provider
class, capability hash, result status, and state transition. They must not
record raw task payloads, prompts, artifacts, worker logs, credentials,
provider payloads, or unbounded histories.

## Provider Authors

Descriptors must report queue support, lease semantics, heartbeat deadlines,
retry limits, dependency limits, concurrency controls, checkpoint formats,
artifact reference support, history bounds, health, and snapshot metadata.
Providers must return structured denied, unavailable, unsupported,
invalid-state, lease-expired, dependency-blocked, retry-exhausted,
quota-exceeded, timeout, cancellation, provider-failure, and version-mismatch
results without exposing native payloads.

Conformance tests should cover descriptor completeness, queue admission, lease
atomicity, heartbeat expiry, retry bounds, dependency ordering, checkpoint
redaction, artifact references, state-machine transitions, policy hooks, trace
and audit events, unavailable behavior, snapshot/replay, and restart recovery.
