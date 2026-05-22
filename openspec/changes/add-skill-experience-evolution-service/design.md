## Context

The governance and alias slices created the service-owned metadata and reference-stability foundation for self-evolving skills. The next research-recommended step is experience draft generation from verified task evidence. This change intentionally stops at proposal creation so active skill mutation remains blocked until future policy, approval, memento, and durable Store support exist.

## Goals

- Convert bounded, sanitized verified task summaries into skill experience proposal records.
- Keep proposal creation behind the traced Skill service command boundary.
- Reject proposals that lack reusable procedure content or evidence references.
- Preserve non-mutating behavior for active skill files, aliases, and lifecycle records.
- Expose SDK facade and Null Object behavior for future Task/Autonomy callers.

## Non-Goals

- No LLM provider or similarity provider integration.
- No automatic creation, patching, promotion, archive, restore, or deletion of skill files.
- No scheduler, task, or context composer automatic hooks.
- No application-specific skill classification or business-domain routing.

## Decisions

- Decision: Extend `service.skill` for the first draft-only evolution slice.
  - Rationale: the existing Skill service owns governance telemetry, curation dry-run, and alias resolution; a separate service can be introduced later once apply/policy workflows require provider independence.
- Decision: Use a deterministic proposal builder.
  - Rationale: the first slice must be auditable and testable without optional LLM modules.
- Decision: Require trace and evidence ids.
  - Rationale: a self-evolving capability must be replayable and tied to verified task completion, not model preference.
- Decision: Store proposals separately from active governance records.
  - Rationale: drafts must not pollute active catalog state or `SKILL.md` instruction content.

## Risks / Trade-offs

- Risk: `service.skill` grows too broad.
  - Mitigation: keep proposal state in `skill_service_provider_state.rs` and typed DTOs in governance contracts; no shell or kernel semantics are added.
- Risk: proposal summaries could leak raw task content.
  - Mitigation: DTO names require bounded summaries, reusable procedure descriptions, and evidence ids; tests assert raw prompt-like fields are not needed.
- Risk: callers may treat a proposal as active skill installation.
  - Mitigation: result fields explicitly say `mutated: false` and proposal status starts as `Draft`.

## Migration Plan

1. Add OpenSpec delta and validate it.
2. Add DTOs, command constant, and descriptor capability.
3. Add provider tests first for accepted and rejected proposals.
4. Implement provider state and command branch with structured logs.
5. Add SDK facade method and run focused checks.

## Open Questions

- A later change must decide whether Task service invokes this command automatically after verified terminal success.
- A later change must decide how proposals are persisted in Store/EventLog and how approval promotes them into active skills.
