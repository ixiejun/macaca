# Agent Complete Self-Evolution Design

## Context

Macaca currently proves a governed Skill-level self-evolution loop, not complete
agent self-evolution. Real `/api/chat/v2` tasks can create proposals, selected
proposals can materialize into proposal-linked Skill packages, registry/load-path
visibility is observable, usage telemetry survives restart through a durable
journal, and API-first audit can verify operations, registry/load-path, and
observer evidence.

The remaining gap is platform autonomy. The OS still lacks a continuously
running, policy-gated loop that discovers improvement opportunities, generates
candidate changes, evaluates them with normalized evidence, promotes or rolls
them back, and publishes governed capability updates without embedding
application-specific behavior.

This design is bounded by:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`
- `macaca/docs/design_patterns.md`
- `openspec/AGENTS.md`

## Brainstormed Approaches

### Option A: Extend the existing Skill service loop only

This keeps scope small by adding stricter Skill admission, paired benchmark
evaluation, quarantine, canary, and rollback around the existing proposal and
materialization lanes.

Benefits:

- Fastest route to stronger proof because existing Skill service slices already
  contain proposal processing, materialization, registry/load-path, telemetry,
  and audit surfaces.
- Low kernel risk because all behavior stays inside service/provider boundaries.

Limits:

- Still cannot claim "agent complete self-evolution" because Macaca source,
  application ABI changes, task/review policy changes, and package release
  governance remain outside the loop.
- Optimization would remain Skill-centric.

### Option B: Add an Autonomy Evolution Control Plane

This adds a service-owned control plane that treats every candidate improvement
as an evidence-backed evolution run. Skills remain one target type, but the same
loop can later route candidates to task-policy, context, memory, package,
application ABI, or OS-code proposals through governed target adapters.

Benefits:

- Fits Macaca's microkernel/service model: the kernel keeps only identity,
  policy, trace, audit, scheduler, and package-guard primitives.
- Creates a generic loop: discover -> propose -> evaluate -> admit -> canary ->
  promote -> observe -> rollback.
- Gives clear extension points for Skill packages today and OS-code changes
  later without hardcoding application behavior.

Limits:

- Requires multiple OpenSpec changes and careful service-boundary work.
- Needs a production Store/EventLog path before 7x24 unattended operation is
  credible.

### Option C: Build a full self-modifying code agent immediately

This would let agents edit Rust/frontend code, run tests, commit, canary, and
roll back automatically.

Benefits:

- Closest to the phrase "complete self-evolution."

Limits:

- Too risky as the next slice. Without standard metrics, Store/EventLog
  durability, capability diffs, blast-radius scoring, quarantine, and release
  gates, this would turn source-code mutation into an unsafe shell workflow.
- High chance of violating the boundary documents by moving semantics into
  scripts, Web, CLI, or application-specific conventions.

## Recommended Direction

Use Option B, delivered incrementally. The first milestone turns the current
Skill-level proof into a service-owned autonomous policy loop. Later milestones
generalize the same loop to application-scoped capability packs and OS-code
change proposals.

This is the most conservative architecture that can still reach the target. It
preserves the microkernel boundary, keeps shells thin, avoids application
hardcoding, and makes every autonomous mutation auditable and reversible.

## Target Architecture

The stable shape is:

```text
Application Agent Task
  -> Observability/EventLog evidence
  -> Evolution Candidate Discovery service
  -> Evolution Control Plane service
  -> Target Adapter Strategy
       - Skill package adapter
       - Application capability-pack adapter
       - Task/context policy adapter
       - OS-code proposal adapter
  -> Evaluation/Benchmark service
  -> Policy/Entitlement/Package Guard decorators
  -> Quarantine/Canary/Promotion/Rollback state machine
  -> Store/EventLog governance ledger
  -> SDK/SystemFacade
  -> Web/CLI diagnostic adapters
