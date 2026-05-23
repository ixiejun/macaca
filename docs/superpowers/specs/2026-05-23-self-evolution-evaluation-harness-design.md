# Self-Evolution Evaluation Harness Design

## Brainstorm Summary

Macaca needs to prove two different claims before calling agent self-evolution
real. First, a governed evolution loop must actually occur and leave
machine-checkable evidence. Second, the evolved skill state must improve later
real task execution without violating OS boundaries, policy, trace, audit, or
sanitization rules.

Three approaches were considered:

1. Run only black-box task comparisons.
   - Benefit: fastest way to see whether later tasks feel better.
   - Risk: cannot prove that improvement came from governed self-evolution.
2. Run only service-level white-box checks.
   - Benefit: strong evidence for governance, proposal, curation, and rollback.
   - Risk: does not prove user-visible task performance improves.
3. Combine white-box governance gates with black-box task A/B measurement.
   - Benefit: separates "evolution happened" from "evolution helped".
   - Risk: requires a small evaluation harness and disciplined metrics.

## Selected Approach

Use option 3. The evaluation harness records a white-box chain from verified
task completion through later skill activation, then compares baseline and
evolved task runs with bounded, provider-neutral metrics.

## Design Pattern Mapping

- Command: evaluation starts, checkpoints, run records, and reports are typed
  commands/results rather than shell-owned scripts.
- Facade: SDK/SystemFacade exposes evaluation operations to Web, CLI,
  frontend, Task, and test harnesses without leaking provider construction.
- Strategy: task fixture selection, metric scoring, pass/fail thresholds, and
  evidence collection are replaceable policies.
- Decorator: trace, policy, resource, entitlement, audit, and sanitization wrap
  every evaluation write.
- State: evaluation lifecycle is explicit: prepared, baseline recorded,
  evolved recorded, scored, passed, failed, or inconclusive.
- Observer: task completion, proposal, curation, promotion, rollback, and skill
  activation events are observed through existing service evidence.
- Memento: baseline/evolved snapshots and rollback refs allow replay and
  restoration after evaluation.
- Specification: white-box gates, black-box metrics, leakage checks, and
  minimum improvement thresholds are executable rules.

## White-Box Gate

The harness must prove the complete evolution chain:

```text
verified task completion
  -> ExperienceCandidate
  -> classification
  -> draft or patch proposal
  -> curation dry-run
  -> approval, promote, or apply
  -> active catalog snapshot
  -> later skill activation
```

Each checkpoint records only sanitized identifiers and bounded counts:

- trace id.
- application id, session id, and task id when present.
- evidence refs.
- proposal id.
- curation run id.
- policy decision id.
- audit event ids.
- before and after snapshot refs.
- rollback ref.

The gate passes only when dry-run has no side effects, proposals stay out of
the active catalog until promotion, rejected proposals keep rationale and
evidence, rollback restores lifecycle and alias state, and reports contain no
raw prompt, secret, provider payload, package bytes, manifest, or full skill
body.

## Black-Box Effect Gate

The harness compares baseline and evolved runs for generic task families. A
task family is defined by capability shape and evidence requirements, not by a
specific application name or business workflow.

Recommended families:

- Spec-change loop: create or update a small OpenSpec slice and validate it.
- Runtime verification loop: start service surfaces, probe typed routes, and
  persist evidence refs.
- Bug-trace loop: reproduce a bounded issue, fix it, and verify the corrected
  behavior.

Each family runs twice:

- Baseline run hides or ignores the newly evolved skill state.
- Evolved run allows the promoted or improved skill state to participate.

The comparison records:

- completion success.
- verified artifact count.
- human intervention count.
- elapsed seconds.
- tool call count.
- retry count.
- policy violation count.
- skill activation count.
- proposal acceptance rate.
- reuse score.
- regression count.

## Pass Criteria

White-box gates must pass completely. Black-box gates must show no completion
regression, no policy or sanitization regression, at least one meaningful
efficiency improvement, and actual activation of the promoted or improved skill
in a later task.

Efficiency improvement is intentionally multi-dimensional. A run may pass by
reducing human interventions, retries, elapsed time, or unnecessary tool calls,
as long as verified artifacts and behavior quality do not regress.

## Ownership And Boundaries

The Skill service owns evolution and curation semantics. The evaluation service
or provider owns metric aggregation and pass/fail scoring. Store/EventLog owns
durable report and memento refs. Task/Autonomy owns verified task completion
events. Context owns skill catalog visibility. Policy/Entitlement owns
approval. Shells may display reports and submit commands only.

The harness must not hardcode application names, workflow names, provider
names, model names, driver names, gateway names, chain names, or business
domains. Task fixtures are generic and selected by declared capability shape.

## Logging, Trace, And Audit

Every evaluation checkpoint logs the command name, trace id, evaluation id,
phase, task family id, run id, metric counts, evidence count, policy decision
id, audit event count, result state, and bounded failure reasons. Logs and
reports must remain sanitized and replayable.

## Implementation Scope

The first implementation slice should add the OpenSpec contract and a focused
evaluation report model with deterministic scoring helpers. Later slices can
wire live service checkpoints, shell display, and fixture runners without
changing the metric contract.
