# Skill Proposal Processing Lane Design

## Context

`docs/macaca-agent-self-evolution-live-monitoring-report.md` proves that real
`/api/chat/v2` task execution reaches the `service.agent_execution` completion
observer and creates governed Skill experience proposals.  The same report also
proves that proposal capture is not closed-loop self-optimization: Draft
proposals accumulate without quality pressure, duplicate suppression, lifecycle
processing, materialization, activation, reuse telemetry, or optimization
metrics.

## Goals

- Add a service-owned processing lane for Skill evolution proposals after
  proposal capture and before any package materialization.
- Score proposal quality with provider-neutral evidence and bounded metadata.
- Detect duplicate low-information proposals and suppress repeated Draft noise.
- Expose backlog health and processing snapshots through the Skill service.
- Keep Web, CLI, frontend, and applications as thin adapters.
- Preserve trace, policy, audit, and sanitized evidence refs for every state
  transition.

## Non-Goals

- Do not write `SKILL.md`, `_meta.json`, `_usage.json`, or package bytes in this
  first slice.
- Do not call an LLM or semantic provider for proposal review.
- Do not hardcode application, workflow, provider, model, driver, gateway, chain,
  payment, or business-domain names.
- Do not let Web, CLI, or frontend classify proposal quality or lifecycle state.
- Do not claim Skill materialization, activation, reuse, or optimization from
  this slice alone.

## Ownership Model

| Layer | Ownership |
| --- | --- |
| Skill service | Proposal processing commands, state machine, duplicate grouping, quality scores, suppression, audit refs, and snapshots. |
| Runtime host | Built-in local Strategy for deterministic processing and provider lifecycle logs. |
| SDK/SystemFacade | Provider-neutral command facade and explicit unavailable behavior. |
| Web/CLI/frontend | Transport adapters and bounded display only. |
| Store/EventLog | Future durable persistence for processing events and mementos. |
| Materializer | Future policy-gated package mutation after processing marks a proposal eligible. |

## Design Patterns

- **Command**: processing runs and snapshots are typed Skill service commands.
- **State**: each proposal receives explicit processing state such as `Queued`,
  `Reviewing`, `SuppressedDuplicate`, `ReadyForMaterialization`, or `Rejected`.
- **Strategy**: the built-in deterministic scorer can later be replaced by a
  semantic review provider without changing shell callers.
- **Specification**: quality thresholds, duplicate signatures, and eligibility
  rules are executable checks rather than shell-side heuristics.
- **Observer**: proposal capture remains an observer of Agent Execution; the new
  lane observes proposals without owning task execution.
- **Memento**: processing run refs and before/after counts make review replayable
  and rollback-ready for future durable providers.
- **Facade**: SDK clients hide service runtime details from Web and CLI.

## Processing Model

A processing record is derived from an existing
`SkillExperienceProposalRecord`.  It stores only proposal id, task id,
application scope, trace id, lifecycle, processing state, quality score,
duplicate signature, duplicate group size, decision reason, evidence refs,
policy decision refs, and timestamps.  It never stores raw prompts, provider
payloads, full task output, full Skill bodies, package bytes, manifests,
credentials, or secrets.

The first Strategy is deterministic and conservative:

- Quality score starts from bounded evidence completeness.
- Empty or repeated generic summaries lose score.
- Missing artifact refs and zero artifact counts are explicit quality signals,
  not proof of absence when other evidence strata disagree.
- Duplicate grouping uses sanitized summary, destination, recommended action,
  classification, and optional target skill id.
- The first sufficiently evidenced proposal in a group can become
  `ReadyForMaterialization`.
- Later low-information duplicates become `SuppressedDuplicate`.
- Proposals with missing trace/evidence or policy denial become `Rejected`.

## Service Boundary

The Skill service exposes:

- `skill.evolution.processing.run`: process current proposals for one scope.
- `skill.evolution.processing.snapshot`: read processing state and backlog
  counters without mutation.

Apply-mode processing requires trace, evidence refs, policy decision refs, and
policy hints.  Dry-run mode can compute recommendations without mutation.  Both
modes emit structured logs with trace id, proposal counts, duplicate groups,
state counts, evidence counts, policy decision counts, and mutation status.

## Shell Boundary

Web and CLI may show processing snapshots and invoke processing commands through
SDK methods.  They must not classify proposal quality, compute duplicate
signatures, decide eligibility, suppress duplicates, or infer materialization
readiness locally.

## Verification

The first implementation must prove:

- OpenSpec validates in strict mode.
- `macaca-skill` DTO tests cover validation, score bounds, and duplicate
  signature sanitization.
- `macaca-runtime-host` tests cover dry-run immutability, apply mutation,
  duplicate suppression, ready-for-materialization marking, rejection on missing
  policy refs, and processing snapshots.
- `macaca-sdk` tests cover unavailable behavior and service-backed command
  forwarding.
- Web route tests, if touched, prove Web only forwards DTOs and returns service
  results.
- `git diff --check` and GitNexus change detection run before completion.

## Risks And Mitigations

- Risk: processing lane is mistaken for materialization.
  Mitigation: DTO names and docs state that this slice does not write Skill
  packages or prove activation/reuse.
- Risk: deterministic scoring becomes business-specific.
  Mitigation: use generic evidence quality, lifecycle, destination, and duplicate
  signals only.
- Risk: duplicate grouping suppresses useful variation.
  Mitigation: keep the first candidate eligible and retain all suppressed records
  with audit refs.
- Risk: in-memory state disappears after restart.
  Mitigation: expose provider strategy limitations and keep event/memento refs
  shaped for future Store/EventLog persistence.
