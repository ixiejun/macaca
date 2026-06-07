## Context

The complete-self-evolution design requires OS-code changes to be represented as
governed proposals before any source mutation. This change adds the first
adapter for that target type. It produces a bounded proposal bundle and explicit
release-gate decision, but it does not edit code.

## Goals

- Keep OS-code evolution service-owned and provider-neutral.
- Preserve OpenSpec/Superpowers/GitNexus as mandatory governance evidence.
- Make source mutation impossible in this adapter.
- Keep output sanitized and bounded.
- Reuse existing release safety vocabulary for blast-radius and rollback refs.

## Non-Goals

- No file writes, patches, commits, shell commands, or test execution.
- No Web/CLI/frontend ownership.
- No application-specific migration or workflow logic.
- No automatic approval bypass.

## Decisions

- **Strategy:** `OsCodeEvolutionProposalAdapter` is replaceable; the default
  Strategy builds proposal metadata and gate findings from DTOs.
- **Command:** `OsCodeEvolutionProposalCommand` carries scoped governance refs
  and proposed artifact names.
- **Specification:** gate checks require OpenSpec proposal/design/tasks refs,
  Superpowers design/plan refs, GitNexus impact refs, expected test refs,
  release-gate refs, and rollback refs.
- **State:** result decisions are `ReadyForReview`, `NeedsEvidence`,
  `Quarantined`, or `Denied`.
- **Memento:** generated proposal refs and rollback refs are body-free mementos
  for the governance ledger.

## Safety Rules

- `allow_source_mutation` must be false. If true, the adapter denies the
  command with `source_mutation_not_allowed`.
- High blast-radius proposals are quarantined, not accepted.
- Missing governance evidence returns `NeedsEvidence`.
- The adapter emits bounded refs only; raw diffs, prompts, provider payloads,
  package bytes, credentials, and unbounded text are excluded.

## Verification

- Tests cover a ready non-mutating proposal, missing gate evidence, high
  blast-radius quarantine, and source-mutation denial.
- `cargo test -p macaca-autonomy-evolution`,
  `openspec validate add-os-code-evolution-proposal-adapter --strict`,
  `git diff --check`, file-size checks, and GitNexus detect-changes run before
  commit.
