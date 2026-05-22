## Context

The research document `docs/macaca-agent-self-evolving-skills-research.md`
recommends a shell-only operations UI after the governance, curation,
experience-evolution, alias, and proposal-snapshot service slices. The current
Skill SDK already exposes the required read commands.

## Goals / Non-Goals

- Goal: show sanitized Skill governance and evolution state to operators.
- Goal: keep all lifecycle, stale, merge, alias, archive, and proposal
  semantics inside the Skill service.
- Goal: keep the UI generic across applications and agents.
- Non-goal: promote, reject, patch, merge, archive, restore, or delete skills.
- Non-goal: add LLM curation review or auto-apply policy.

## Decisions

- Decision: expose a single Web aggregation route.
  - Reason: the frontend needs one bounded snapshot, while each underlying
    service command remains typed, traceable, and independently replaceable.
- Decision: place the panel in the existing application operations dialog.
  - Reason: Skill governance is an application-scoped OS operations concern, not
    a session transcript or delegated-agent trace tab.
- Decision: render the curation dry-run output as service recommendations.
  - Reason: the frontend must not derive lifecycle state locally.

## Risks / Trade-offs

- Risk: an operations UI can accidentally become the semantic owner.
  - Mitigation: only consume DTO fields returned by `SystemSkillClient`; do not
    compute archive, merge, stale, or alias decisions in TypeScript.
- Risk: sensitive skill content could leak to the browser.
  - Mitigation: return only sanitized governance, alias, proposal, and dry-run
    records; do not expose `SKILL.md` bodies, raw prompts, manifests, package
    bytes, provider payloads, or secrets.

## Migration Plan

This is additive. Existing Skill service commands and application operations
tabs continue to work.

## Open Questions

- Future slices should add approval-gated promote/reject/archive commands only
  after policy and audit contracts are specified.
