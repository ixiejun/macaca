## Context

The control plane added in `add-autonomy-evolution-control-plane` provides a
generic evolution run state machine. This change adds the next guardrail:
admission quality gates that decide whether a candidate has enough semantic
quality, validation evidence, and duplicate/staleness hygiene to continue.

## Goals

- Model admission as executable Specifications owned by the autonomy evolution
  service.
- Keep all inputs metadata-only and provider-neutral.
- Return structured decisions that are traceable, auditable, and safe to render
  in shells.
- Start with Skill package candidates while keeping the contract extensible for
  later target types.

## Non-Goals

- Do not materialize, edit, delete, or promote Skill packages.
- Do not benchmark candidates.
- Do not run package validation commands directly.
- Do not classify application-specific business workflows.
- Do not store raw prompts, raw manifests, package bytes, or unbounded Skill
  bodies in admission results.

## Design Patterns

- **Specification**: Each quality gate is a deterministic predicate that emits
  bounded findings.
- **Command**: Admission is invoked through typed command/result DTOs.
- **Facade**: SDK callers use a focused client method and receive structured
  unavailable behavior when the service is absent.
- **Adapter**: Runtime-host only decodes service envelopes into typed commands.
- **Strategy-ready**: The default evaluator is generic; future providers can
  replace scoring and gate policy without changing callers.
- **Observer/Memento**: Evidence refs and sanitized findings are replayable
  without carrying raw candidate content.

## Admission Flow

```text
Candidate metadata
  -> EvolutionAdmissionCommand
  -> DefaultEvolutionAdmissionSpecification
  -> gate findings
  -> AdmissionDecision
  -> EvolutionAdmissionResult
```

The evaluator applies the following gates:

- Semantic package naming rejects generic experiment names such as
  `skill-exp-*`, empty names, and unstable generated identifiers.
- Trigger/frontmatter quality requires bounded, semantic triggers or
  frontmatter trigger descriptions.
- `SKILL.md` summary quality requires a focused bounded summary reference and
  rejects oversize body summaries.
- Resource structure requires declared file categories instead of opaque
  package blobs.
- Quick validation and forward-test gates require evidence refs, not command
  execution inside the service.
- Duplicate suppression marks known duplicate candidates as denied.
- Stale metadata returns `NeedsEvidence` so a later metadata regeneration step
  can refresh the candidate before admission.

## Boundary Decisions

- Admission lives in `macaca-autonomy-evolution` because it decides whether a
  candidate may advance in the self-evolution loop.
- Skill file mutation remains in `macaca-skill`.
- SDK remains a facade over service calls and does not construct providers.
- Runtime-host remains an adapter and does not own gate semantics.
- Web/CLI/frontend may render the result but must not duplicate gate logic.

## Observability

Every admission command carries trace context, actor id, target type, scope,
candidate id, bounded evidence refs, and sanitized metadata. Logs record
admission start, per-decision outcome, and missing evidence without exposing raw
package contents, raw prompts, manifests, credentials, or unbounded output.
