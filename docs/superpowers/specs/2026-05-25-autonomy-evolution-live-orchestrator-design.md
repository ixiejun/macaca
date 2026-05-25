# Autonomy Evolution Live Orchestrator Design

## Context

The previous complete-self-evolution slices created the Autonomy Evolution
Control Plane, admission quality gates, normalized paired benchmarking, release
safety gates, a governance ledger boundary, and a non-mutating OS-code proposal
adapter. Those pieces define the governed vocabulary for self-evolution, but
they do not yet run a continuous unattended policy loop.

The missing piece is a live orchestrator that consumes observer evidence,
discovers candidate evolution opportunities, advances the control-plane state
machine, invokes admission, runs paired benchmark collection, evaluates release
safety, dispatches target adapter apply or rollback intents, appends governance
ledger records, and exposes API-first audit reconstruction.

This design is bounded by:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`
- `macaca/docs/design_patterns.md`
- `openspec/AGENTS.md`

## Brainstormed Approaches

### Option A: Put orchestration inside the existing control-plane provider

This keeps the number of crates small by adding a background loop directly to
`service.autonomy_evolution`.

Benefits:

- Fastest implementation path because DTOs and state machine already exist.
- Fewer service descriptors and fewer SDK clients.

Limits:

- Risks turning the control-plane service into a large semantic owner for
  discovery, workload execution, release decisions, and target mutation.
- Makes lease, scheduling, and replay behavior harder to replace independently.

### Option B: Add a thin live orchestrator service over existing services

This creates a dedicated service-owned orchestrator that owns only loop
progress, leases, phase ordering, idempotency keys, and audit stitching. It calls
the existing autonomy evolution, admission, benchmark, release, ledger, task,
skill, store, policy, and audit services through typed commands.

Benefits:

- Preserves the microkernel boundary and keeps shells thin.
- Keeps discovery, benchmark collection, target adapters, and release policies
  replaceable through Strategy interfaces.
- Makes the unattended loop auditable and resumable without moving target
  mutation into the orchestrator.

Limits:

- Requires more service wiring than Option A.
- Needs careful idempotency and lease semantics to avoid duplicate promotion or
  rollback.

### Option C: Build an end-to-end autonomous mutation worker

This would create a worker that directly discovers, mutates, benchmarks,
promotes, rolls back, and commits changes.

Benefits:

- Shortest path to a visible "complete self-evolution" demo.

Limits:

- Violates the service boundary by mixing discovery, mutation, benchmark,
  release, and audit semantics in one worker.
- Too risky for OS-code evolution because it would bypass existing
  OpenSpec/Superpowers/GitNexus and release-safety gates.

## Recommended Direction

Use Option B. The live orchestrator should be thin, resumable, and
service-owned. It should not classify application business domains, write Skill
files, mutate OS source code, or infer lifecycle state from local filesystem
paths. Its job is to connect the six existing governed building blocks into one
unattended loop with explicit leases, idempotency, policy checks, and audit
records.

## Target Architecture

```text
Observer/EventLog evidence
  -> Autonomy Evolution Live Orchestrator
       - lease acquisition
       - candidate discovery Strategy
       - phase planner State machine
       - idempotency guard
       - audit stitcher
  -> Autonomy Evolution Control Plane transition commands
  -> Admission quality gate command
  -> Benchmark workload runner Strategy
  -> Normalized paired benchmark scoring command
  -> Release safety command
  -> Target Adapter Strategy
       - Skill package adapter first
       - OS-code proposal adapter remains non-mutating
       - Unsupported targets fail closed
  -> Governance Ledger append/replay
  -> API-first audit reconstruction
```

## Ownership

| Area | Owner |
| --- | --- |
| Loop leases, phase ordering, idempotency, audit stitching | Autonomy Evolution Live Orchestrator service |
| Lifecycle state validation | Autonomy Evolution Control Plane service |
| Candidate metadata quality | Admission quality gate Specification |
| Workload collection and paired measurement | Benchmark workload runner Strategy plus normalized benchmark service |
| Release safety decisions | Release safety Strategy |
| Target-specific apply and rollback | Replaceable Target Adapter Strategies |
| Durable replay source | Governance ledger Store/EventLog Strategy |
| Shell/API visibility | SDK/SystemFacade and Web/CLI diagnostic adapters |

## Design Patterns

- Command: every phase boundary uses typed command/result DTOs.
- State: the orchestrator records phase progress and delegates lifecycle state
  validation to the control plane.
- Strategy: candidate discovery, workload collection, target adapters,
  canary observation, and audit reconstruction are replaceable.
- Decorator: trace, policy, entitlement, resource, and metering decorators wrap
  side-effecting service calls.
- Observer: task, skill, benchmark, release, rollback, and ledger events become
  bounded evidence refs.
- Memento: baseline/candidate measurements, release refs, rollback refs, and
  ledger checkpoints allow replay and rollback.
- Specification: admission, benchmark comparability, release safety, and
  duplicate/idempotency checks are executable gates.
- Facade: SDK/SystemFacade exposes focused live-orchestrator commands.
- Abstract Factory: runtime-host composes built-in, plugin, remote, mock, and
  unavailable orchestrator providers.

## Loop Semantics

Each loop tick is bounded and idempotent:

1. Acquire a lease for an application/tenant/session/task scope or a global
   maintenance scope.
2. Read bounded observer evidence and ledger cursors.
3. Discover candidate opportunities without reading raw prompts, manifests,
   package bytes, provider payloads, or application-specific content.
4. Advance the control-plane lifecycle from observation through candidate
   classification and proposal generation.
5. Evaluate admission. Denied or missing-evidence candidates are recorded and
   not retried until new evidence appears.
6. Prepare and collect paired baseline/candidate measurements for a generic
   task family.
7. Score the benchmark. Failed candidates go to rejected or rollback-capable
   states; inconclusive candidates remain quarantined with bounded reasons.
8. Evaluate release safety for quarantine, canary, promotion, monitoring, or
   rollback.
9. Dispatch apply or rollback only through target adapters.
10. Append every transition, decision, measurement, release result, adapter
    result, and audit reconstruction checkpoint to the governance ledger.

## Safety Rules

- The orchestrator must never write Skill files directly.
- The orchestrator must never write OS source files, run shell commands, apply
  patches, or commit code.
- The orchestrator must never branch on application names, workflow names,
  provider names, driver names, model names, or business domains.
- A missing service, missing adapter, invalid lease, duplicate idempotency key,
  failed admission, failed benchmark, failed release gate, or missing rollback
  memento must return a structured unavailable, denied, rejected, rolled-back,
  or inconclusive result.
- Logs, snapshots, ledger records, and audit payloads must contain only bounded
  refs, counts, state names, reason codes, and sanitized diagnostics.

## Success Criteria

Macaca can claim this slice is complete when:

1. A real observer evidence ref can create or resume an orchestrated evolution
   run without manual materialization/operator invocation.
2. The orchestrator progresses at least one Skill candidate through discovery,
   control-plane transition, admission, paired benchmark scoring, release safety,
   target adapter dispatch, governance ledger append, and API audit
   reconstruction.
3. Duplicate ticks do not duplicate promotion, rollback, or ledger records.
4. Missing providers and unsupported targets fail closed.
5. Restart replay resumes from ledger cursors rather than workspace-local
   process memory.
6. OS-code targets are limited to the non-mutating proposal adapter until a
   separate source-mutation proposal is approved.