```

The control plane owns orchestration state, not target-specific mutation. Each
target adapter owns only the mechanics for one replaceable target type. The
Skill adapter delegates to existing Skill service commands. A future OS-code
adapter creates OpenSpec/Superpowers/GitNexus-backed change proposals before any
source mutation.

## Ownership

| Area | Owner |
| --- | --- |
| Evolution run identity, policy-required state, audit refs | Kernel primitives and service runtime contracts only |
| Discovery, candidate queues, lifecycle orchestration | Autonomy Evolution Control Plane service |
| Skill proposal/materialization/usage | Skill service |
| Metrics, paired benchmarks, quality scoring | Evaluation service |
| Durable governance ledger, snapshots, replay cursors | Store/EventLog service |
| Entitlement, approval, package admission, blast-radius policy | Policy, Entitlement, Package Guard services |
| Target-specific apply/rollback mechanics | Replaceable target adapter strategies |
| Developer/shell access | SDK/SystemFacade focused clients |
| Rendering and diagnostics | Web/CLI/frontend thin adapters |

## Design Patterns

- Command: every cross-boundary operation is a typed command/result.
- Facade: SDK/SystemFacade exposes stable clients for shells and applications.
- Strategy: candidate discovery, target adapters, metric policies, admission
  gates, canary policies, and rollback policies are replaceable.
- Decorator: trace, policy, resource, entitlement, package guard, metering, and
  sanitization wrap every side-effecting command.
- State: evolution runs move through explicit lifecycle states.
- Observer: task, skill, context, Store/EventLog, benchmark, canary, and rollback
  events are consumed as evidence.
- Memento: snapshots, benchmark baselines, rollback refs, and release refs are
  replayable.
- Specification: admission checks, Skill Creator quality rules, normalized
  benchmark pass/fail rules, and blast-radius thresholds are executable gates.
- Abstract Factory: runtime-host composition creates built-in, plugin, remote,
  mock, and unavailable providers.

## Required Lifecycle

An evolution run uses this state machine:

```text
Observed
  -> CandidateQueued
  -> CandidateClassified
  -> ProposalGenerated
  -> AdmissionReview
  -> Quarantined
  -> BenchmarkPrepared
  -> BaselineMeasured
  -> CandidateMeasured
  -> CanaryRunning
  -> Promoted
  -> ActiveMonitoring
  -> Superseded | RolledBack | Rejected | Inconclusive
```

Every transition records trace id, policy decision id, actor, target type,
application/tenant scope, bounded evidence refs, sanitized metrics, and rollback
or rejection reason when applicable.

## Scope Boundaries

The next upgrade must not:

- Put self-evolution orchestration into the kernel.
- Let Web, CLI, or frontend classify, promote, roll back, or benchmark
  candidates.
- Branch on application names, workflow names, provider names, model names,
  driver names, or business domains.
- Treat a materialized package, telemetry counter, or generated artifact alone
  as optimization proof.
- Store raw prompts, raw provider payloads, raw manifests, package bytes,
  secrets, credentials, private keys, raw signatures, or unbounded output in
  observability surfaces.

## Success Criteria

Macaca can claim complete agent self-evolution only when these are true:

1. A real app-scoped agent task can autonomously create a candidate evolution run
   without manual operator invocation.
2. The candidate is admitted or rejected by executable quality, safety, and
   policy gates.
3. Accepted Skill candidates enter quarantine, paired benchmark, canary,
   promotion, active monitoring, and rollback-capable states.
4. Standard metrics include token counts, elapsed time, retries, tool calls,
   failure recovery, quality score, intervention rate, policy decisions,
   activation/use/success counters, and regression reasons.
5. Store/EventLog, not local ad hoc files, is the durable governance source of
   truth for runs, metrics, snapshots, release refs, and rollback refs.
6. API-first audit can reconstruct the full chain after restart.
7. OS-code evolution is represented as governed OpenSpec/Superpowers/GitNexus
   proposals before any source mutation, with blast-radius scoring and release
   gates.
