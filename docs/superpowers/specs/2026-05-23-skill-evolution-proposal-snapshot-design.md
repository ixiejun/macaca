# Skill Evolution Proposal Snapshot Design

## Context

`docs/macaca-agent-self-evolving-skills-research.md` recommends landing self-evolving skills as small governed service slices. The current implementation can create draft-only experience proposals, but there is no service-owned read path for later curation, approval, or review providers to inspect those proposals.

## Options Considered

1. Add promotion and rejection commands now.
   - Benefit: proposal lifecycle becomes more complete.
   - Risk: introduces mutation and approval semantics before a proposal audit read model exists.
2. Add an LLM curator provider now.
   - Benefit: moves toward automated review.
   - Risk: the provider would need a stable proposal snapshot surface anyway, and direct review could become a prompt-driven black box.
3. Add a read-only proposal snapshot command.
   - Benefit: exposes draft proposals through the Skill service boundary, keeps active skills unchanged, and creates an auditable input for later curation/review.
   - Risk: expands the Skill service contract, so the descriptor change must remain append-only.

## Decision

Use option 3. Add `skill.evolution.snapshot` as a provider-neutral Skill service command. The built-in runtime-host provider returns sorted, sanitized draft proposals from its in-memory governance strategy. The command is read-only and returns `mutated = false` so shells, task services, and future review providers can observe proposal state without owning evolution semantics.

## Architecture

- Command pattern: `SkillExperienceProposalSnapshotCommand` and `SkillExperienceProposalSnapshotResult` live in `macaca-skill`.
- Facade pattern: SDK callers use `SystemSkillClient::skill_experience_snapshot`.
- Strategy pattern: runtime-host keeps the current in-memory strategy replaceable by a later Store/EventLog-backed provider.
- Observer/Memento vocabulary: logs include trace id and proposal count; result timestamps make the read model replayable for audit.
- Specification pattern: snapshot filtering is explicit and provider-neutral; this slice starts with `include_discarded` so later lifecycle states can extend without contract churn.

## Boundaries

- Kernel receives only a typed service call and trace context.
- Runtime-host owns provider adapter logic and local state strategy.
- SDK exposes Null Object unavailable behavior and service-backed routing.
- Web/CLI and applications remain consumers only; they do not scan directories or interpret proposal files.
- No application, workflow, provider, model, driver, or business-domain names are hardcoded.

## Acceptance

- OpenSpec validates the new change.
- Provider tests prove proposals can be listed after creation and missing evidence still rejects without storing.
- Snapshot results are sorted, sanitized, and non-mutating.
- SDK compiles with unavailable and service-backed behavior.
