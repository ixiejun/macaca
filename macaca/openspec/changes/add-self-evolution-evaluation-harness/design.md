# Self-Evolution Evaluation Harness Design

## Context

The completed self-evolving Skill OS work provides governed proposals,
curation, lifecycle transitions, aliases, safe mutation, rollback, telemetry,
and operations surfaces. The next problem is measurement. Macaca must be able
to distinguish a real governed evolution loop from a generated draft that is
never promoted or reused.

## Goals

- Prove the white-box evolution chain from verified task completion to later
  skill activation with traceable checkpoints.
- Compare baseline and evolved task runs with provider-neutral metrics.
- Produce sanitized JSON and Markdown reports that are replayable and safe to
  expose in shell surfaces.
- Keep evaluation as a generic OS capability rather than an application-owned
  test script.

## Non-Goals

- Do not define application-specific benchmark tasks.
- Do not let Web, CLI, or frontend own scoring, pass/fail semantics, or
  checkpoint interpretation.
- Do not require an LLM, semantic provider, or remote telemetry system.
- Do not store raw prompts, raw provider payloads, package bytes, manifests,
  credentials, secrets, full skill bodies, or unbounded task output.

## Ownership Model

| Layer | Ownership |
| --- | --- |
| Skill service | Skill proposal, curation, lifecycle, alias, promotion, rollback, and activation evidence. |
| Evaluation provider | Metric aggregation, gate evaluation, report construction, and deterministic scoring policy. |
| Store/EventLog | Durable evaluation records, report refs, snapshot refs, rollback refs, audit event ids, and replay cursors. |
| Task/Autonomy services | Verified terminal task completion events and bounded evidence refs. |
| Context service | Evidence that an evolved skill became visible and was later read or activated. |
| Policy/Entitlement services | Approval, denial, and protected-resource decisions. |
| SDK/SystemFacade | Provider-neutral commands and Null Object behavior for evaluation operations. |
| Web/CLI/frontend | Thin display and command adapters only. |

## Design Patterns

- **Command**: evaluation start, checkpoint append, score, report, and rollback
  operations are typed commands/results.
- **Facade**: SDK clients hide provider construction from shells and
  applications.
- **Strategy**: metric thresholds, task-family fixtures, and scoring policies
  are replaceable.
- **Decorator**: trace, policy, resource, entitlement, audit, and sanitization
  wrap every evaluation write.
- **State**: evaluation runs move through explicit lifecycle states:
  `Prepared`, `BaselineRecorded`, `EvolvedRecorded`, `Scored`, `Passed`,
  `Failed`, and `Inconclusive`.
- **Observer**: existing task, skill, curation, promotion, rollback, and
  context events are observed as evidence instead of duplicated.
- **Memento**: before/after snapshots and rollback refs make evaluations
  replayable and reversible.
- **Specification**: white-box gates, black-box metrics, leakage checks, and
  pass/fail thresholds are executable rules.

## Evaluation Model

An evaluation record contains:

- evaluation id, trace id, actor kind, tenant id, and optional application id.
- task family id and generic capability tags.
- baseline and evolved run ids.
- white-box checkpoints for candidate, classification, proposal, curation,
  promotion/apply, catalog visibility, activation, rejection, and rollback.
- black-box metrics for success, artifacts, human intervention, elapsed time,
  tool calls, retries, policy violations, activations, acceptance rate, reuse,
  and regressions.
- scoring output with pass/fail state, reasons, and bounded diagnostics.
- report refs, snapshot refs, rollback refs, policy decision ids, and audit
  event ids.

Task family ids describe capability shape, such as `spec_change_loop`,
`runtime_verification_loop`, or `bug_trace_loop`. They are not application names
and must not branch into business-specific behavior.

## White-Box Gate

The system SHALL verify this chain:

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

Dry-run immutability, proposal isolation, rejection durability, rollback
restoration, policy decisions, audit ids, and sanitization are hard gates.

## Black-Box Gate

The system SHALL compare baseline and evolved task runs. A passing evolved run
must preserve completion quality, avoid policy/sanitization regressions, show at
least one efficiency improvement, and prove that the evolved skill state was
read or activated.

Efficiency can improve through fewer human interventions, fewer retries, lower
elapsed time, lower unnecessary tool calls, or higher verified artifact density.

## Logging And Reports

Every checkpoint and scoring operation emits structured logs with evaluation id,
trace id, phase, task family id, run ids, metric counts, evidence counts, policy
decision ids, audit event counts, result state, and bounded failure reasons.
Reports are generated as JSON plus Markdown summary refs. Both formats must be
sanitized.

## Live Completion Observer Boundary

Live task-loop proposal extraction is observed at the `service.agent_execution`
completion boundary. The Web composition root wraps its concrete
`AgentExecutionBackend` with a Decorator that records bounded observer EventLog
checkpoints and forwards successful `AgentExecutionResult` values to the Skill
service through `skill.evolution.propose_from_task`.

The wrapper is an Observer only. It does not parse prompts, branch on
application names, inspect business output, mutate skill files, promote drafts,
or decide curation lifecycle. Proposal validation, storage, promotion,
rejection, and rollback remain owned by the Skill service. If Skill
self-evolution is unavailable or rejects a candidate, the wrapper records a
structured observer outcome and still returns the original Agent Execution
result.

## Migration Plan

1. Add the OpenSpec requirement and a focused evaluation report model.
2. Add deterministic scoring helpers and tests for white-box and black-box
   gates.
3. Add SDK/SystemFacade commands and unavailable behavior.
4. Wire runtime-host provider checkpoints to existing Skill, Task, Store,
   Context, Policy, and Audit evidence.
5. Add thin Web/CLI/frontend display adapters.
6. Add live evaluation fixtures and operator runbook.

## Risks And Mitigations

- Risk: evaluation becomes a benchmark for one application.
  Mitigation: require generic task family ids and capability tags only.
- Risk: metric gaming hides quality regressions.
  Mitigation: require verified artifacts and no completion regression before
  efficiency improvements count.
- Risk: reports leak sensitive task data.
  Mitigation: use bounded refs and sanitization gates for every report field.
- Risk: harness blocks normal curation.
  Mitigation: keep evaluation as observational by default and side-effecting
  only through existing governed commands.
