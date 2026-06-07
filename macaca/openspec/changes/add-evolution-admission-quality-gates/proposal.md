# Change: Add Evolution Admission Quality Gates

## Why

Macaca can now represent generic evolution runs, but it still needs executable
admission gates before autonomous candidates can become quarantined or active.
Without service-owned quality gates, low-signal Skill candidates such as
experiment-style names, weak triggers, missing validation evidence, duplicate
proposals, or stale metadata can enter the self-evolution loop and pollute later
benchmark, canary, and promotion decisions.

## What Changes

- Add provider-neutral admission DTOs for evolution candidates, starting with
  Skill package candidates.
- Add executable Specification-style quality gates for semantic naming,
  trigger/frontmatter quality, focused `SKILL.md` summaries, resource structure,
  quick validation refs, forward-test refs, duplicate suppression, and stale
  metadata regeneration.
- Add a service-owned admission command/result path to
  `service.autonomy_evolution` with structured `Accepted`, `Denied`,
  `NeedsEvidence`, and `Quarantined` decisions.
- Keep admission generic and metadata-only; the service SHALL NOT read package
  bytes, mutate Skill files, or hardcode application-specific workflows.

## Impact

- Affected specs: `autonomy-evolution-control-plane`
- Affected code:
  - `macaca/crates/services/macaca-autonomy-evolution`
  - `macaca/crates/facade/macaca-sdk`
  - `macaca/crates/runtime/macaca-runtime-host`
- Follow-up changes:
  - `add-normalized-evolution-benchmarking`
  - `add-evolution-release-safety-chain`
